//! Agent marketplace SDK for Tenzro Network
//!
//! This module provides agent template listing, registration, and retrieval functionality.

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use crate::types::AgentTemplate;
use std::sync::Arc;

/// Marketplace client for agent template marketplace operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let marketplace = client.marketplace();
///
/// // List free agent templates
/// let templates = marketplace.list_agent_templates(Some(true), Some(10), Some(0)).await?;
/// println!("Found {} agent templates", templates.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct MarketplaceClient {
    rpc: Arc<RpcClient>,
}

impl MarketplaceClient {
    /// Creates a new marketplace client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Lists agent templates from the marketplace
    pub async fn list_agent_templates(
        &self,
        free_only: Option<bool>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> SdkResult<Vec<AgentTemplate>> {
        let mut params = serde_json::Map::new();
        if let Some(free) = free_only {
            params.insert("free_only".to_string(), serde_json::json!(free));
        }
        if let Some(lim) = limit {
            params.insert("limit".to_string(), serde_json::json!(lim));
        }
        if let Some(off) = offset {
            params.insert("offset".to_string(), serde_json::json!(off));
        }

        let value: serde_json::Value = self
            .rpc
            .call(
                "tenzro_listAgentTemplates",
                serde_json::Value::Object(params),
            )
            .await?;

        let templates_value = value
            .get("templates")
            .cloned()
            .unwrap_or(value);

        serde_json::from_value(templates_value).map_err(|e| {
            SdkError::RpcError(format!("Failed to parse Vec<AgentTemplate>: {}", e))
        })
    }

    /// Registers a new agent template on the marketplace.
    ///
    /// Paid-agent marketplace semantics:
    /// * `creator_did` — optional DID binding (e.g. `did:tenzro:human:...`) for
    ///   creator attribution. Bound at registration time and immutable.
    /// * `creator_wallet` — hex-encoded (0x-prefixed) payout wallet. **Mandatory**
    ///   for any non-free pricing; all creator payouts are routed here.
    /// * `pricing` — compact string form accepted by the `tenzro_registerAgentTemplate`
    ///   RPC: `"free"`, `"per_execution:<u128>"`, `"per_token:<u128>"`,
    ///   `"subscription:<u128>"`, or `"revenue_share:<bps>"`. A canonical JSON
    ///   `AgentPricingModel` object is also accepted by passing a JSON string
    ///   (e.g. `serde_json::to_string(&model)?`).
    ///
    /// On successful invocation via `run_agent_template`, the `AGENT_MARKETPLACE_COMMISSION_BPS`
    /// (5%) network commission flows to the treasury and the remainder is paid
    /// to `creator_wallet`.
    pub async fn register_agent_template(
        &self,
        name: &str,
        description: &str,
        template_type: &str,
        system_prompt: &str,
        tags: Vec<String>,
        creator_did: Option<&str>,
        creator_wallet: Option<&str>,
        pricing: &str,
    ) -> SdkResult<AgentTemplate> {
        let mut params = serde_json::Map::new();
        params.insert("name".into(), serde_json::json!(name));
        params.insert("description".into(), serde_json::json!(description));
        params.insert("template_type".into(), serde_json::json!(template_type));
        params.insert("system_prompt".into(), serde_json::json!(system_prompt));
        params.insert("tags".into(), serde_json::json!(tags));
        params.insert("pricing".into(), serde_json::json!(pricing));
        if let Some(did) = creator_did {
            params.insert("creator_did".into(), serde_json::json!(did));
        }
        if let Some(wallet) = creator_wallet {
            params.insert("creator_wallet".into(), serde_json::json!(wallet));
        }

        let value: serde_json::Value = self
            .rpc
            .call(
                "tenzro_registerAgentTemplate",
                serde_json::Value::Object(params),
            )
            .await?;

        serde_json::from_value(value).map_err(|e| {
            SdkError::RpcError(format!("Failed to parse AgentTemplate: {}", e))
        })
    }

    /// Invokes (runs) a spawned agent template end-to-end.
    ///
    /// For paid templates the `payer_wallet` is charged per invocation:
    /// * `AGENT_MARKETPLACE_COMMISSION_BPS` (5%) flows to the network treasury
    /// * the remainder is paid to the template's `creator_wallet`
    ///
    /// Returns the raw report JSON from `tenzro_runAgentTemplate`, which
    /// includes: `template_id`, `steps_executed`, `steps_failed`,
    /// `steps_skipped_by_dry_run`, `fee_paid`, `commission_bps`,
    /// `network_commission`, `creator_share`, `payer_wallet`,
    /// `creator_wallet`, `treasury`, `invocation_count`, `total_revenue`.
    pub async fn run_agent_template(
        &self,
        agent_id: &str,
        payer_wallet: Option<&str>,
        tokens_estimate: u64,
        max_iterations: u64,
        dry_run: bool,
    ) -> SdkResult<serde_json::Value> {
        let mut params = serde_json::Map::new();
        params.insert("agent_id".into(), serde_json::json!(agent_id));
        params.insert("tokens_estimate".into(), serde_json::json!(tokens_estimate));
        params.insert("max_iterations".into(), serde_json::json!(max_iterations));
        params.insert("dry_run".into(), serde_json::json!(dry_run));
        if let Some(wallet) = payer_wallet {
            params.insert("payer_wallet".into(), serde_json::json!(wallet));
        }

        self.rpc
            .call(
                "tenzro_runAgentTemplate",
                serde_json::Value::Object(params),
            )
            .await
    }

    /// Spawns a new agent instance from a marketplace template.
    ///
    /// When `parent_machine_did` is `Some`, the spawned agent's effective
    /// delegation scope is the strict intersection of the parent's scope
    /// and the template's spec — the child can never be broader than its
    /// parent on any axis (numeric ceilings, allow-lists, time bound).
    pub async fn spawn_agent_from_template(
        &self,
        template_id: &str,
        name: &str,
        parent_machine_did: Option<&str>,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_spawnAgentFromTemplate",
                serde_json::json!([{
                    "template_id": template_id,
                    "name": name,
                    "parent_machine_did": parent_machine_did,
                }]),
            )
            .await
    }

    /// Rates an agent template on the marketplace
    pub async fn rate_agent_template(
        &self,
        template_id: &str,
        rating: u8,
        review: Option<&str>,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_rateAgentTemplate",
                serde_json::json!([{ "template_id": template_id, "rating": rating, "review": review }]),
            )
            .await
    }

    /// Searches agent templates by free-text query
    pub async fn search_agent_templates(
        &self,
        query: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_searchAgentTemplates",
                serde_json::json!([{ "query": query }]),
            )
            .await
    }

    /// Gets usage and rating statistics for an agent template
    pub async fn get_agent_template_stats(
        &self,
        template_id: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getAgentTemplateStats",
                serde_json::json!([{ "template_id": template_id }]),
            )
            .await
    }

    /// Gets an agent template by its ID
    pub async fn get_agent_template(&self, template_id: &str) -> SdkResult<AgentTemplate> {
        let value: serde_json::Value = self
            .rpc
            .call(
                "tenzro_getAgentTemplate",
                serde_json::json!({ "template_id": template_id }),
            )
            .await?;

        serde_json::from_value(value).map_err(|e| {
            SdkError::RpcError(format!("Failed to parse AgentTemplate: {}", e))
        })
    }
}
