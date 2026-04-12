//! Tool Registry SDK for Tenzro Network
//!
//! This module provides tool registration, discovery, and execution
//! functionality for the Tenzro Tool Registry.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Tool client for Tool Registry operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let tools = client.tool();
///
/// // Search for tools
/// let results = tools.search("code review").await?;
/// println!("Found {} tools", results.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct ToolClient {
    rpc: Arc<RpcClient>,
}

impl ToolClient {
    /// Creates a new tool client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Registers a new tool in the Tool Registry
    ///
    /// # Arguments
    ///
    /// * `params` - Tool registration parameters
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # use tenzro_sdk::tool::RegisterToolParams;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let tools = client.tool();
    /// let tool = tools.register(RegisterToolParams {
    ///     name: "code-formatter".to_string(),
    ///     version: "1.0.0".to_string(),
    ///     creator_did: "did:tenzro:human:abc123".to_string(),
    ///     description: "Formats source code in multiple languages".to_string(),
    ///     input_schema: serde_json::json!({"type": "object", "properties": {"code": {"type": "string"}, "language": {"type": "string"}}}),
    ///     output_schema: serde_json::json!({"type": "object", "properties": {"formatted_code": {"type": "string"}}}),
    ///     tags: vec!["code".to_string(), "formatting".to_string()],
    ///     pricing: None,
    ///     mcp_endpoint: None,
    /// }).await?;
    /// println!("Registered tool: {}", tool.tool_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register(&self, params: RegisterToolParams) -> SdkResult<ToolInfo> {
        self.rpc
            .call(
                "tenzro_registerTool",
                serde_json::json!([{
                    "name": params.name,
                    "version": params.version,
                    "creator_did": params.creator_did,
                    "description": params.description,
                    "input_schema": params.input_schema,
                    "output_schema": params.output_schema,
                    "tags": params.tags,
                    "pricing": params.pricing,
                    "mcp_endpoint": params.mcp_endpoint,
                }]),
            )
            .await
    }

    /// Lists tools from the registry with optional filtering
    ///
    /// # Arguments
    ///
    /// * `filter` - Optional filter parameters
    pub async fn list(&self, filter: Option<ToolFilter>) -> SdkResult<Vec<ToolInfo>> {
        let mut params = serde_json::Map::new();
        if let Some(f) = filter {
            if let Some(tag) = f.tag {
                params.insert("tag".to_string(), serde_json::json!(tag));
            }
            if let Some(creator) = f.creator_did {
                params.insert("creator_did".to_string(), serde_json::json!(creator));
            }
            if let Some(lim) = f.limit {
                params.insert("limit".to_string(), serde_json::json!(lim));
            }
            if let Some(off) = f.offset {
                params.insert("offset".to_string(), serde_json::json!(off));
            }
        }

        self.rpc
            .call(
                "tenzro_listTools",
                serde_json::json!([serde_json::Value::Object(params)]),
            )
            .await
    }

    /// Searches tools by free-text query
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let tools = client.tool();
    /// let results = tools.search("database migration").await?;
    /// for tool in &results {
    ///     println!("{}: {}", tool.name, tool.description);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(&self, query: &str) -> SdkResult<Vec<ToolInfo>> {
        self.rpc
            .call(
                "tenzro_searchTools",
                serde_json::json!([{ "query": query }]),
            )
            .await
    }

    /// Executes a tool with the given input
    ///
    /// # Arguments
    ///
    /// * `tool_id` - The tool to execute
    /// * `input` - Input data for the tool (must conform to the tool's input schema)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let tools = client.tool();
    /// let result = tools.use_tool(
    ///     "tool-abc123",
    ///     serde_json::json!({"code": "fn main() {}", "language": "rust"}),
    /// ).await?;
    /// println!("Result: {}", result.output);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn use_tool(
        &self,
        tool_id: &str,
        input: serde_json::Value,
    ) -> SdkResult<ToolExecutionResult> {
        self.rpc
            .call(
                "tenzro_useTool",
                serde_json::json!([{
                    "tool_id": tool_id,
                    "input": input,
                }]),
            )
            .await
    }

    /// Gets a tool by its ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let tools = client.tool();
    /// let tool = tools.get("tool-abc123").await?;
    /// println!("Tool: {} v{}", tool.name, tool.version);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, tool_id: &str) -> SdkResult<ToolInfo> {
        self.rpc
            .call(
                "tenzro_getTool",
                serde_json::json!([{ "tool_id": tool_id }]),
            )
            .await
    }

    /// Gets usage statistics for a tool
    ///
    /// # Arguments
    ///
    /// * `tool_id` - The tool to get usage stats for
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let tools = client.tool();
    /// let usage = tools.get_tool_usage("tool-abc123").await?;
    /// println!("Total invocations: {}", usage.total_invocations);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_tool_usage(&self, tool_id: &str) -> SdkResult<ToolUsage> {
        self.rpc
            .call(
                "tenzro_getToolUsage",
                serde_json::json!([{ "tool_id": tool_id }]),
            )
            .await
    }

    /// Updates an existing tool in the registry
    ///
    /// Only the tool creator can update a tool. All fields in `params`
    /// are optional; only provided fields are updated.
    ///
    /// # Arguments
    ///
    /// * `tool_id` - The tool to update
    /// * `params` - Update parameters (all fields optional)
    pub async fn update(
        &self,
        tool_id: &str,
        params: UpdateToolParams,
    ) -> SdkResult<ToolInfo> {
        self.rpc
            .call(
                "tenzro_updateTool",
                serde_json::json!([{
                    "tool_id": tool_id,
                    "name": params.name,
                    "version": params.version,
                    "description": params.description,
                    "input_schema": params.input_schema,
                    "output_schema": params.output_schema,
                    "tags": params.tags,
                    "pricing": params.pricing,
                    "mcp_endpoint": params.mcp_endpoint,
                }]),
            )
            .await
    }
}

/// Parameters for registering a new tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterToolParams {
    /// Tool name
    pub name: String,
    /// Semantic version (e.g., "1.0.0")
    pub version: String,
    /// DID of the tool creator
    pub creator_did: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema for the tool's input
    pub input_schema: serde_json::Value,
    /// JSON Schema for the tool's output
    pub output_schema: serde_json::Value,
    /// Discovery tags
    pub tags: Vec<String>,
    /// Optional pricing model
    pub pricing: Option<serde_json::Value>,
    /// Optional MCP endpoint URL for remote tool execution
    pub mcp_endpoint: Option<String>,
}

/// Filter parameters for listing tools
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolFilter {
    /// Filter by tag
    pub tag: Option<String>,
    /// Filter by creator DID
    pub creator_did: Option<String>,
    /// Limit number of results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Parameters for updating a tool
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateToolParams {
    /// New name (optional)
    pub name: Option<String>,
    /// New version (optional)
    pub version: Option<String>,
    /// New description (optional)
    pub description: Option<String>,
    /// New input schema (optional)
    pub input_schema: Option<serde_json::Value>,
    /// New output schema (optional)
    pub output_schema: Option<serde_json::Value>,
    /// New tags (optional)
    pub tags: Option<Vec<String>>,
    /// New pricing model (optional)
    pub pricing: Option<serde_json::Value>,
    /// New MCP endpoint URL (optional)
    pub mcp_endpoint: Option<String>,
}

/// Tool information from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Unique tool ID
    #[serde(default)]
    pub tool_id: String,
    /// Tool name
    #[serde(default)]
    pub name: String,
    /// Semantic version
    #[serde(default)]
    pub version: String,
    /// Creator DID
    #[serde(default)]
    pub creator_did: String,
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    /// Input JSON Schema
    #[serde(default)]
    pub input_schema: serde_json::Value,
    /// Output JSON Schema
    #[serde(default)]
    pub output_schema: serde_json::Value,
    /// Discovery tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Pricing model
    #[serde(default)]
    pub pricing: Option<serde_json::Value>,
    /// MCP endpoint URL (if remote)
    #[serde(default)]
    pub mcp_endpoint: Option<String>,
    /// Number of times this tool has been used
    #[serde(default)]
    pub usage_count: u64,
    /// Registration timestamp
    #[serde(default)]
    pub created_at: u64,
}

/// Usage statistics for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsage {
    /// Tool ID
    #[serde(default)]
    pub tool_id: String,
    /// Total number of invocations
    #[serde(default)]
    pub total_invocations: u64,
    /// Timestamp of the last usage (ISO 8601), if ever used
    #[serde(default)]
    pub last_used: Option<String>,
}

/// Result from executing a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    /// Execution ID
    #[serde(default)]
    pub execution_id: String,
    /// Tool ID that was executed
    #[serde(default)]
    pub tool_id: String,
    /// Output data from the tool
    #[serde(default)]
    pub output: serde_json::Value,
    /// Execution status
    #[serde(default)]
    pub status: String,
    /// Execution duration in milliseconds
    #[serde(default)]
    pub duration_ms: u64,
    /// Cost charged (in TNZO wei, as decimal string)
    #[serde(default)]
    pub cost: Option<String>,
}
