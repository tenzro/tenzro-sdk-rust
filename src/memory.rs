//! Per-agent memory tier client.
//!
//! Wraps the `tenzro_memory*` RPC namespace: Lance vector kNN + Tantivy
//! BM25 hybrid search with Reciprocal Rank Fusion (k=60). Records can
//! be archived to the node's DA backend; archived records keep the
//! original metadata but replace the payload with a `DaPointer`.
//!
//! ## Auth
//!
//! Every memory RPC requires DPoP+JWT bearer auth. Set
//! `TENZRO_BEARER_JWT` and `TENZRO_DPOP_PROOF` in the process
//! environment before calling these methods. The server matches the
//! bearer's DID against the requested `agent_did` (or its
//! `controller_did` for delegated agents) and rejects cross-agent
//! reads with JSON-RPC `-32001`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::rpc::RpcClient;
use crate::SdkResult;

/// Kind of memory record.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Granted,
    Recalled,
    SelfNoted,
    Archived,
}

impl MemoryKind {
    fn as_wire(&self) -> &'static str {
        match self {
            MemoryKind::Granted => "granted",
            MemoryKind::Recalled => "recalled",
            MemoryKind::SelfNoted => "self_noted",
            MemoryKind::Archived => "archived",
        }
    }
}

/// Where the memory record originated.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    Controller,
    Tool,
    Peer,
    #[serde(rename = "self")]
    SelfSrc,
}

impl MemorySource {
    fn as_wire(&self) -> &'static str {
        match self {
            MemorySource::Controller => "controller",
            MemorySource::Tool => "tool",
            MemorySource::Peer => "peer",
            MemorySource::SelfSrc => "self",
        }
    }
}

/// Search mode for [`MemoryClient::recall`]. `Hybrid` is the production
/// default and merges Lance vector kNN with Tantivy BM25 via Reciprocal
/// Rank Fusion at k=60.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemorySearchMode {
    Hybrid,
    Vector,
    Text,
}

impl MemorySearchMode {
    fn as_wire(&self) -> &'static str {
        match self {
            MemorySearchMode::Hybrid => "hybrid",
            MemorySearchMode::Vector => "vector",
            MemorySearchMode::Text => "text",
        }
    }
}

/// Pointer to a memory record offloaded to a DA backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaPointer {
    pub backend: String,
    pub namespace: String,
    pub locator: String,
    pub commitment_kzg: Option<String>,
    pub attestation_root: Option<String>,
}

/// A single memory record returned by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: String,
    pub agent_did: String,
    pub created_at_ms: i64,
    pub kind: String,
    pub source: String,
    pub text: String,
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub da_pointer: Option<DaPointer>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecallResult {
    pub count: u64,
    pub records: Vec<MemoryRecord>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListMemoryResult {
    pub count: u64,
    pub records: Vec<MemoryRecord>,
}

/// Per-agent memory tier client. Construct via
/// [`crate::TenzroClient::memory`] (no direct constructor — the client
/// is always bound to a parent `TenzroClient`'s `RpcClient`).
#[derive(Clone)]
pub struct MemoryClient {
    rpc: Arc<RpcClient>,
}

impl MemoryClient {
    pub fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Persist a memory for the agent identified by `agent_did`.
    pub async fn grant(
        &self,
        agent_did: impl Into<String>,
        text: impl Into<String>,
        kind: Option<MemoryKind>,
        source: Option<MemorySource>,
        metadata: Option<serde_json::Value>,
    ) -> SdkResult<MemoryRecord> {
        let params = serde_json::json!({
            "agent_did": agent_did.into(),
            "text": text.into(),
            "kind": kind.unwrap_or(MemoryKind::Granted).as_wire(),
            "source": source.unwrap_or(MemorySource::Controller).as_wire(),
            "metadata": metadata.unwrap_or_else(|| serde_json::json!({})),
        });
        self.rpc.call("tenzro_memoryGrant", params).await
    }

    /// Recall up to `k` memories matching `query`. `mode` defaults to
    /// `Hybrid` when `None`.
    pub async fn recall(
        &self,
        agent_did: impl Into<String>,
        query: impl Into<String>,
        k: Option<usize>,
        mode: Option<MemorySearchMode>,
    ) -> SdkResult<RecallResult> {
        let params = serde_json::json!({
            "agent_did": agent_did.into(),
            "query": query.into(),
            "k": k.unwrap_or(10),
            "mode": mode.unwrap_or(MemorySearchMode::Hybrid).as_wire(),
        });
        self.rpc.call("tenzro_memoryRecall", params).await
    }

    /// Archive a record to the DA backend.
    pub async fn archive(
        &self,
        record_id: impl Into<String>,
        agent_did: impl Into<String>,
    ) -> SdkResult<MemoryRecord> {
        let params = serde_json::json!({
            "record_id": record_id.into(),
            "agent_did": agent_did.into(),
        });
        self.rpc.call("tenzro_memoryArchive", params).await
    }

    /// List newest-first memories for an agent. `limit` defaults to 50.
    pub async fn list_records(
        &self,
        agent_did: impl Into<String>,
        limit: Option<usize>,
    ) -> SdkResult<ListMemoryResult> {
        let params = serde_json::json!({
            "agent_did": agent_did.into(),
            "limit": limit.unwrap_or(50),
        });
        self.rpc.call("tenzro_listMemoryRecords", params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_kind_wire_matches_server() {
        assert_eq!(MemoryKind::Granted.as_wire(), "granted");
        assert_eq!(MemoryKind::Recalled.as_wire(), "recalled");
        assert_eq!(MemoryKind::SelfNoted.as_wire(), "self_noted");
        assert_eq!(MemoryKind::Archived.as_wire(), "archived");
    }

    #[test]
    fn memory_source_wire_matches_server() {
        assert_eq!(MemorySource::Controller.as_wire(), "controller");
        assert_eq!(MemorySource::Tool.as_wire(), "tool");
        assert_eq!(MemorySource::Peer.as_wire(), "peer");
        assert_eq!(MemorySource::SelfSrc.as_wire(), "self");
    }

    #[test]
    fn memory_search_mode_wire_matches_server() {
        assert_eq!(MemorySearchMode::Hybrid.as_wire(), "hybrid");
        assert_eq!(MemorySearchMode::Vector.as_wire(), "vector");
        assert_eq!(MemorySearchMode::Text.as_wire(), "text");
    }

    #[test]
    fn memory_record_deserializes_real_response() {
        let body = serde_json::json!({
            "id": "rec-1",
            "agent_did": "did:tenzro:machine:abc",
            "created_at_ms": 1_700_000_000_000_i64,
            "kind": "granted",
            "source": "controller",
            "text": "hello",
            "metadata": { "tag": "test" },
            "da_pointer": null,
        });
        let rec: MemoryRecord = serde_json::from_value(body).unwrap();
        assert_eq!(rec.id, "rec-1");
        assert_eq!(rec.kind, "granted");
        assert!(rec.da_pointer.is_none());
    }

    #[test]
    fn memory_record_with_da_pointer_deserializes() {
        let body = serde_json::json!({
            "id": "rec-2",
            "agent_did": "did:tenzro:machine:abc",
            "created_at_ms": 1_700_000_000_000_i64,
            "kind": "archived",
            "source": "self",
            "text": "stored to DA",
            "metadata": {},
            "da_pointer": {
                "backend": "iroh-blobs",
                "namespace": "agent-memory",
                "locator": "0xdeadbeef",
                "commitment_kzg": null,
                "attestation_root": null,
            },
        });
        let rec: MemoryRecord = serde_json::from_value(body).unwrap();
        assert!(rec.da_pointer.is_some());
        let p = rec.da_pointer.unwrap();
        assert_eq!(p.backend, "iroh-blobs");
        assert_eq!(p.locator, "0xdeadbeef");
    }

    // Hint to the compiler that SdkResult is used downstream.
    #[allow(dead_code)]
    fn _typecheck_result_path() -> SdkResult<()> {
        Ok(())
    }
}
