//! State-sync snapshot client for Tenzro Network.
//!
//! Wraps the five snapshot RPCs that drive state-sync between nodes:
//!
//! - `tenzro_listSnapshots` — enumerate local snapshots (summary only,
//!   per-chunk hashes elided).
//! - `tenzro_getSnapshotManifest` — fetch the full manifest for a
//!   snapshot at `height`, including per-chunk SHA-256 hashes used by
//!   clients to verify chunks before applying.
//! - `tenzro_getSnapshotChunk` — fetch one chunk by `(height, chunk_index)`,
//!   base64-encoded on the wire.
//! - `tenzro_offerSnapshot` — register an inbound manifest from a peer.
//!   **The caller MUST verify `state_root_hex` against a trusted QC at
//!   the same height before invoking** — this RPC just registers the
//!   offer and provisions the spool dir.
//! - `tenzro_applySnapshotChunk` — write one inbound chunk. The chunk's
//!   SHA-256 is verified against `manifest.chunk_hashes_hex[chunk_index]`
//!   before any disk write. On the last chunk, all chunks are decoded and
//!   atomically committed via `write_batch_sync`.
//!
//! # Example
//!
//! ```no_run
//! # use tenzro_sdk::TenzroClient;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = TenzroClient::new("https://rpc.tenzro.xyz").await?;
//! let snap = client.snapshot();
//!
//! let summaries = snap.list_snapshots().await?;
//! for s in &summaries.snapshots {
//!     println!("snapshot h={} chunks={}", s.height, s.num_chunks);
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// State-sync snapshot client.
#[derive(Clone)]
pub struct SnapshotClient {
    rpc: Arc<RpcClient>,
}

impl SnapshotClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Enumerate local snapshots. Per-chunk hashes are elided for
    /// compactness — use `get_snapshot_manifest(height)` to retrieve
    /// the full manifest.
    pub async fn list_snapshots(&self) -> SdkResult<SnapshotList> {
        self.rpc
            .call("tenzro_listSnapshots", serde_json::json!([]))
            .await
    }

    /// Fetch the full manifest for the snapshot at `height`, including
    /// per-chunk SHA-256 hashes. Returns the node's `-32004 no snapshot
    /// at height` error if no snapshot is taken at that height.
    pub async fn get_snapshot_manifest(&self, height: u64) -> SdkResult<SnapshotManifest> {
        self.rpc
            .call(
                "tenzro_getSnapshotManifest",
                serde_json::json!([{ "height": height }]),
            )
            .await
    }

    /// Fetch one chunk by `(height, chunk_index)`. The returned
    /// `data_b64` is the base64-encoded chunk bytes; verify against
    /// `manifest.chunk_hashes_hex[chunk_index]` before applying.
    pub async fn get_snapshot_chunk(
        &self,
        height: u64,
        chunk_index: u32,
    ) -> SdkResult<SnapshotChunk> {
        self.rpc
            .call(
                "tenzro_getSnapshotChunk",
                serde_json::json!([{
                    "height": height,
                    "chunk_index": chunk_index,
                }]),
            )
            .await
    }

    /// Register an inbound manifest from a peer.
    ///
    /// **Caller MUST verify `manifest.state_root_hex` against a trusted
    /// QC at the same height before invoking.** This RPC only registers
    /// the offer and provisions the spool directory; it does not itself
    /// validate the manifest against chain state.
    pub async fn offer_snapshot(
        &self,
        manifest: SnapshotManifest,
    ) -> SdkResult<SnapshotOfferAccepted> {
        self.rpc
            .call(
                "tenzro_offerSnapshot",
                serde_json::json!([manifest]),
            )
            .await
    }

    /// Write one inbound chunk. The chunk's SHA-256 is verified against
    /// `manifest.chunk_hashes_hex[chunk_index]` before any disk write.
    /// On the last chunk, all chunks are decoded and atomically
    /// committed via `write_batch_sync`; `complete` will be `true` on
    /// that final call.
    pub async fn apply_snapshot_chunk(
        &self,
        height: u64,
        chunk_index: u32,
        data_b64: impl Into<String>,
    ) -> SdkResult<SnapshotChunkApplied> {
        self.rpc
            .call(
                "tenzro_applySnapshotChunk",
                serde_json::json!([{
                    "height": height,
                    "chunk_index": chunk_index,
                    "data_b64": data_b64.into(),
                }]),
            )
            .await
    }
}

/// Compact summary returned by `tenzro_listSnapshots`. Per-chunk hashes
/// are elided here — fetch the full manifest with
/// `get_snapshot_manifest(height)` to verify chunks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSummary {
    #[serde(default)]
    pub height: u64,
    #[serde(default)]
    pub state_root_hex: String,
    #[serde(default)]
    pub num_chunks: u32,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub format: u32,
}

/// Result of `tenzro_listSnapshots`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotList {
    #[serde(default)]
    pub snapshots: Vec<SnapshotSummary>,
}

/// Full snapshot manifest with per-chunk SHA-256 hashes. Returned by
/// `tenzro_getSnapshotManifest` and consumed by `tenzro_offerSnapshot`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    /// Block height at which this snapshot was taken.
    #[serde(default)]
    pub height: u64,
    /// State root committed at `height`. The caller MUST verify this
    /// against a trusted QC at the same height before offering or
    /// applying this snapshot.
    #[serde(default)]
    pub state_root_hex: String,
    /// Number of chunks. Chunk indices are `0..num_chunks`.
    #[serde(default)]
    pub num_chunks: u32,
    /// Per-chunk SHA-256 hash (hex), indexed by chunk number. Used by
    /// the receiver to verify chunks before disk write.
    #[serde(default)]
    pub chunk_hashes_hex: Vec<String>,
    /// Wall-clock time the snapshot was produced (ISO 8601, UTC).
    #[serde(default)]
    pub created_at: String,
    /// Manifest format version. Bumps on incompatible chunk-encoding
    /// changes.
    #[serde(default)]
    pub format: u32,
}

/// Result of `tenzro_getSnapshotChunk`. `data_b64` is the base64-encoded
/// chunk bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotChunk {
    #[serde(default)]
    pub height: u64,
    #[serde(default)]
    pub chunk_index: u32,
    #[serde(default)]
    pub data_b64: String,
}

/// Result of `tenzro_offerSnapshot`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotOfferAccepted {
    #[serde(default)]
    pub accepted: bool,
    #[serde(default)]
    pub height: u64,
    #[serde(default)]
    pub num_chunks: u32,
}

/// Result of `tenzro_applySnapshotChunk`. `complete` flips to `true` on
/// the final chunk, after which the snapshot has been atomically
/// committed via `write_batch_sync`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotChunkApplied {
    #[serde(default)]
    pub complete: bool,
    #[serde(default)]
    pub height: u64,
    #[serde(default)]
    pub chunk_index: u32,
}
