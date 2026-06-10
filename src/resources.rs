//! Unified resource discovery + invocation SDK.
//!
//! Collapses six resource registries (tools, skills, knowledge,
//! workflow templates, agent templates, models) into a single
//! discovery + invocation surface so an agent can ask "what
//! resources match this filter?" once and invoke any of them
//! without per-registry plumbing.
//!
//! ```no_run
//! # use tenzro_sdk::{TenzroClient, config::SdkConfig};
//! # use tenzro_sdk::resources::{ResourceFilter, UseResourceParams};
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = TenzroClient::connect(SdkConfig::testnet()).await?;
//! let r = client.resources();
//!
//! // Discover finance-related resources under 10 mTNZO each.
//! let hits = r.list(ResourceFilter {
//!     capability_tags: vec!["prices".into()],
//!     max_tnzo_price: Some("10000000000000000".into()),
//!     ..Default::default()
//! }).await?;
//!
//! // Use the first hit. Class is auto-detected.
//! let result = r.use_resource(UseResourceParams {
//!     resource_id: hits[0].resource_id.clone(),
//!     class: None,
//!     params: serde_json::json!({ "symbol": "SOL/USD" }),
//!     payer_wallet: Some("0xabc...".into()),
//! }).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct ResourcesClient {
    rpc: Arc<RpcClient>,
}

impl ResourcesClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// List resources across one or more registries.
    pub async fn list(&self, filter: ResourceFilter) -> SdkResult<Vec<ResourceDescriptor>> {
        let v = serde_json::to_value(&filter).map_err(|_| {
            crate::error::SdkError::SerializationError
        })?;
        self.rpc.call("tenzro_listResources", v).await
    }

    /// Invoke a resource by id. The class is auto-detected unless
    /// the caller passes `class` explicitly.
    pub async fn use_resource(
        &self,
        params: UseResourceParams,
    ) -> SdkResult<serde_json::Value> {
        let v = serde_json::to_value(&params).map_err(|_| {
            crate::error::SdkError::SerializationError
        })?;
        self.rpc.call("tenzro_useResource", v).await
    }

    /// Spawn a child agent atomically — TDIP identity + MPC wallet +
    /// TNZO funding from `parent_wallet` + runtime spending policy.
    pub async fn spawn_child_agent(
        &self,
        params: SpawnChildAgentParams,
    ) -> SdkResult<SpawnChildAgentResponse> {
        let v = serde_json::to_value(&params).map_err(|_| {
            crate::error::SdkError::SerializationError
        })?;
        self.rpc.call("tenzro_spawnChildAgent", v).await
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceFilter {
    /// Subset of resource classes to query. Empty = all classes.
    /// Class names: "tool", "skill", "knowledge", "workflow_template",
    /// "agent_template", "model".
    #[serde(default)]
    pub classes: Vec<String>,
    /// Free-text query — matches name, description, capabilities.
    #[serde(default)]
    pub query: Option<String>,
    /// Capability tags — AND-match.
    #[serde(default)]
    pub capability_tags: Vec<String>,
    /// Optional category filter.
    #[serde(default)]
    pub category: Option<String>,
    /// Cost ceiling in atto-TNZO (decimal string for u128 safety).
    #[serde(default)]
    pub max_tnzo_price: Option<String>,
    /// Filter by creator DID.
    #[serde(default)]
    pub creator_did: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDescriptor {
    pub class: String,
    pub resource_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: String,
    pub capabilities: Vec<String>,
    pub creator_did: Option<String>,
    pub creator_wallet: Option<String>,
    /// atto-TNZO as decimal string. `0` = free.
    pub price_per_call: String,
    pub is_available: bool,
    pub last_seen_at: u64,
    pub subtype: Option<String>,
    pub reputation: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseResourceParams {
    pub resource_id: String,
    /// Force a specific class to skip auto-detect. None = auto-detect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    pub params: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer_wallet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnChildAgentParams {
    pub parent_did: String,
    pub display_name: String,
    /// Initial TNZO budget in atto-TNZO (decimal string for u128 safety).
    /// `0` = no funding step.
    pub tnzo_budget: String,
    /// Required when `tnzo_budget > 0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_wallet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_per_transaction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_daily_spend: Option<String>,
    /// "ed25519" or "secp256k1". Default "ed25519".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnChildAgentResponse {
    pub child_did: String,
    pub parent_did: String,
    pub child_wallet: Option<String>,
    pub registration: serde_json::Value,
    pub funding: serde_json::Value,
    pub spending_policy: serde_json::Value,
}
