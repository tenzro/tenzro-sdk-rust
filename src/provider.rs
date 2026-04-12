//! Provider client for Tenzro Network
//!
//! This module provides functionality for network participation, model serving,
//! and provider operations.

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    /// let task_id = provider.download_model("gemma4-9b").await?;
    /// println!("Download started: {}", task_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_model(&self, model_id: &str) -> SdkResult<String> {
        let result: serde_json::Value = self
            .rpc
            .call("tenzro_downloadModel", json!([model_id]))
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
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
    /// println!("Progress: {:.1}%", progress.progress * 100.0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_download_progress(&self, model_id: &str) -> SdkResult<DownloadProgress> {
        let result = self
            .rpc
            .call("tenzro_getDownloadProgress", json!([model_id]))
            .await?;
        serde_json::from_value(result).map_err(|e| {
            SdkError::RpcError(format!("Failed to parse download progress: {}", e))
        })
    }

    /// Start serving a model on the network
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
        self.rpc
            .call::<serde_json::Value>("tenzro_serveModel", json!([model_id]))
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
            .call::<serde_json::Value>("tenzro_stopModel", json!([model_id]))
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
            .call("tenzro_chat", json!([model_id, messages]))
            .await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse chat response: {}", e)))
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
    /// println!("CPU: {}", profile.cpu);
    /// println!("Memory: {} GB", profile.memory_gb);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_hardware_profile(&self) -> SdkResult<HardwareProfile> {
        let result = self.rpc.call("tenzro_getHardwareProfile", json!([])).await?;
        serde_json::from_value(result).map_err(|e| {
            SdkError::RpcError(format!("Failed to parse hardware profile: {}", e))
        })
    }

    /// Set node role (validator, provider, light_client)
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
    /// provider.set_role("provider").await?;
    /// println!("Role updated");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_role(&self, role: &str) -> SdkResult<()> {
        self.rpc
            .call::<serde_json::Value>("tenzro_setRole", json!([role]))
            .await?;
        Ok(())
    }

    /// Register as a provider on the network
    ///
    /// Model/inference providers do not need to stake TNZO — pass `0` for
    /// `stake_amount`. Staking is only required for validators.
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
    /// // No staking required for model providers — pass 0
    /// let tx_hash = provider.register(models, 0).await?;
    /// println!("Registration tx: {}", tx_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register(&self, models: Vec<String>, stake_amount: u64) -> SdkResult<String> {
        let result: serde_json::Value = self
            .rpc
            .call("tenzro_registerProvider", json!([models, stake_amount]))
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
    /// discovered via the `tenzro/providers/1.0.0` gossipsub topic. Each node serving models
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

/// Model download progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// Model ID being downloaded
    pub model_id: String,
    /// Download status (pending, downloading, completed, failed)
    pub status: String,
    /// Progress as a fraction (0.0 to 1.0)
    pub progress: f64,
    /// Bytes downloaded so far
    pub bytes_downloaded: u64,
    /// Total bytes to download
    pub total_bytes: u64,
}

/// Hardware profile of a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    /// CPU model/type
    pub cpu: String,
    /// Memory in GB
    pub memory_gb: u64,
    /// GPU model (if available)
    pub gpu: Option<String>,
    /// List of supported TEE types (tdx, sev-snp, nitro, nvidia-gpu)
    pub tee_support: Vec<String>,
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
/// Providers are discovered via the `tenzro/providers/1.0.0` gossipsub topic.
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
