//! Task marketplace SDK for Tenzro Network
//!
//! This module provides task posting, listing, and quote submission functionality.

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use crate::types::{Address, TaskInfo, TaskQuote};
use std::sync::Arc;

/// Task client for task marketplace operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let task_client = client.task();
///
/// // Post a new task
/// let task = task_client.post_task(
///     "Analyze sentiment",
///     "Analyze sentiment of customer reviews",
///     "inference",
///     1_000_000_000_000_000_000u128,
///     "[\"Great product!\", \"Needs improvement\"]",
/// ).await?;
/// println!("Task posted: {}", task.task_id);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct TaskClient {
    rpc: Arc<RpcClient>,
}

impl TaskClient {
    /// Creates a new task client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Posts a new task to the marketplace
    pub async fn post_task(
        &self,
        title: &str,
        description: &str,
        task_type: &str,
        max_price: u128,
        input: &str,
    ) -> SdkResult<TaskInfo> {
        let poster_hex = format!("0x{}", hex::encode(Address::zero().as_bytes()));
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

    /// Assigns a task to a specific agent
    pub async fn assign_task(&self, task_id: &str, agent_id: &str) -> SdkResult<TaskInfo> {
        let value: serde_json::Value = self
            .rpc
            .call(
                "tenzro_assignTask",
                serde_json::json!({
                    "task_id": task_id,
                    "agent_id": agent_id,
                }),
            )
            .await?;

        serde_json::from_value(value)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse TaskInfo: {}", e)))
    }

    /// Completes a task with the given result
    pub async fn complete_task(
        &self,
        task_id: &str,
        result: &str,
    ) -> SdkResult<TaskInfo> {
        let value: serde_json::Value = self
            .rpc
            .call(
                "tenzro_completeTask",
                serde_json::json!({
                    "task_id": task_id,
                    "result": result,
                }),
            )
            .await?;

        serde_json::from_value(value)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse TaskInfo: {}", e)))
    }

    /// Submits a quote for a task
    pub async fn submit_quote(
        &self,
        task_id: &str,
        price: u128,
        model_id: &str,
        estimated_secs: u64,
        confidence: u8,
        notes: Option<String>,
    ) -> SdkResult<TaskQuote> {
        let provider_hex = format!("0x{}", hex::encode(Address::zero().as_bytes()));
        let mut params = serde_json::Map::new();
        params.insert("task_id".to_string(), serde_json::json!(task_id));
        params.insert("provider".to_string(), serde_json::json!(provider_hex));
        params.insert("price".to_string(), serde_json::json!(price.to_string()));
        params.insert("model_id".to_string(), serde_json::json!(model_id));
        params.insert(
            "estimated_duration_secs".to_string(),
            serde_json::json!(estimated_secs),
        );
        params.insert("confidence".to_string(), serde_json::json!(confidence));
        if let Some(n) = notes {
            params.insert("notes".to_string(), serde_json::json!(n));
        }

        let value: serde_json::Value = self
            .rpc
            .call("tenzro_quoteTask", serde_json::Value::Object(params))
            .await?;

        serde_json::from_value(value)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse TaskQuote: {}", e)))
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
