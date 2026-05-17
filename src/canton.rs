//! Canton/DAML SDK for Tenzro Network
//!
//! This module provides Canton synchronizer domain and DAML contract interaction
//! functionality, including domain listing, contract queries, and command submission.
//!
//! Uses the Canton 3.x JSON Ledger API v2 endpoints:
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
/// // List available Canton domains
/// let domains = canton.list_domains().await?;
/// for domain in &domains {
///     println!("Domain: {} ({})", domain.domain_id, domain.status);
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

    /// Lists all available Canton synchronizer domains
    ///
    /// Returns metadata about each domain including its ID, alias, status,
    /// and connected participants.
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
    /// let domains = canton.list_domains().await?;
    /// println!("Found {} Canton domains", domains.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_domains(&self) -> SdkResult<Vec<CantonDomain>> {
        self.rpc
            .call("tenzro_listCantonDomains", serde_json::json!([]))
            .await
    }

    /// Lists DAML contracts, optionally filtered by template ID and party
    ///
    /// Queries the Canton 3.x JSON Ledger API v2 active-contracts endpoint
    /// with `identifierFilter` for template-based filtering.
    ///
    /// # Arguments
    ///
    /// * `template_id` - Optional DAML template ID to filter by (passed as `identifierFilter`)
    /// * `party` - Optional party ID to filter active contracts for
    /// * `limit` - Optional limit on number of contracts returned
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
    /// let contracts = canton.list_contracts(
    ///     Some("Tenzro.Escrow:EscrowContract"),
    ///     Some("party::tenzro-validator"),
    ///     Some(50),
    /// ).await?;
    /// println!("Found {} contracts", contracts.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_contracts(
        &self,
        template_id: Option<&str>,
        party: Option<&str>,
        limit: Option<u32>,
    ) -> SdkResult<Vec<DamlContract>> {
        let mut params = serde_json::Map::new();
        if let Some(tid) = template_id {
            params.insert("template_id".to_string(), serde_json::json!(tid));
        }
        if let Some(p) = party {
            params.insert("party".to_string(), serde_json::json!(p));
        }
        if let Some(lim) = limit {
            params.insert("limit".to_string(), serde_json::json!(lim));
        }

        self.rpc
            .call(
                "tenzro_listDamlContracts",
                serde_json::json!([serde_json::Value::Object(params)]),
            )
            .await
    }

    /// Submits a DAML command (create or exercise) to a Canton domain
    ///
    /// Uses the Canton 3.x JSON Ledger API v2 `submit-and-wait-for-transaction`
    /// endpoint (not `submit-and-wait-for-transaction-tree`).
    ///
    /// # Arguments
    ///
    /// * `params` - The DAML command parameters
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # use tenzro_sdk::canton::DamlCommandParams;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let canton = client.canton();
    /// let result = canton.submit_command(DamlCommandParams {
    ///     domain_id: "domain-1".to_string(),
    ///     command_type: "create".to_string(),
    ///     template_id: "Tenzro.Escrow:EscrowContract".to_string(),
    ///     party: "party::tenzro-validator".to_string(),
    ///     payload: serde_json::json!({"payer": "alice", "payee": "bob", "amount": 1000}),
    ///     contract_id: None,
    ///     choice: None,
    /// }).await?;
    /// println!("Command result: {}", result.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn submit_command(
        &self,
        params: DamlCommandParams,
    ) -> SdkResult<DamlCommandResult> {
        self.rpc
            .call(
                "tenzro_submitDamlCommand",
                serde_json::json!([{
                    "domain_id": params.domain_id,
                    "command_type": params.command_type,
                    "template_id": params.template_id,
                    "party": params.party,
                    "payload": params.payload,
                    "contract_id": params.contract_id,
                    "choice": params.choice,
                }]),
            )
            .await
    }
}

/// A Canton synchronizer domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CantonDomain {
    /// Domain identifier
    #[serde(default)]
    pub domain_id: String,
    /// Human-readable alias
    #[serde(default)]
    pub alias: String,
    /// Domain status (active, inactive, etc.)
    #[serde(default)]
    pub status: String,
    /// Number of connected participants
    #[serde(default)]
    pub participant_count: u32,
    /// Sequencer endpoint URL
    #[serde(default)]
    pub sequencer_url: String,
}

/// A DAML contract on a Canton domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamlContract {
    /// Contract ID
    #[serde(default)]
    pub contract_id: String,
    /// DAML template ID (e.g., "Tenzro.Escrow:EscrowContract")
    #[serde(default)]
    pub template_id: String,
    /// Contract payload (create arguments)
    #[serde(default)]
    pub payload: serde_json::Value,
    /// Signatories
    #[serde(default)]
    pub signatories: Vec<String>,
    /// Observers
    #[serde(default)]
    pub observers: Vec<String>,
    /// Whether the contract is active
    #[serde(default)]
    pub active: bool,
}

/// Parameters for submitting a DAML command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamlCommandParams {
    /// Canton domain to submit to
    pub domain_id: String,
    /// Command type: "create" or "exercise"
    pub command_type: String,
    /// DAML template ID
    pub template_id: String,
    /// Submitting party
    pub party: String,
    /// Command payload (create arguments or exercise arguments)
    pub payload: serde_json::Value,
    /// Contract ID (required for exercise commands)
    pub contract_id: Option<String>,
    /// Choice name (required for exercise commands)
    pub choice: Option<String>,
}

/// Result from submitting a DAML command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamlCommandResult {
    /// Command ID
    #[serde(default)]
    pub command_id: String,
    /// Result status
    #[serde(default)]
    pub status: String,
    /// Transaction ID (if command resulted in a transaction)
    #[serde(default)]
    pub transaction_id: Option<String>,
    /// Created contract ID (for create commands)
    #[serde(default)]
    pub contract_id: Option<String>,
    /// Exercise result (for exercise commands)
    #[serde(default)]
    pub exercise_result: Option<serde_json::Value>,
}
