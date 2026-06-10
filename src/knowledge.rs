//! Knowledge registry SDK.
//!
//! Operator-curated queryable data resources: vector DBs, RAG
//! indices, document corpora, indexed datasets, live data feeds,
//! embedding stores. Mirrors the tools registry shape, settled in
//! TNZO with the standard 5% commission split.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct KnowledgeClient {
    rpc: Arc<RpcClient>,
}

impl KnowledgeClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    pub async fn register(&self, params: RegisterKnowledgeParams) -> SdkResult<KnowledgeInfo> {
        let v = serde_json::to_value(&params).map_err(|_| {
            crate::error::SdkError::SerializationError
        })?;
        self.rpc.call("tenzro_registerKnowledge", v).await
    }

    pub async fn list(
        &self,
        filter: Option<KnowledgeFilter>,
    ) -> SdkResult<Vec<KnowledgeInfo>> {
        let v = serde_json::to_value(&filter.unwrap_or_default()).map_err(|_| {
            crate::error::SdkError::SerializationError
        })?;
        self.rpc.call("tenzro_listKnowledge", v).await
    }

    pub async fn search(
        &self,
        filter: KnowledgeFilter,
    ) -> SdkResult<Vec<KnowledgeInfo>> {
        let v = serde_json::to_value(&filter).map_err(|_| {
            crate::error::SdkError::SerializationError
        })?;
        self.rpc.call("tenzro_searchKnowledge", v).await
    }

    pub async fn get(&self, knowledge_id: &str) -> SdkResult<KnowledgeInfo> {
        self.rpc
            .call(
                "tenzro_getKnowledge",
                serde_json::json!({"knowledge_id": knowledge_id}),
            )
            .await
    }

    pub async fn use_resource(
        &self,
        params: UseKnowledgeParams,
    ) -> SdkResult<KnowledgeInvocationResult> {
        let v = serde_json::to_value(&params).map_err(|_| {
            crate::error::SdkError::SerializationError
        })?;
        self.rpc.call("tenzro_useKnowledge", v).await
    }
}

/// Kind of knowledge resource. Wire form is snake_case.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeKind {
    VectorIndex,
    DocumentCorpus,
    IndexedDataset,
    Feed,
    EmbeddingStore,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterKnowledgeParams {
    pub name: String,
    pub version: String,
    pub kind: KnowledgeKind,
    pub endpoint: String,
    pub description: String,
    pub category: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_did: Option<String>,
    /// Required when `price_per_call > 0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_wallet: Option<String>,
    /// atto-TNZO as decimal string for u128 safety. `0` = free.
    #[serde(default = "default_zero")]
    pub price_per_call: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params_schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backing_tool_id: Option<String>,
    /// Optional subject-level access list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_to_subjects: Option<Vec<String>>,
}

fn default_zero() -> String {
    "0".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KnowledgeFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_did: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeInfo {
    pub knowledge_id: String,
    pub name: String,
    pub version: String,
    pub kind: String,
    pub endpoint: String,
    pub description: String,
    pub category: String,
    pub capabilities: Vec<String>,
    pub creator_did: Option<String>,
    pub creator_wallet: Option<String>,
    pub price_per_call: String,
    pub status: String,
    pub created_at: u64,
    pub invocation_count: u64,
    pub last_seen_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseKnowledgeParams {
    pub knowledge_id: String,
    pub params: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer_wallet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeInvocationResult {
    pub knowledge_id: String,
    pub invocation_id: String,
    pub output: serde_json::Value,
    pub amount_paid: String,
    pub completed_at: u64,
}
