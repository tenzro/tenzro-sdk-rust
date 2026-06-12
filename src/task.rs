//! Task marketplace SDK for Tenzro Network
//!
//! Drives the full task-marketplace settlement cycle against the live RPC:
//!
//! ```text
//!   post_task   →  quote_task   →  assign_task   →  complete_task
//!   (poster)      (provider)      (locks price)    (transfers TNZO)
//! ```
//!
//! `complete_task` is the moneyed step: the RPC handler issues a real
//! `tenzro-token` transfer of the locked price (`quoted_price`, falling
//! back to `max_price`) from poster to assignee through the unified
//! token registry — observable as a balance delta via
//! `eth_getBalance` / `tenzro_getTokenBalance`.

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use crate::types::{Address, TaskInfo, TaskQuote};
use std::sync::Arc;

/// Task client for task marketplace operations.
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # use tenzro_sdk::task::QuoteOpts;
/// # use tenzro_sdk::types::Address;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// # let poster: Address = unimplemented!();
/// # let provider: Address = unimplemented!();
/// let task_client = client.task();
///
/// // 1. Poster opens a task
/// let task = task_client.post_task(
///     "Analyze sentiment",
///     "Score customer reviews",
///     "inference",
///     1_000_000_000_000_000_000u128, // 1 TNZO max_price
///     "[\"Great product!\", \"Needs improvement\"]",
///     &poster,
/// ).await?;
///
/// // 2. Provider quotes
/// task_client.quote_task(
///     &task.task_id,
///     &provider,
///     900_000_000_000_000_000u128, // 0.9 TNZO
///     QuoteOpts::default(),
/// ).await?;
///
/// // 3. Poster assigns
/// task_client.assign_task(
///     &task.task_id,
///     &provider,
///     Some(900_000_000_000_000_000u128),
/// ).await?;
///
/// // 4. Settlement — real on-chain TNZO transfer poster → provider
/// let receipt = task_client.complete_task(&task.task_id, "score=4.2/5").await?;
/// println!("settled: {:?}", receipt.settlement);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct TaskClient {
    rpc: Arc<RpcClient>,
}

/// Optional knobs for `quote_task`. The RPC fills sensible defaults
/// (`model_id="any"`, `confidence=80`, `estimated_duration_secs=60`).
#[derive(Clone, Default, Debug)]
pub struct QuoteOpts {
    pub model_id: Option<String>,
    pub confidence: Option<u8>,
    pub estimated_duration_secs: Option<u64>,
    pub notes: Option<String>,
}

/// Receipt returned by `complete_task`. The `settlement` block carries
/// the on-chain post-transfer balances reported by the RPC handler.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct CompleteTaskReceipt {
    pub task_id: String,
    pub status: String,
    #[serde(default)]
    pub settlement: serde_json::Value,
}

impl TaskClient {
    /// Creates a new task client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Post a new task to the marketplace.
    ///
    /// `poster` is the wallet that will be charged at settlement time
    /// — it must already have a TNZO balance >= `max_price`.
    pub async fn post_task(
        &self,
        title: &str,
        description: &str,
        task_type: &str,
        max_price: u128,
        input: &str,
        poster: &Address,
    ) -> SdkResult<TaskInfo> {
        let poster_hex = format!("0x{}", hex::encode(poster.as_bytes()));
        let value = self
            .rpc
            .call::<serde_json::Value>(
                "tenzro_postTask",
                serde_json::json!({
                    "title": title,
                    "description": description,
                    "task_type": task_type,
                    "max_price": max_price.to_string(),
                    "input": input,
                    "poster": poster_hex,
                }),
            )
            .await?;

        serde_json::from_value(value)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse TaskInfo: {}", e)))
    }

    /// Lists tasks from the marketplace
    pub async fn list_tasks(
        &self,
        status_filter: Option<&str>,
        task_type_filter: Option<&str>,
        model_filter: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> SdkResult<Vec<TaskInfo>> {
        let mut params = serde_json::Map::new();
        if let Some(status) = status_filter {
            params.insert("status".to_string(), serde_json::json!(status));
        }
        if let Some(task_type) = task_type_filter {
            params.insert("task_type".to_string(), serde_json::json!(task_type));
        }
        if let Some(model) = model_filter {
            params.insert("required_model".to_string(), serde_json::json!(model));
        }
        if let Some(lim) = limit {
            params.insert("limit".to_string(), serde_json::json!(lim));
        }
        if let Some(off) = offset {
            params.insert("offset".to_string(), serde_json::json!(off));
        }

        let value: serde_json::Value = self
            .rpc
            .call("tenzro_listTasks", serde_json::Value::Object(params))
            .await?;

        let tasks_value = value
            .get("tasks")
            .cloned()
            .unwrap_or(value);

        serde_json::from_value(tasks_value)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse Vec<TaskInfo>: {}", e)))
    }

    /// Gets a task by its ID
    pub async fn get_task(&self, task_id: &str) -> SdkResult<TaskInfo> {
        let value: serde_json::Value = self
            .rpc
            .call(
                "tenzro_getTask",
                serde_json::json!({ "task_id": task_id }),
            )
            .await?;

        serde_json::from_value(value)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse TaskInfo: {}", e)))
    }

    /// Cancels a task
    pub async fn cancel_task(&self, task_id: &str) -> SdkResult<()> {
        let _: serde_json::Value = self
            .rpc
            .call(
                "tenzro_cancelTask",
                serde_json::json!({ "task_id": task_id }),
            )
            .await?;
        Ok(())
    }

    /// Submit a quote for a task. The `provider` address is the wallet
    /// that will receive the TNZO at settlement time if assigned.
    pub async fn quote_task(
        &self,
        task_id: &str,
        provider: &Address,
        price: u128,
        opts: QuoteOpts,
    ) -> SdkResult<TaskQuote> {
        let provider_hex = format!("0x{}", hex::encode(provider.as_bytes()));
        let mut params = serde_json::Map::new();
        params.insert("task_id".to_string(), serde_json::json!(task_id));
        params.insert("provider".to_string(), serde_json::json!(provider_hex));
        params.insert("price".to_string(), serde_json::json!(price.to_string()));
        if let Some(m) = opts.model_id {
            params.insert("model_id".to_string(), serde_json::json!(m));
        }
        if let Some(c) = opts.confidence {
            params.insert("confidence".to_string(), serde_json::json!(c));
        }
        if let Some(d) = opts.estimated_duration_secs {
            params.insert("estimated_duration_secs".to_string(), serde_json::json!(d));
        }
        if let Some(n) = opts.notes {
            params.insert("notes".to_string(), serde_json::json!(n));
        }

        let value: serde_json::Value = self
            .rpc
            .call("tenzro_quoteTask", serde_json::Value::Object(params))
            .await?;

        serde_json::from_value(value)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse TaskQuote: {}", e)))
    }

    /// Assign an open task to a provider wallet.
    ///
    /// `quoted_price` (if `Some`) locks the settlement amount; if
    /// `None`, the task's `max_price` will be paid out at completion.
    pub async fn assign_task(
        &self,
        task_id: &str,
        provider: &Address,
        quoted_price: Option<u128>,
    ) -> SdkResult<serde_json::Value> {
        let provider_hex = format!("0x{}", hex::encode(provider.as_bytes()));
        let mut params = serde_json::Map::new();
        params.insert("task_id".to_string(), serde_json::json!(task_id));
        params.insert("provider".to_string(), serde_json::json!(provider_hex));
        if let Some(p) = quoted_price {
            params.insert("quoted_price".to_string(), serde_json::json!(p.to_string()));
        }

        self.rpc
            .call("tenzro_assignTask", serde_json::Value::Object(params))
            .await
    }

    /// Complete an assigned task and trigger on-chain settlement.
    ///
    /// The RPC handler transfers the locked price (`quoted_price` or
    /// `max_price`) from poster to assignee via the token registry.
    /// The returned `settlement` block contains the post-transfer
    /// balances.
    pub async fn complete_task(
        &self,
        task_id: &str,
        output: &str,
    ) -> SdkResult<CompleteTaskReceipt> {
        let value: serde_json::Value = self
            .rpc
            .call(
                "tenzro_completeTask",
                serde_json::json!({
                    "task_id": task_id,
                    "output": output,
                }),
            )
            .await?;

        serde_json::from_value(value)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse CompleteTaskReceipt: {}", e)))
    }

    /// Updates an existing task
    pub async fn update_task(&self, task_id: &str, title: Option<&str>, description: Option<&str>, status: Option<&str>, output: Option<&str>) -> SdkResult<serde_json::Value> {
        let mut params = serde_json::Map::new();
        params.insert("task_id".to_string(), serde_json::json!(task_id));
        if let Some(t) = title { params.insert("title".to_string(), serde_json::json!(t)); }
        if let Some(d) = description { params.insert("description".to_string(), serde_json::json!(d)); }
        if let Some(s) = status { params.insert("status".to_string(), serde_json::json!(s)); }
        if let Some(o) = output { params.insert("output".to_string(), serde_json::json!(o)); }
        self.rpc.call("tenzro_updateTask", serde_json::Value::Object(params)).await
    }
}
