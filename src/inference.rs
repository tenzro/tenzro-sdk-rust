//! Inference SDK for Tenzro Network
//!
//! This module provides AI model inference functionality.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use crate::types::ModelInfo;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Inference client for AI model operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let inference = client.inference();
///
/// // List available models
/// let models = inference.list_models().await?;
/// println!("Found {} models", models.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct InferenceClient {
    rpc: Arc<RpcClient>,
}

impl InferenceClient {
    /// Creates a new inference client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Lists all available models on the network
    pub async fn list_models(&self) -> SdkResult<Vec<ModelInfo>> {
        self.rpc
            .call("tenzro_listModels", serde_json::json!([]))
            .await
    }

    /// Submits an inference request to a model
    pub async fn request(
        &self,
        model_id: &str,
        input: &str,
        max_tokens: Option<u32>,
    ) -> SdkResult<InferenceResult> {
        let mut params = serde_json::json!({
            "model_id": model_id,
            "input": input,
        });

        if let Some(mt) = max_tokens {
            params["max_tokens"] = serde_json::json!(mt);
        }

        self.rpc
            .call("tenzro_inferenceRequest", serde_json::json!([params]))
            .await
    }

    /// Estimates the cost of an inference request
    pub async fn estimate_cost(&self, _model_id: &str, input_tokens: u32) -> SdkResult<u64> {
        let cost_per_input_token = 10u64;
        let cost_per_output_token = 20u64;
        let estimated_output_tokens = input_tokens * 2;

        let total_cost = (input_tokens as u64 * cost_per_input_token)
            + (estimated_output_tokens as u64 * cost_per_output_token);

        Ok(total_cost)
    }
}

/// Inference result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    /// Model output text
    #[serde(default)]
    pub output: String,
    /// Request ID
    #[serde(default)]
    pub request_id: String,
    /// Model ID that processed the request
    #[serde(default)]
    pub model_id: String,
    /// Tokens used
    #[serde(default)]
    pub tokens_used: u64,
}
