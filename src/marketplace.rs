//! Agent marketplace SDK for Tenzro Network
//!
//! This module provides agent template listing, registration, and retrieval functionality.

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use crate::types::{Address, AgentTemplate};
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

    /// Registers a new agent template on the marketplace
    pub async fn register_agent_template(
        &self,
        name: &str,
        description: &str,
        template_type: &str,
        system_prompt: &str,
        tags: Vec<String>,
        pricing: serde_json::Value,
    ) -> SdkResult<AgentTemplate> {
        let creator_hex = format!("0x{}", hex::encode(Address::zero().as_bytes()));
        let value: serde_json::Value = self
            .rpc
            .call(
                "tenzro_registerAgentTemplate",
                serde_json::json!({
                    "name": name,
                    "description": description,
                    "template_type": template_type,
                    "system_prompt": system_prompt,
                    "creator": creator_hex,
                    "tags": tags,
                    "pricing": pricing,
                }),
            )
            .await?;

        serde_json::from_value(value).map_err(|e| {
            SdkError::RpcError(format!("Failed to parse AgentTemplate: {}", e))
        })
    }

    /// Spawns a new agent instance from a marketplace template
    pub async fn spawn_agent_from_template(
        &self,
        template_id: &str,
        name: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_spawnAgentFromTemplate",
                serde_json::json!([{ "template_id": template_id, "name": name }]),
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
