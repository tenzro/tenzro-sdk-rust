//! Skills Registry SDK for Tenzro Network
//!
//! This module provides skill registration, discovery, and execution
//! functionality for the Tenzro Skills Registry.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Skills client for Skills Registry operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let skills = client.skill();
///
/// // Search for skills
/// let results = skills.search("blockchain analysis").await?;
/// println!("Found {} skills", results.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct SkillClient {
    rpc: Arc<RpcClient>,
}

impl SkillClient {
    /// Creates a new skill client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Registers a new skill in the Skills Registry
    ///
    /// # Arguments
    ///
    /// * `params` - Skill registration parameters
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # use tenzro_sdk::skill::RegisterSkillParams;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let skills = client.skill();
    /// let skill = skills.register(RegisterSkillParams {
    ///     name: "sentiment-analysis".to_string(),
    ///     version: "1.0.0".to_string(),
    ///     creator_did: "did:tenzro:human:abc123".to_string(),
    ///     description: "Analyzes text sentiment using NLP".to_string(),
    ///     input_schema: serde_json::json!({"type": "object", "properties": {"text": {"type": "string"}}}),
    ///     output_schema: serde_json::json!({"type": "object", "properties": {"sentiment": {"type": "string"}}}),
    ///     tags: vec!["nlp".to_string(), "sentiment".to_string()],
    ///     pricing: None,
    /// }).await?;
    /// println!("Registered skill: {}", skill.skill_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register(&self, params: RegisterSkillParams) -> SdkResult<SkillInfo> {
        self.rpc
            .call(
                "tenzro_registerSkill",
                serde_json::json!([{
                    "name": params.name,
                    "version": params.version,
                    "creator_did": params.creator_did,
                    "description": params.description,
                    "input_schema": params.input_schema,
                    "output_schema": params.output_schema,
                    "tags": params.tags,
                    "pricing": params.pricing,
                }]),
            )
            .await
    }

    /// Lists skills from the registry with optional filtering
    ///
    /// # Arguments
    ///
    /// * `filter` - Optional filter parameters
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # use tenzro_sdk::skill::SkillFilter;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let skills = client.skill();
    /// let list = skills.list(Some(SkillFilter {
    ///     tag: Some("nlp".to_string()),
    ///     creator_did: None,
    ///     limit: Some(20),
    ///     offset: None,
    /// })).await?;
    /// println!("Found {} skills", list.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self, filter: Option<SkillFilter>) -> SdkResult<Vec<SkillInfo>> {
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
                "tenzro_listSkills",
                serde_json::json!([serde_json::Value::Object(params)]),
            )
            .await
    }

    /// Searches skills by free-text query
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
    /// let skills = client.skill();
    /// let results = skills.search("image classification").await?;
    /// for skill in &results {
    ///     println!("{}: {}", skill.name, skill.description);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(&self, query: &str) -> SdkResult<Vec<SkillInfo>> {
        self.rpc
            .call(
                "tenzro_searchSkills",
                serde_json::json!([{ "query": query }]),
            )
            .await
    }

    /// Executes a skill with the given input
    ///
    /// # Arguments
    ///
    /// * `skill_id` - The skill to execute
    /// * `input` - Input data for the skill (must conform to the skill's input schema)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let skills = client.skill();
    /// let result = skills.use_skill(
    ///     "skill-abc123",
    ///     serde_json::json!({"text": "This product is excellent!"}),
    /// ).await?;
    /// println!("Result: {}", result.output);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn use_skill(
        &self,
        skill_id: &str,
        input: serde_json::Value,
    ) -> SdkResult<SkillExecutionResult> {
        self.rpc
            .call(
                "tenzro_useSkill",
                serde_json::json!([{
                    "skill_id": skill_id,
                    "input": input,
                }]),
            )
            .await
    }

    /// Gets a skill by its ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let skills = client.skill();
    /// let skill = skills.get("skill-abc123").await?;
    /// println!("Skill: {} v{}", skill.name, skill.version);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, skill_id: &str) -> SdkResult<SkillInfo> {
        self.rpc
            .call(
                "tenzro_getSkill",
                serde_json::json!([{ "skill_id": skill_id }]),
            )
            .await
    }

    /// Gets usage statistics for a skill
    ///
    /// # Arguments
    ///
    /// * `skill_id` - The skill to get usage stats for
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let skills = client.skill();
    /// let usage = skills.get_skill_usage("skill-abc123").await?;
    /// println!("Total invocations: {}", usage.total_invocations);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_skill_usage(&self, skill_id: &str) -> SdkResult<SkillUsage> {
        self.rpc
            .call(
                "tenzro_getSkillUsage",
                serde_json::json!([{ "skill_id": skill_id }]),
            )
            .await
    }

    /// Updates an existing skill in the registry
    ///
    /// Only the skill creator can update a skill. All fields in `params`
    /// are optional; only provided fields are updated.
    ///
    /// # Arguments
    ///
    /// * `skill_id` - The skill to update
    /// * `params` - Update parameters (all fields optional)
    pub async fn update(
        &self,
        skill_id: &str,
        params: UpdateSkillParams,
    ) -> SdkResult<SkillInfo> {
        self.rpc
            .call(
                "tenzro_updateSkill",
                serde_json::json!([{
                    "skill_id": skill_id,
                    "name": params.name,
                    "version": params.version,
                    "description": params.description,
                    "input_schema": params.input_schema,
                    "output_schema": params.output_schema,
                    "tags": params.tags,
                    "pricing": params.pricing,
                }]),
            )
            .await
    }
}

/// Parameters for registering a new skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSkillParams {
    /// Skill name
    pub name: String,
    /// Semantic version (e.g., "1.0.0")
    pub version: String,
    /// DID of the skill creator
    pub creator_did: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema for the skill's input
    pub input_schema: serde_json::Value,
    /// JSON Schema for the skill's output
    pub output_schema: serde_json::Value,
    /// Discovery tags
    pub tags: Vec<String>,
    /// Optional pricing model
    pub pricing: Option<serde_json::Value>,
}

/// Filter parameters for listing skills
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillFilter {
    /// Filter by tag
    pub tag: Option<String>,
    /// Filter by creator DID
    pub creator_did: Option<String>,
    /// Limit number of results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Parameters for updating a skill
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateSkillParams {
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
}

/// Skill information from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    /// Unique skill ID
    #[serde(default)]
    pub skill_id: String,
    /// Skill name
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
    /// Number of times this skill has been used
    #[serde(default)]
    pub usage_count: u64,
    /// Registration timestamp
    #[serde(default)]
    pub created_at: u64,
}

/// Usage statistics for a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUsage {
    /// Skill ID
    #[serde(default)]
    pub skill_id: String,
    /// Total number of invocations
    #[serde(default)]
    pub total_invocations: u64,
    /// Timestamp of the last usage (ISO 8601), if ever used
    #[serde(default)]
    pub last_used: Option<String>,
}

/// Result from executing a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionResult {
    /// Execution ID
    #[serde(default)]
    pub execution_id: String,
    /// Skill ID that was executed
    #[serde(default)]
    pub skill_id: String,
    /// Output data from the skill
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
