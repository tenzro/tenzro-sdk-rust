//! Tenzro Train read-side inspection client
//!
//! Wraps the `tenzro_training_listRuns` / `tenzro_training_getRun` /
//! `tenzro_training_getReceipt` / `tenzro_training_getSealedManifest`
//! RPCs — the operator / dashboard / analytics surface for inspecting
//! in-flight training runs, fetching sealed receipts, and auditing
//! Confidential-tier sealed-shard manifests.
//!
//! Write-side endpoints (`postTask`, `enrollTrainer`,
//! `submitOuterGradient`, `finalizeRound`, `installSealedManifest`)
//! live in a future write-side client — this read client is safe to
//! expose to monitoring agents that should never mutate state.
//!
//! # Example
//!
//! ```no_run
//! # use tenzro_sdk::TenzroClient;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = TenzroClient::new("https://rpc.tenzro.network").await?;
//! let runs = client.training_inspection().list_runs().await?;
//! for run in runs.runs {
//!     println!("{}: round={} status={:?}", run.task_id, run.current_round, run.status);
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Lifecycle status of a `TrainingRun`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingRunStatus {
    Pending,
    Active,
    Completed,
    Failed,
    Cancelled,
}

/// Read-side view of a `TrainingRun`. Opaque sub-payloads (current
/// `SyncRound`, aggregator config, attestations) are passed through
/// as `serde_json::Value` so this SDK doesn't lock callers to a
/// specific decode shape — consumers that want strongly-typed access
/// can deserialize via `tenzro-types::training` directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRun {
    #[serde(default)]
    pub task_id: String,
    #[serde(default)]
    pub task_spec: serde_json::Value,
    pub status: TrainingRunStatus,
    #[serde(default)]
    pub current_round: u64,
    #[serde(default)]
    pub state_root: String,
    #[serde(default)]
    pub enrolled_trainers: Vec<String>,
    #[serde(default)]
    pub created_at_ms: u64,
    #[serde(default)]
    pub updated_at_ms: u64,
    /// Present once `status` reaches `Completed` / `Failed` / `Cancelled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<serde_json::Value>,
    /// Aggregator parameters, decay schedule, witness committee config.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Sealed training receipt — produced when a run reaches its terminal
/// state. Persisted under `CF_TRAINING_RECEIPTS`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingReceipt {
    #[serde(default)]
    pub task_id: String,
    #[serde(default)]
    pub final_state_root: String,
    #[serde(default)]
    pub rounds_completed: u64,
    #[serde(default)]
    pub witness_committees: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_hash: Option<String>,
    #[serde(default)]
    pub sealed_at_ms: u64,
    /// Trailing fields not yet schemaized — preserved for forward-compat.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Confidential-tier sealed-shard manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedDatasetManifest {
    #[serde(default)]
    pub task_id: String,
    #[serde(default)]
    pub manifest_hash: String,
    #[serde(default)]
    pub envelopes: Vec<SealedShardEnvelope>,
    #[serde(default)]
    pub created_at_ms: u64,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// One trainer's encrypted shard plus HPKE-wrapped data key and the
/// enclave attestation the sponsor sealed to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedShardEnvelope {
    #[serde(default)]
    pub trainer_did: String,
    #[serde(default)]
    pub shard_index: u64,
    #[serde(default)]
    pub shard_ciphertext_hash: String,
    #[serde(default)]
    pub shard_ciphertext_bytes: u64,
    #[serde(default)]
    pub wrapped_data_key: String,
    /// Always `"hpke-x25519-hkdf-sha256-aes-256-gcm"`.
    #[serde(default)]
    pub wrap_alg: String,
    #[serde(default)]
    pub enclave_pubkey: String,
    #[serde(default)]
    pub enclave_measurements_hex: String,
    #[serde(default)]
    pub created_at_ms: u64,
}

/// Result of `tenzro_training_listRuns`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTrainingRunsResult {
    #[serde(default)]
    pub runs: Vec<TrainingRun>,
}

/// Read-only Tenzro Train inspection client.
#[derive(Clone)]
pub struct TrainingInspectionClient {
    rpc: Arc<RpcClient>,
}

impl TrainingInspectionClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// List every active training run this node is syncing.
    pub async fn list_runs(&self) -> SdkResult<ListTrainingRunsResult> {
        self.rpc
            .call("tenzro_training_listRuns", serde_json::json!({}))
            .await
    }

    /// Look up a single run by `task_id`. The node returns JSON-RPC
    /// `-32602` if the run is unknown — surfaces as `SdkError::RpcError`.
    pub async fn get_run(&self, task_id: &str) -> SdkResult<TrainingRun> {
        self.rpc
            .call(
                "tenzro_training_getRun",
                serde_json::json!({ "task_id": task_id }),
            )
            .await
    }

    /// Fetch the sealed receipt for a finalized run. Returns `None`
    /// when the run is still active.
    pub async fn get_receipt(
        &self,
        task_id: &str,
    ) -> SdkResult<Option<TrainingReceipt>> {
        self.rpc
            .call(
                "tenzro_training_getReceipt",
                serde_json::json!({ "task_id": task_id }),
            )
            .await
    }

    /// Fetch the installed sealed-shard manifest for a Confidential-tier
    /// task. Returns `None` when no manifest has been installed (Open
    /// and Verified tiers never have a manifest).
    pub async fn get_sealed_manifest(
        &self,
        task_id: &str,
    ) -> SdkResult<Option<SealedDatasetManifest>> {
        self.rpc
            .call(
                "tenzro_training_getSealedManifest",
                serde_json::json!({ "task_id": task_id }),
            )
            .await
    }
}
