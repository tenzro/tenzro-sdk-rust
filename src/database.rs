//! Managed-database client for Tenzro Network.
//!
//! A node can serve one or more database engines the operator wired up —
//! external processes the operator runs (Postgres, Qdrant, Valkey) and embedded
//! engines that run in-process (Lance, Tantivy). The catalog layer records
//! *what* databases exist and *where* their partitions land; the engine drivers
//! answer queries against the partitions a node holds.
//!
//! Every database is owned. Reads and writes are gated by the database's
//! [access policy]: the owner (or a caller holding an AAP capability that names
//! the database) may query it, and administrative operations — create, rescale,
//! drop, issue-connection — require the write action. The
//! [`issue_connection`](DatabaseClient::issue_connection) call mints a bearer
//! token a developer plugs into a managed-DB client.
//!
//! The `body` passed to [`query`](DatabaseClient::query) is engine-dialect: a
//! SQL `{sql, params}` for Postgres, a `{op, ...}` document for Qdrant / Lance /
//! Tantivy, a `{command: [...]}` for Valkey. The node dispatches it verbatim to
//! the backend for the database's engine.
//!
//! [access policy]: https://docs.tenzro.xyz

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

/// Managed-database client.
pub struct DatabaseClient {
    rpc: Arc<RpcClient>,
}

impl DatabaseClient {
    /// Creates a new database client.
    pub fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Lists the engine catalog a node can serve — each engine with its data
    /// models, license, sharding model, and native-cluster topology.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let catalog = client.database().list_engines().await?;
    /// println!("{} engines", catalog.count);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_engines(&self) -> SdkResult<EngineCatalog> {
        let result = self.rpc.call("tenzro_listDatabaseEngines", json!([])).await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse engine catalog: {}", e)))
    }

    /// Registers a database this node serves, computing and persisting its
    /// partition placement over the live cluster. `owner_did` becomes the
    /// database's admin authority (an owner-only access policy). `placement` is
    /// `local | lan_cluster | network`.
    ///
    /// Pass `engine_config` for per-engine tuning (opaque to the catalog; the
    /// driver interprets it) and `confidential` for network-tier
    /// encryption-at-rest. `replication` is the `(min, max)` holders-per-partition
    /// policy — writes below `min` fail, repair never grows past `max`; when
    /// `None` the node default of `(2, 4)` applies.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # use serde_json::json;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let created = client.database().create(
    ///     "agent-memory",
    ///     "qdrant",
    ///     "did:tenzro:human:alice",
    ///     "lan_cluster",
    ///     3,
    ///     Some((2, 4)),
    ///     Some(json!({ "vector_size": 1536 })),
    /// ).await?;
    /// println!("{} partitions", created.partitions.len());
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        database_id: &str,
        engine_id: &str,
        owner_did: &str,
        placement: &str,
        partitions: usize,
        replication: Option<(u8, u8)>,
        engine_config: Option<Value>,
    ) -> SdkResult<CreatedDatabase> {
        let mut params = json!({
            "database_id": database_id,
            "engine_id": engine_id,
            "owner_did": owner_did,
            "placement": placement,
            "partitions": partitions,
        });
        if let Some((min, max)) = replication {
            params["replication"] = json!({
                "min_replication": min,
                "max_replication": max,
            });
        }
        if let Some(cfg) = engine_config {
            params["engine_config"] = cfg;
        }
        let result = self.rpc.call("tenzro_createDatabase", json!([params])).await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse created database: {}", e)))
    }

    /// Looks up a database descriptor by id.
    pub async fn get(&self, database_id: &str) -> SdkResult<Value> {
        self.rpc
            .call("tenzro_getDatabase", json!([{ "database_id": database_id }]))
            .await
    }

    /// Lists every database this node serves.
    pub async fn list(&self) -> SdkResult<Value> {
        self.rpc.call("tenzro_listDatabases", json!([])).await
    }

    /// Lists every partition placement of a database.
    pub async fn list_partitions(&self, database_id: &str) -> SdkResult<Value> {
        self.rpc
            .call("tenzro_listDatabasePartitions", json!([{ "database_id": database_id }]))
            .await
    }

    /// Returns the placement of one partition.
    pub async fn get_partition(&self, database_id: &str, partition_index: usize) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_getDatabasePartition",
                json!([{ "database_id": database_id, "partition_index": partition_index }]),
            )
            .await
    }

    /// Mints a managed-database connection credential bound to `bearer_did`,
    /// scoped to this one database. The owner (`caller_did`) — or a caller
    /// holding the write-action capability — issues it. When `write` is true the
    /// token also carries the admin action. The returned `capability` + a
    /// `bearer_did` are what a developer presents on every
    /// [`query`](DatabaseClient::query).
    pub async fn issue_connection(
        &self,
        database_id: &str,
        caller_did: &str,
        bearer_did: Option<&str>,
        write: bool,
        ttl_secs: Option<u64>,
        capability: Option<&str>,
    ) -> SdkResult<DatabaseConnection> {
        let mut params = json!({
            "database_id": database_id,
            "caller_did": caller_did,
            "write": write,
        });
        if let Some(bearer) = bearer_did {
            params["bearer_did"] = json!(bearer);
        }
        if let Some(ttl) = ttl_secs {
            params["ttl_secs"] = json!(ttl);
        }
        if let Some(cap) = capability {
            params["capability"] = json!(cap);
        }
        let result = self
            .rpc
            .call("tenzro_issueDatabaseConnection", json!([params]))
            .await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse connection: {}", e)))
    }

    /// Runs an engine-dialect query against a database partition. `caller_did`
    /// is authorized against the access policy (writes require the admin action,
    /// reads the read action) before any engine is touched. `body` is the
    /// engine dialect; `write` gates the query against the admin action;
    /// `capability` is an AAP token when the caller is not the owner.
    /// `consistency` is the write acknowledgement level — `"quorum"` (default)
    /// or `"all"`; ignored on the read path.
    ///
    /// When this node holds the target partition the result carries
    /// `served_here=true` and the engine `result`; otherwise it carries the
    /// holder endpoints so the caller can reach a node that does.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # use serde_json::json;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let res = client.database().query(
    ///     "ledger",
    ///     "did:tenzro:human:alice",
    ///     json!({ "sql": "select id, name from tz_ledger_0.rows limit $1", "params": [10] }),
    ///     0,
    ///     false,
    ///     None,
    ///     None,
    /// ).await?;
    /// println!("{res}");
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub async fn query(
        &self,
        database_id: &str,
        caller_did: &str,
        body: Value,
        partition_index: usize,
        write: bool,
        capability: Option<&str>,
        consistency: Option<&str>,
    ) -> SdkResult<Value> {
        let mut params = json!({
            "database_id": database_id,
            "caller_did": caller_did,
            "body": body,
            "partition_index": partition_index,
            "write": write,
        });
        if let Some(cap) = capability {
            params["capability"] = json!(cap);
        }
        if let Some(c) = consistency {
            params["consistency"] = json!(c);
        }
        self.rpc.call("tenzro_databaseQuery", json!([params])).await
    }

    /// Checks — without side effects — whether `caller_did` may read the
    /// database. Returns `{authorized, reason}`.
    pub async fn authorize_read(
        &self,
        database_id: &str,
        caller_did: &str,
        capability: Option<&str>,
    ) -> SdkResult<AccessDecision> {
        let mut params = json!({
            "database_id": database_id,
            "caller_did": caller_did,
        });
        if let Some(cap) = capability {
            params["capability"] = json!(cap);
        }
        let result = self
            .rpc
            .call("tenzro_authorizeDatabaseRead", json!([params]))
            .await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse access decision: {}", e)))
    }

    /// Grows or shrinks a database along the local → LAN-cluster → network
    /// continuum in place. Administrative — gated on the write action.
    /// `partitions`/`replication` default to the database's current values when
    /// omitted; `replication` is the `(min, max)` holders-per-partition policy.
    #[allow(clippy::too_many_arguments)]
    pub async fn rescale(
        &self,
        database_id: &str,
        caller_did: &str,
        placement: &str,
        partitions: Option<usize>,
        replication: Option<(u8, u8)>,
        capability: Option<&str>,
    ) -> SdkResult<Value> {
        let mut params = json!({
            "database_id": database_id,
            "caller_did": caller_did,
            "placement": placement,
        });
        if let Some(p) = partitions {
            params["partitions"] = json!(p);
        }
        if let Some((min, max)) = replication {
            params["replication"] = json!({
                "min_replication": min,
                "max_replication": max,
            });
        }
        if let Some(cap) = capability {
            params["capability"] = json!(cap);
        }
        self.rpc.call("tenzro_rescaleDatabase", json!([params])).await
    }

    /// Removes a database and all its partition placements, tearing down the
    /// engine backing for every partition this node holds.
    pub async fn drop(&self, database_id: &str) -> SdkResult<Value> {
        self.rpc
            .call("tenzro_dropDatabase", json!([{ "database_id": database_id }]))
            .await
    }
}

/// The engine catalog a node can serve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineCatalog {
    /// One entry per engine the catalog knows about.
    pub engines: Vec<Value>,
    /// Number of engines.
    pub count: usize,
}

/// Result of [`DatabaseClient::create`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedDatabase {
    /// The normalized database descriptor.
    pub database: Value,
    /// The computed partition placements.
    pub partitions: Vec<Value>,
}

/// A managed-database connection credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConnection {
    /// The database this credential is scoped to.
    pub database_id: String,
    /// The engine dialect the database speaks.
    pub engine_id: String,
    /// The DID the credential was minted for.
    pub bearer_did: String,
    /// `read_only` or `read_write`.
    pub mode: String,
    /// The bearer token to present on every query.
    pub capability: String,
    /// Credential lifetime in seconds.
    pub ttl_secs: u64,
    /// The RPC method the developer dials with this credential.
    pub query_method: String,
}

/// The outcome of an access check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessDecision {
    /// Whether the caller may read the database.
    pub authorized: bool,
    /// Human-readable reason for the decision.
    pub reason: String,
}
