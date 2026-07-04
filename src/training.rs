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

/// Result of `tenzro_getTrainerDaemonStatus`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainerDaemonStatus {
    /// `true` when the node runs the trainer auto-provisioning daemon.
    pub running: bool,
    /// DID the daemon enrolls trainers under. Absent when not running.
    #[serde(default)]
    pub trainer_did: Option<String>,
    /// Number of inner-loop trainer processes currently live.
    #[serde(default)]
    pub live_trainers: u64,
    /// Concurrency ceiling from the `[training]` config. Absent when not running.
    #[serde(default)]
    pub max_concurrent_trainers: Option<u64>,
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

    /// Report the trainer auto-provisioning daemon status. When the node
    /// has no `[training]` section (or `enabled = false`), `running` is
    /// `false` and the DID / concurrency fields are absent.
    pub async fn daemon_status(&self) -> SdkResult<TrainerDaemonStatus> {
        self.rpc
            .call("tenzro_getTrainerDaemonStatus", serde_json::json!({}))
            .await
    }
}

/// Optional Confidential-tier enrollment payload — required when the
/// task being enrolled into has a sealed-shard manifest installed.
/// The attestation proves the trainer is running inside a TEE enclave
/// whose pubkey + measurements were sealed into the manifest by the
/// task sponsor.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfidentialEnrollment {
    pub attestation: String,
    pub enclave_pubkey: String,
    pub measurements_hex: String,
}

/// Write-side client for the Tenzro Train protocol. Backed by the
/// `tenzro_training_postTask` / `enrollTrainer` / `submitOuterGradient`
/// / `finalizeRound` / `installSealedManifest` RPCs.
#[derive(Clone)]
pub struct TrainingClient {
    rpc: std::sync::Arc<crate::rpc::RpcClient>,
}

impl TrainingClient {
    pub(crate) fn new(rpc: std::sync::Arc<crate::rpc::RpcClient>) -> Self {
        Self { rpc }
    }

    /// Register a new training run with the local syncer.
    pub async fn post_task(
        &self,
        task_spec: serde_json::Value,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_training_postTask",
                serde_json::json!({ "task_spec": task_spec }),
            )
            .await
    }

    /// Enroll a trainer DID into an active training run. For
    /// Confidential-tier tasks, pass `confidential` carrying the
    /// trainer's TEE attestation + enclave pubkey + measurements.
    pub async fn enroll_trainer(
        &self,
        task_id: &str,
        trainer_did: &str,
        confidential: Option<&ConfidentialEnrollment>,
    ) -> SdkResult<serde_json::Value> {
        let mut params = serde_json::Map::new();
        params.insert("task_id".into(), serde_json::Value::String(task_id.to_string()));
        params.insert(
            "trainer_did".into(),
            serde_json::Value::String(trainer_did.to_string()),
        );
        if let Some(c) = confidential {
            params.insert(
                "attestation".into(),
                serde_json::Value::String(c.attestation.clone()),
            );
            params.insert(
                "enclave_pubkey".into(),
                serde_json::Value::String(c.enclave_pubkey.clone()),
            );
            params.insert(
                "measurements_hex".into(),
                serde_json::Value::String(c.measurements_hex.clone()),
            );
        }
        self.rpc
            .call("tenzro_training_enrollTrainer", serde_json::Value::Object(params))
            .await
    }

    /// Submit an outer gradient for the current round.
    pub async fn submit_outer_gradient(
        &self,
        task_id: &str,
        gradient: serde_json::Value,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_training_submitOuterGradient",
                serde_json::json!({ "task_id": task_id, "gradient": gradient }),
            )
            .await
    }

    /// Finalize the current round. Idempotent under the k-of-N witness
    /// committee model: redundant submissions for the same
    /// `(round, state_root)` return Ok; conflicting state roots return
    /// `ConflictingFinalize` for fork detection.
    pub async fn finalize_round(
        &self,
        task_id: &str,
        sync_round: serde_json::Value,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_training_finalizeRound",
                serde_json::json!({ "task_id": task_id, "sync_round": sync_round }),
            )
            .await
    }

    /// Install a sealed-shard manifest for a Confidential-tier task.
    pub async fn install_sealed_manifest(
        &self,
        task_id: &str,
        manifest: &SealedDatasetManifest,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_training_installSealedManifest",
                serde_json::json!({ "task_id": task_id, "manifest": manifest }),
            )
            .await
    }

    /// Ask the syncer whether the current round should finalize, keep waiting,
    /// or advance on a no-endorsement certificate. The decision is driven by
    /// the DiLoCo grace window: `wait` reports the remaining milliseconds,
    /// `finalize` and `no_quorum` report the round number. Returns the raw
    /// `{ decision, .. }` object.
    pub async fn decide_round(&self, task_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_training_decideRound",
                serde_json::json!({ "task_id": task_id }),
            )
            .await
    }
}
