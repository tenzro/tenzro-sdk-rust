//! Canton/DAML SDK for Tenzro Network
//!
//! This module provides Canton synchronizer domain and DAML contract interaction
//! functionality, including domain listing, contract queries, and command submission.
//!
//! Uses the Canton 3.5+ JSON Ledger API v2 endpoints:
//! - Commands: `POST /v2/commands/submit-and-wait-for-transaction`
//! - Active contracts: `POST /v2/state/active-contracts` (with `identifierFilter`)
//! - Events: `POST /v2/events/events-by-contract-id`

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Canton client for Canton/DAML operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let canton = client.canton();
///
/// // List available Canton synchronizer domains
/// let domains = canton.list_domains().await?;
/// for domain in &domains.domains {
///     println!("Domain: {} ({})", domain.id, domain.name);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct CantonClient {
    rpc: Arc<RpcClient>,
}

impl CantonClient {
    /// Creates a new Canton client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Lists configured Canton synchronizer domains on this node
    ///
    /// Returns the `{enabled, domains}` envelope from the node. When Canton is
    /// not enabled the call still succeeds with `enabled: false` and an empty
    /// `domains` array — callers should check `enabled` before proceeding.
    pub async fn list_domains(&self) -> SdkResult<CantonDomainList> {
        self.rpc
            .call("tenzro_listCantonDomains", serde_json::json!({}))
            .await
    }

    /// Queries active DAML contracts on the shared Canton domain.
    ///
    /// The Canton v2 active-contracts endpoint requires at least one template
    /// id. Pass either a single `template_id` or a list via the
    /// [`DamlContractsQuery`] builder. The optional `query` object is applied
    /// client-side against `createArguments` as a structural filter.
    pub async fn list_contracts(
        &self,
        query: DamlContractsQuery,
    ) -> SdkResult<DamlContractsResponse> {
        let mut params = serde_json::Map::new();
        if !query.template_ids.is_empty() {
            params.insert(
                "template_ids".to_string(),
                serde_json::json!(query.template_ids),
            );
        }
        if let Some(filter) = query.query {
            params.insert("query".to_string(), filter);
        }
        self.rpc
            .call(
                "tenzro_listDamlContracts",
                serde_json::Value::Object(params),
            )
            .await
    }

    /// Submits a DAML `create` command on the configured Canton domain.
    ///
    /// The node mediates the call to the Canton participant — callers
    /// never handle the upstream credentials.
    pub async fn create_contract(
        &self,
        template_id: &str,
        create_arguments: serde_json::Value,
    ) -> SdkResult<DamlCommandResult> {
        self.rpc
            .call(
                "tenzro_submitDamlCommand",
                serde_json::json!({
                    "command_type": "create",
                    "template_id": template_id,
                    "create_arguments": create_arguments,
                }),
            )
            .await
    }

    /// Submits a DAML `exercise` command on an existing contract.
    pub async fn exercise_choice(
        &self,
        template_id: &str,
        contract_id: &str,
        choice: &str,
        choice_argument: serde_json::Value,
    ) -> SdkResult<DamlCommandResult> {
        self.rpc
            .call(
                "tenzro_submitDamlCommand",
                serde_json::json!({
                    "command_type": "exercise",
                    "template_id": template_id,
                    "contract_id": contract_id,
                    "choice": choice,
                    "choice_argument": choice_argument,
                }),
            )
            .await
    }

    /// Allocate a new party on the participant via `POST /v2/parties`.
    ///
    /// Returns the participant's response containing the
    /// fully-qualified party id `<party_id_hint>::<participant-hash>`.
    /// The newly-allocated party has no `CanActAs` / `CanReadAs`
    /// grants on any user by default — follow up with
    /// [`grant_user_rights`] so the operator's OAuth user can submit
    /// DAML commands on behalf of the new party.
    pub async fn allocate_party(
        &self,
        party_id_hint: &str,
        display_name: Option<&str>,
    ) -> SdkResult<serde_json::Value> {
        let mut params = serde_json::Map::new();
        params.insert(
            "party_id_hint".into(),
            serde_json::Value::String(party_id_hint.to_string()),
        );
        if let Some(d) = display_name {
            params.insert("display_name".into(), serde_json::Value::String(d.to_string()));
        }
        self.rpc
            .call("tenzro_allocateParty", serde_json::Value::Object(params))
            .await
    }

    /// Grant `CanActAs` / `CanReadAs` rights on a party to a user
    /// (Canton 3.5+ User Management Service via
    /// `POST /v2/users/{userId}/rights`).
    ///
    /// Without these grants, the calling user cannot submit DAML
    /// commands on behalf of a newly-allocated party. Pass
    /// `user_id = None` to grant to the calling principal's own
    /// Canton user.
    pub async fn grant_user_rights(
        &self,
        user_id: Option<&str>,
        party: &str,
        can_act_as: bool,
        can_read_as: bool,
    ) -> SdkResult<serde_json::Value> {
        let mut params = serde_json::Map::new();
        if let Some(u) = user_id {
            params.insert("user_id".into(), serde_json::Value::String(u.to_string()));
        }
        params.insert("party".into(), serde_json::Value::String(party.to_string()));
        params.insert("can_act_as".into(), serde_json::Value::Bool(can_act_as));
        params.insert("can_read_as".into(), serde_json::Value::Bool(can_read_as));
        self.rpc
            .call(
                "tenzro_canton_grantUserRights",
                serde_json::Value::Object(params),
            )
            .await
    }

    /// List the rights granted to a Canton user
    /// (`GET /v2/users/{userId}/rights`). Pass `None` to list rights
    /// for the OAuth principal's own user.
    pub async fn list_user_rights(
        &self,
        user_id: Option<&str>,
    ) -> SdkResult<serde_json::Value> {
        let params = match user_id {
            Some(u) => serde_json::json!({ "user_id": u }),
            None => serde_json::json!({}),
        };
        self.rpc.call("tenzro_canton_listUserRights", params).await
    }

    /// Subject self-read: returns the Canton call aggregates for the
    /// API key presented by this RPC client. Counters are maintained
    /// server-side in RocksDB (`CF_CANTON_ANALYTICS`) — every
    /// canton-scoped call increments `calls_total` (or
    /// `errors_total`) plus the corresponding per-method bucket.
    /// Lets a tenant answer "how many DAML transactions have I
    /// submitted, and which methods am I hitting?" without operator
    /// help.
    pub async fn get_my_analytics(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_canton_getMyAnalytics", serde_json::json!({}))
            .await
    }

    /// Operator admin-read: returns every per-tenant aggregate.
    /// Gated behind the operator admin token (`X-Tenzro-Admin-Token`)
    /// at the node — non-admin callers see `-32001`. Optional
    /// `key_id` filter narrows to a single tenant.
    pub async fn list_api_key_analytics(
        &self,
        key_id: Option<&str>,
    ) -> SdkResult<serde_json::Value> {
        let params = match key_id {
            Some(k) => serde_json::json!({ "key_id": k }),
            None => serde_json::json!({}),
        };
        self.rpc
            .call("tenzro_canton_listApiKeyAnalytics", params)
            .await
    }

    // ── Canton 3.5+ JSON Ledger API extension methods ──

    /// Upload a DAR (DAML Archive) to the participant via
    /// `POST /v2/packages`. `dar_bytes` is the raw DAR file bytes; the
    /// node base64-encodes them on the way out.
    pub async fn upload_dar(&self, dar_bytes: &[u8]) -> SdkResult<serde_json::Value> {
        use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
        let b64 = B64.encode(dar_bytes);
        self.rpc
            .call(
                "tenzro_canton_uploadDar",
                serde_json::json!({ "dar_content_base64": b64 }),
            )
            .await
    }

    /// List every party known to the participant. Note: on the Tenzro
    /// DevNet the `daml_ledger_api` scope may not grant read access to
    /// the party registry; expect `{"partyDetails":[]}` in that case.
    pub async fn list_parties(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_canton_listParties", serde_json::json!({})).await
    }

    /// Combined health probe: `/livez`, `/readyz`, `/v2/version`.
    /// Returns `{alive, ready, ready_detail, version}` where `version`
    /// carries Canton CIP feature flags when reachable.
    pub async fn health(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_canton_health", serde_json::json!({})).await
    }

    /// Returns participant version + CIP feature flags via
    /// `GET /v2/version`.
    pub async fn version(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_canton_version", serde_json::json!({})).await
    }

    /// Fetch a Canton transaction tree by update id (hex string).
    pub async fn get_transaction(&self, update_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_canton_getTransaction",
                serde_json::json!({ "update_id": update_id }),
            )
            .await
    }

    /// List every DAML package installed on the participant.
    pub async fn list_packages(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_canton_listPackages", serde_json::json!({})).await
    }

    /// Returns the Canton Coin (CIP-56) balance for the participant's
    /// party by summing every `Splice.Amulet:Amulet` contract the
    /// party is a stakeholder on.
    pub async fn canton_coin_balance(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_canton_coinBalance", serde_json::json!({})).await
    }

    /// Returns the participant's Canton fee schedule sourced from the
    /// latest `Splice.AmuletRules:AmuletRules` contract.
    pub async fn fee_schedule(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_canton_feeSchedule", serde_json::json!({})).await
    }

    /// Returns the synchronizers the participant's party is currently
    /// connected to.
    pub async fn connected_synchronizers(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_canton_connectedSynchronizers", serde_json::json!({}))
            .await
    }

    /// Returns the Canton user record for the calling principal via
    /// CIP-26 User Management.
    pub async fn get_my_user(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_canton_getMyUser", serde_json::json!({})).await
    }
}

/// Response envelope for `tenzro_listCantonDomains`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CantonDomainList {
    /// Whether Canton/DAML is enabled on this node
    #[serde(default)]
    pub enabled: bool,
    /// Configured synchronizer domains
    #[serde(default)]
    pub domains: Vec<CantonDomain>,
    /// Optional human-readable status message (present when `enabled` is false)
    #[serde(default)]
    pub message: Option<String>,
}

/// A Canton synchronizer domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CantonDomain {
    /// Synchronizer / domain identifier
    #[serde(default)]
    pub id: String,
    /// Human-readable domain name
    #[serde(default)]
    pub name: String,
    /// Native settlement token for this domain
    #[serde(default)]
    pub native_token: String,
    /// Expected finality time in seconds
    #[serde(default)]
    pub finality_time_secs: u64,
}

/// Query parameters for [`CantonClient::list_contracts`]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DamlContractsQuery {
    /// One or more DAML template ids (required — at least one)
    pub template_ids: Vec<String>,
    /// Optional structural filter applied against `createArguments`
    pub query: Option<serde_json::Value>,
}

impl DamlContractsQuery {
    /// Build a query for a single template id
    pub fn for_template(template_id: impl Into<String>) -> Self {
        Self {
            template_ids: vec![template_id.into()],
            query: None,
        }
    }

    /// Build a query for multiple template ids
    pub fn for_templates(template_ids: impl IntoIterator<Item = String>) -> Self {
        Self {
            template_ids: template_ids.into_iter().collect(),
            query: None,
        }
    }

    /// Attach a structural filter
    pub fn with_query(mut self, query: serde_json::Value) -> Self {
        self.query = Some(query);
        self
    }
}

/// Response envelope for `tenzro_listDamlContracts`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamlContractsResponse {
    /// Active contracts matching the query
    #[serde(default)]
    pub contracts: Vec<DamlContract>,
    /// Template ids that were queried (echoed for traceability)
    #[serde(default)]
    pub template_ids: Vec<String>,
    /// Structural filter that was applied (echoed for traceability)
    #[serde(default)]
    pub query: serde_json::Value,
}

/// A DAML contract on a Canton domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamlContract {
    /// Contract ID
    #[serde(default)]
    pub contract_id: String,
    /// DAML template ID (e.g., "Tenzro.Workflow:WorkflowAnchor")
    #[serde(default)]
    pub template_id: String,
    /// Contract payload (create arguments)
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Result from submitting a DAML command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamlCommandResult {
    /// "create" or "exercise"
    #[serde(default)]
    pub command_type: String,
    /// DAML template ID the command was submitted against
    #[serde(default)]
    pub template_id: String,
    /// Created contract ID (for create commands)
    #[serde(default)]
    pub contract_id: Option<String>,
    /// Contract payload returned by the participant (for create commands)
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    /// Choice name (for exercise commands)
    #[serde(default)]
    pub choice: Option<String>,
    /// Exercise result (for exercise commands)
    #[serde(default)]
    pub exercise_result: Option<serde_json::Value>,
    /// Ledger events produced by the command (for exercise commands)
    #[serde(default)]
    pub events: Option<serde_json::Value>,
}
