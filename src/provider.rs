//! Provider client for Tenzro Network
//!
//! This module provides functionality for network participation, model serving,
//! and provider operations.

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

/// Provider client for network participation and model serving
pub struct ProviderClient {
    rpc: Arc<RpcClient>,
}

impl ProviderClient {
    /// Creates a new provider client
    pub fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// One-click network participation — provisions identity, wallet, and hardware profile
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// let result = provider.participate("my-secure-password").await?;
    /// println!("DID: {}", result.did);
    /// println!("Address: {}", result.address);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn participate(&self, password: &str) -> SdkResult<ParticipateResponse> {
        let result = self.rpc.call("tenzro_participate", json!([password])).await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse participate response: {}", e)))
    }

    /// Download a model from the registry
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// let status = provider.download_model("gemma4-9b").await?;
    /// println!("Download {}: {:.1}%", status.status, status.progress_percent);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_model(&self, model_id: &str) -> SdkResult<DownloadProgress> {
        let result = self
            .rpc
            .call("tenzro_downloadModel", json!({ "model_id": model_id }))
            .await?;
        serde_json::from_value(result).map_err(|e| {
            SdkError::RpcError(format!("Failed to parse download status: {}", e))
        })
    }

    /// Get download progress for a model
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// let progress = provider.get_download_progress("gemma4-9b").await?;
    /// println!("Progress: {:.1}%", progress.progress_percent);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_download_progress(&self, model_id: &str) -> SdkResult<DownloadProgress> {
        let result = self
            .rpc
            .call("tenzro_getDownloadProgress", json!({ "model_id": model_id }))
            .await?;
        serde_json::from_value(result).map_err(|e| {
            SdkError::RpcError(format!("Failed to parse download progress: {}", e))
        })
    }

    /// Start serving a model on the network with default placement.
    ///
    /// When a model is too large for a single host, the node auto-clusters:
    /// it reads the GGUF header for layer count and hidden dimension,
    /// discovers LAN members from gossiped cluster announcements, and runs a
    /// layer-wise pipeline across them. No extra arguments are required — the
    /// node decides single-host vs. cluster from the model shape and the
    /// reachable members. Use [`serve_model_with`](Self::serve_model_with) to
    /// force a cluster, force single-host, or serve privately.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// provider.serve_model("gemma4-9b").await?;
    /// println!("Now serving model");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn serve_model(&self, model_id: &str) -> SdkResult<()> {
        self.serve_model_with(model_id, ServeOptions::default()).await
    }

    /// Start serving a model with explicit placement and visibility options.
    ///
    /// - `force_cluster` (`user_forced`): split across the LAN cluster even
    ///   when the model fits one host (trades decode speed for memory).
    /// - `force_single`: never form a cluster; serve single-host (errors at
    ///   the node only if the model cannot fit, which the SDK surfaces).
    /// - `visibility`: `Network` (default) gossips the model so any peer can
    ///   route to it; `Private` registers it locally without announcing, so it
    ///   is reachable only over a direct/LAN connection.
    ///
    /// Use [`Discovery::cluster_plan`](crate::discovery::Discovery::cluster_plan)
    /// first to preview the layer split before serving.
    pub async fn serve_model_with(
        &self,
        model_id: &str,
        opts: ServeOptions,
    ) -> SdkResult<()> {
        let mut params = serde_json::Map::new();
        params.insert("model_id".into(), json!(model_id));
        if opts.force_cluster {
            params.insert("user_forced".into(), json!(true));
        }
        if opts.force_single {
            params.insert("force_single".into(), json!(true));
        }
        params.insert("visibility".into(), json!(opts.visibility.as_str()));
        self.rpc
            .call::<serde_json::Value>("tenzro_serveModel", Value::Object(params))
            .await?;
        Ok(())
    }

    /// Stop serving a model
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// provider.stop_model("gemma4-9b").await?;
    /// println!("Stopped serving model");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stop_model(&self, model_id: &str) -> SdkResult<()> {
        self.rpc
            .call::<serde_json::Value>("tenzro_stopModel", json!({ "model_id": model_id }))
            .await?;
        Ok(())
    }

    /// Delete a downloaded model
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// provider.delete_model("gemma4-9b").await?;
    /// println!("Model deleted");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_model(&self, model_id: &str) -> SdkResult<()> {
        self.rpc
            .call::<serde_json::Value>("tenzro_deleteModel", json!([model_id]))
            .await?;
        Ok(())
    }

    /// Chat with a loaded model
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # use tenzro_sdk::provider::ChatMessage;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// let messages = vec![
    ///     ChatMessage {
    ///         role: "user".to_string(),
    ///         content: "What is Tenzro Network?".to_string(),
    ///     }
    /// ];
    /// let response = provider.chat("gemma4-9b", messages).await?;
    /// println!("Response: {}", response.response);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn chat(&self, model_id: &str, messages: Vec<ChatMessage>) -> SdkResult<ChatResponse> {
        let result = self
            .rpc
            .call(
                "tenzro_chat",
                json!({ "model_id": model_id, "messages": messages }),
            )
            .await?;
        parse_rich_chat_response(result)
    }

    /// Send a chat completion with generation options. Use this when
    /// you need to pass [`ChatOptions`] (Multi-Token Prediction
    /// `draft_n`, max_tokens, temperature, top_p, etc.).
    ///
    /// MTP throughput uplift requires the target model to declare a
    /// drafter in its catalog entry (`HfModelEntry.drafter_id` +
    /// `mtp_kind`). Gemma 4 12B and 31B currently advertise this. When
    /// `draft_n` is set on a model whose runtime cannot satisfy it,
    /// the node returns a structured `MtpUnavailable` error.
    pub async fn chat_with(
        &self,
        model_id: &str,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> SdkResult<ChatResponse> {
        let mut params = json!({
            "model_id": model_id,
            "messages": messages,
        });
        if let Some(t) = opts.temperature {
            params["temperature"] = json!(t);
        }
        if let Some(p) = opts.top_p {
            params["top_p"] = json!(p);
        }
        if let Some(m) = opts.max_tokens {
            params["max_tokens"] = json!(m);
        }
        if let Some(n) = opts.draft_n {
            params["draft_n"] = json!(n);
        }
        let result = self.rpc.call("tenzro_chat", params).await?;
        parse_rich_chat_response(result)
    }

    /// Get hardware profile of the node
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// let profile = provider.get_hardware_profile().await?;
    /// println!("CPU: {}", profile.cpu_model);
    /// println!("Memory: {} GB", profile.total_ram_gb);
    /// for gpu in &profile.gpus {
    ///     println!("{:?} {} ({} GiB)", gpu.vendor, gpu.name, gpu.vram_gb);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_hardware_profile(&self) -> SdkResult<HardwareProfile> {
        let result = self.rpc.call("tenzro_getHardwareProfile", json!([])).await?;
        serde_json::from_value(result).map_err(|e| {
            SdkError::RpcError(format!("Failed to parse hardware profile: {}", e))
        })
    }

    /// Set this node's active roles. One stake backs every role.
    ///
    /// `roles` is a comma-separated set of role tokens: `validator`, `ai`,
    /// `storage`, `tee`, `user`. The empty string resolves to a client node.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// provider.set_roles("validator,storage,ai").await?;
    /// println!("Roles updated");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_roles(&self, roles: &str) -> SdkResult<()> {
        self.rpc
            .call::<serde_json::Value>("tenzro_setRole", json!([{ "roles": roles }]))
            .await?;
        Ok(())
    }

    /// Get this node's active roles (normalized role-token strings).
    pub async fn get_roles(&self) -> SdkResult<Vec<String>> {
        let result: serde_json::Value = self.rpc.call("tenzro_getRole", json!([])).await?;
        let roles = result
            .get("roles")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        Ok(roles)
    }

    /// Register as a provider on the network
    ///
    /// Model/inference providers do not need to stake TNZO — pass `0` for
    /// `stake`. Staking is only required for validators.
    ///
    /// # Arguments
    ///
    /// * `provider_type` - One of `"validator"`, `"model_provider"`, `"tee_provider"`, `"storage_provider"`
    /// * `models` - List of model IDs this provider serves (informational; not stored on-chain by handler)
    /// * `stake` - Stake amount in **wei** (10^-18 TNZO). Pass `0` for non-validator providers.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// let models = vec!["gemma3-270m".to_string()];
    /// // No staking required for model providers — pass 0 wei
    /// let tx_hash = provider.register("model_provider", models, 0).await?;
    /// println!("Registration tx: {}", tx_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register(
        &self,
        provider_type: &str,
        models: Vec<String>,
        stake: u128,
    ) -> SdkResult<String> {
        let result: serde_json::Value = self
            .rpc
            .call(
                "tenzro_registerProvider",
                json!([{
                    "provider_type": provider_type,
                    "models": models,
                    "stake": stake.to_string(),
                }]),
            )
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// Get provider statistics
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// let stats = provider.stats().await?;
    /// println!("Serving: {}", stats.is_serving);
    /// println!("Total inferences: {}", stats.total_inferences);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stats(&self) -> SdkResult<ProviderStats> {
        let result = self.rpc.call("tenzro_providerStats", json!([])).await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse provider stats: {}", e)))
    }

    /// List all model service endpoints with load information
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// let endpoints = provider.list_model_endpoints().await?;
    /// for ep in &endpoints {
    ///     println!("{}: {} ({})", ep.model_name, ep.status, ep.location);
    ///     if let Some(load) = &ep.load {
    ///         println!("  Load: {}/{} ({}%)", load.active_requests, load.max_concurrent, load.utilization_percent);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_model_endpoints(&self) -> SdkResult<Vec<ModelEndpoint>> {
        let result = self.rpc.call("tenzro_listModelEndpoints", json!([])).await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse model endpoints: {}", e)))
    }

    /// Get load information for a specific model
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// if let Some(load) = provider.get_model_load("gemma3-270m").await? {
    ///     println!("Load: {}/{} ({}%)", load.active_requests, load.max_concurrent, load.utilization_percent);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_model_load(&self, model_id: &str) -> SdkResult<Option<ModelLoad>> {
        let endpoints: Vec<ModelEndpoint> = self.list_model_endpoints().await?;
        Ok(endpoints.into_iter()
            .find(|ep| ep.model_id == model_id || ep.model_name == model_id)
            .and_then(|ep| ep.load))
    }

    /// Joins as a micro node (light participant)
    pub async fn join_as_micro_node(&self, display_name: Option<&str>, participant_type: Option<&str>) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_joinAsMicroNode",
                serde_json::json!([{"display_name": display_name, "participant_type": participant_type}]),
            )
            .await
    }

    /// Sets the provider availability schedule
    pub async fn set_provider_schedule(&self, schedule: serde_json::Value) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_setProviderSchedule", serde_json::json!(schedule))
            .await
    }

    /// Gets the current provider schedule
    pub async fn get_provider_schedule(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_getProviderSchedule", serde_json::json!([]))
            .await
    }

    /// Sets provider pricing configuration
    pub async fn set_provider_pricing(&self, pricing: serde_json::Value) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_setProviderPricing", serde_json::json!(pricing))
            .await
    }

    /// Gets the current provider pricing
    pub async fn get_provider_pricing(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_getProviderPricing", serde_json::json!([]))
            .await
    }

    /// Gets a specific model endpoint by instance_id or model_id
    pub async fn get_model_endpoint(&self, instance_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getModelEndpoint",
                serde_json::json!({"instance_id": instance_id}),
            )
            .await
    }

    /// Registers a remote model endpoint
    pub async fn register_model_endpoint(&self, model_id: &str, api_endpoint: &str, mcp_endpoint: Option<&str>, model_name: Option<&str>, provider_name: Option<&str>) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_registerModelEndpoint",
                serde_json::json!({"model_id": model_id, "api_endpoint": api_endpoint, "mcp_endpoint": mcp_endpoint, "model_name": model_name, "provider_name": provider_name}),
            )
            .await
    }

    /// Unregisters a model endpoint
    pub async fn unregister_model_endpoint(&self, instance_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_unregisterModelEndpoint",
                serde_json::json!({"instance_id": instance_id}),
            )
            .await
    }

    /// Adds a resource to the node
    pub async fn add_resource(&self, resource_id: &str, resource_type: Option<&str>) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_addResource",
                serde_json::json!([{"resource_id": resource_id, "resource_type": resource_type}]),
            )
            .await
    }

    /// Sends a signed transaction
    pub async fn send_signed_transaction(&self, from: &str, to: &str, amount: &str, asset: Option<&str>) -> SdkResult<String> {
        self.rpc
            .call(
                "tenzro_sendTransaction",
                serde_json::json!([{"from": from, "to": to, "amount": amount, "asset": asset}]),
            )
            .await
    }

    /// Submits a block to the node
    pub async fn submit_block(&self, block: serde_json::Value) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_submitBlock", serde_json::json!(block))
            .await
    }

    /// Gets the node status including health, block height, and peer count
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// let status = provider.get_node_status().await?;
    /// println!("Status: {:?}", status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_node_status(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_getNodeStatus", json!([]))
            .await
    }

    /// Gets provider statistics for a specific address
    ///
    /// Returns served models, inference counts, and staking totals.
    ///
    /// # Arguments
    ///
    /// * `address` - Provider address (hex)
    pub async fn get_provider_stats(&self, address: &str) -> SdkResult<ProviderStats> {
        let result = self
            .rpc
            .call("tenzro_getProviderStats", json!([{ "address": address }]))
            .await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse provider stats: {}", e)))
    }

    /// Starts serving a model via the MCP server
    ///
    /// Registers as a provider and begins serving the specified model.
    ///
    /// # Arguments
    ///
    /// * `model_id` - Model identifier to serve
    pub async fn serve_model_mcp(&self, model_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_serveModelMcp", json!([{ "model_id": model_id }]))
            .await
    }

    /// List all providers discovered on the Tenzro Network via gossipsub announcements.
    ///
    /// Returns both the local node (if it is actively serving models) and all remote providers
    /// discovered via the `tenzro/providers` gossipsub topic. Each node serving models
    /// broadcasts a `ProviderAnnouncement` message every 60 seconds; this call merges the
    /// local provider entry with all gossipsub-discovered entries.
    ///
    /// # Example
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let provider = client.provider();
    /// let providers = provider.list_providers().await?;
    /// for p in &providers {
    ///     println!("{}: {} ({}) — models: {:?}",
    ///         p.peer_id, p.provider_type, p.status, p.served_models);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_providers(&self) -> SdkResult<Vec<NetworkProvider>> {
        let result = self.rpc.call("tenzro_listProviders", serde_json::json!({})).await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse list_providers response: {}", e)))
    }
}

/// Maps the node's rich chat envelope (Anthropic Messages-style:
/// `{model, content: [{type: "text", text}, ...], usage}`) into the flat
/// [`ChatResponse`]. Text blocks are concatenated; thinking/tool blocks
/// are skipped.
fn parse_rich_chat_response(result: serde_json::Value) -> SdkResult<ChatResponse> {
    let content = result
        .get("content")
        .and_then(|c| c.as_array())
        .ok_or_else(|| {
            SdkError::RpcError(format!(
                "chat response missing 'content' blocks: {}",
                result
            ))
        })?;
    let response = content
        .iter()
        .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
        .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join("");
    let model_id = result
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let usage = result.get("usage");
    let tokens_used = usage
        .and_then(|u| u.get("input_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        + usage
            .and_then(|u| u.get("output_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
    Ok(ChatResponse {
        response,
        model_id,
        tokens_used: tokens_used as u32,
    })
}

/// Whether a served model is announced to the network or kept local-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    /// Gossip the model so any peer can route inference to it.
    #[default]
    Network,
    /// Register locally without announcing; reachable only over a direct/LAN
    /// connection.
    Private,
}

impl Visibility {
    /// Wire string sent to `tenzro_serveModel`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Network => "network",
            Visibility::Private => "private",
        }
    }
}

/// Placement and visibility options for [`ProviderClient::serve_model_with`].
#[derive(Debug, Clone, Default)]
pub struct ServeOptions {
    /// Split across the LAN cluster even when the model fits one host.
    pub force_cluster: bool,
    /// Never form a cluster; serve single-host.
    pub force_single: bool,
    /// Network (default) or private visibility.
    pub visibility: Visibility,
}

/// Response from network participation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipateResponse {
    /// The DID created for this identity
    pub did: String,
    /// The wallet address
    pub address: String,
    /// Hardware profile detected
    pub hardware_profile: serde_json::Value,
}

/// Provider statistics information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    /// Whether the provider is currently serving
    pub is_serving: bool,
    /// List of model IDs being served
    pub models_served: Vec<String>,
    /// Total number of inferences processed
    pub total_inferences: u64,
}

/// A chat message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message role (user, assistant, system)
    pub role: String,
    /// Message content
    pub content: String,
}

/// Response from a chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// The generated response text
    pub response: String,
    /// Model ID that generated the response
    pub model_id: String,
    /// Number of tokens used
    pub tokens_used: u32,
}

/// Optional generation knobs for [`ProviderClient::chat_with`].
///
/// Use the `with_*` builder methods to set fields fluently; all
/// fields default to `None`, in which case the node falls back to
/// its own per-model defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatOptions {
    /// Sampling temperature (0.0..=2.0). Default: 0.7.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Top-p nucleus-sampling threshold. Default: 0.9.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Maximum new tokens to generate. Default: 512.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Multi-Token-Prediction draft count (1..=6). Only meaningful on
    /// targets whose catalog entry declares a drafter
    /// (`HfModelEntry.mtp_kind == DraftMtp`). Unsloth recommends 2 as
    /// a starting point on Gemma 4; optimal value is hardware-
    /// dependent. When the runtime can't satisfy MTP the node returns
    /// a structured `MtpUnavailable` error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_n: Option<u8>,
}

impl ChatOptions {
    /// Set the sampling temperature.
    pub fn with_temperature(mut self, t: f64) -> Self {
        self.temperature = Some(t);
        self
    }
    /// Set the top-p threshold.
    pub fn with_top_p(mut self, p: f64) -> Self {
        self.top_p = Some(p);
        self
    }
    /// Set the max-tokens cap.
    pub fn with_max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = Some(n);
        self
    }
    /// Enable Multi-Token Prediction with the given draft count
    /// (clamped to 1..=6).
    pub fn with_draft_n(mut self, n: u8) -> Self {
        self.draft_n = Some(n.clamp(1, 6));
        self
    }
}

/// Model download progress
///
/// Mirrors the node's `ModelDownloadStatus` wire shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// Model ID being downloaded
    #[serde(default)]
    pub model_id: String,
    /// Download status (not_started, downloading, completed, failed)
    #[serde(default)]
    pub status: String,
    /// Progress as a percentage (0.0 to 100.0)
    #[serde(default)]
    pub progress_percent: f64,
    /// Bytes downloaded so far
    #[serde(default)]
    pub downloaded_bytes: u64,
    /// Total bytes to download
    #[serde(default)]
    pub total_bytes: u64,
    /// Error message when status is "failed"
    #[serde(default)]
    pub error: Option<String>,
}

/// Accelerator vendor as reported by the node's hardware probe. Drives the
/// compute-capability interpretation and the FP8/FP4 derivation rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuVendor {
    /// NVIDIA — compute capability is the SM version string ("8.9", "9.0").
    Nvidia,
    /// AMD — compute capability is the gfx target ("gfx942", "gfx1100").
    Amd,
    /// Apple Silicon — unified-memory Metal GPU; no discrete VRAM.
    Apple,
    /// Any other accelerator (no precision derivation applied).
    Other,
}

/// A single accelerator discovered by the node's hardware probe. A node may
/// expose more than one, so [`HardwareProfile::gpus`] is a list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    /// Accelerator vendor.
    pub vendor: GpuVendor,
    /// Device marketing name (e.g. "NVIDIA H100 80GB HBM3", "Apple M3 Max").
    pub name: String,
    /// Device memory in GiB (unified-memory budget on Apple Silicon).
    pub vram_gb: u32,
    /// Vendor-native compute-capability string: SM version for NVIDIA, gfx
    /// target for AMD, "metal" for Apple. Empty when the probe could not read it.
    pub compute_capability: String,
    /// Hardware FP8 matrix/tensor units present.
    pub fp8: bool,
    /// Hardware FP4 matrix/tensor units present.
    pub fp4: bool,
}

/// Hardware profile of a node, as returned by `tenzro_getHardwareProfile`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    /// CPU model string.
    pub cpu_model: String,
    /// Physical core count.
    pub cpu_cores: usize,
    /// Logical thread count.
    pub cpu_threads: usize,
    /// Total system RAM in GB.
    pub total_ram_gb: f64,
    /// Every accelerator the node detected. Empty on a CPU-only node.
    pub gpus: Vec<GpuDevice>,
    /// Free storage in GB under the node's data dir.
    pub storage_available_gb: f64,
    /// Whether a TEE is available on this host.
    pub tee_available: bool,
    /// TEE vendor string when `tee_available` (e.g. "Intel TDX"), else `None`.
    pub tee_vendor: Option<String>,
    /// Operating system.
    pub os: String,
    /// CPU architecture.
    pub arch: String,
    /// Stable per-device fingerprint.
    pub device_fingerprint: String,
}

/// Load information for a model being served
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLoad {
    /// Number of active inference requests
    pub active_requests: u32,
    /// Maximum concurrent requests supported
    pub max_concurrent: u32,
    /// Current utilization percentage (0-100)
    pub utilization_percent: u8,
    /// Load level classification
    pub load_level: String,
}

/// Model pricing information (TNZO per token)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Cost per input token in TNZO
    #[serde(default)]
    pub input_per_token: f64,
    /// Cost per output token in TNZO
    #[serde(default)]
    pub output_per_token: f64,
    /// Currency (always "TNZO")
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "TNZO".to_string()
}

/// Model service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEndpoint {
    /// Model name
    pub model_name: String,
    /// Model ID
    #[serde(default)]
    pub model_id: String,
    /// Instance ID
    pub instance_id: String,
    /// API endpoint URL
    #[serde(default)]
    pub api_endpoint: String,
    /// MCP endpoint URL
    #[serde(default)]
    pub mcp_endpoint: String,
    /// Location (local or network)
    #[serde(default)]
    pub location: String,
    /// iroh EndpointId of the serving node (hex-encoded). Populated for
    /// network-discovered endpoints so callers can see the NAT-agnostic
    /// address a remote model is reached at. Empty for local services.
    #[serde(default)]
    pub iroh_endpoint_id: String,
    /// Provider name
    #[serde(default)]
    pub provider_name: String,
    /// Service status
    #[serde(default)]
    pub status: String,
    /// Availability: "local", "downloaded", "downloadable", or "network"
    #[serde(default)]
    pub availability: String,
    /// Pricing information
    #[serde(default)]
    pub pricing: Option<ModelPricing>,
    /// Load information (when serving)
    #[serde(default)]
    pub load: Option<ModelLoad>,
}

/// A provider discovered on the Tenzro Network via gossipsub announcements.
///
/// Providers are discovered via the `tenzro/providers` gossipsub topic.
/// Each node serving models broadcasts a `ProviderAnnouncement` every 60s and all
/// peers merge announcements into their `network_providers` cache.
///
/// Use [`ProviderClient::list_providers`] to discover providers via
/// the `tenzro_listProviders` JSON-RPC method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkProvider {
    /// libp2p peer ID of the announcing node
    pub peer_id: String,
    /// Wallet/account address of the provider
    pub provider_address: String,
    /// Provider type (e.g. "llm", "tee", "general")
    #[serde(default)]
    pub provider_type: String,
    /// Model IDs currently being served by this node
    #[serde(default)]
    pub served_models: Vec<String>,
    /// Capability labels (e.g. "inference", "tee-attestation")
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// HTTP RPC endpoint for direct inference routing (e.g. "http://10.128.0.5:8545")
    #[serde(default)]
    pub rpc_endpoint: String,
    /// Lifecycle status (e.g. "active", "draining")
    #[serde(default)]
    pub status: String,
    /// Whether this is the local node
    #[serde(default)]
    pub is_local: bool,
}
