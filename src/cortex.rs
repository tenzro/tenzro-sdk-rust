//! Cortex SDK — recurrent-depth reasoning client for Tenzro Network.
//!
//! Cortex is the reasoning-worker tier on Tenzro Network. It treats *loop
//! depth* of recurrent-depth transformers (e.g. OpenMythos-style RDT models)
//! as a first-class, schedulable, billable primitive.
//!
//! This module wraps the four `tenzro_*Cortex*` JSON-RPC methods:
//! - `tenzro_cortexReason` (alias: `tenzro_cortexInference`)
//! - `tenzro_listCortexWorkers`
//! - `tenzro_listRemoteCortexWorkers`
//! - `tenzro_registerCortexWorker`
//!
//! # Example
//!
//! ```no_run
//! # use tenzro_sdk::{TenzroClient, config::SdkConfig};
//! # use tenzro_sdk::cortex::{ReasoningTier, AttestationRequirement};
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = SdkConfig::testnet();
//! # let client = TenzroClient::connect(config).await?;
//! let cortex = client.cortex();
//!
//! // Submit a Standard-tier reasoning request.
//! let resp = cortex
//!     .reason("openmythos-3b", "What is 2 + 2?", ReasoningTier::Standard)
//!     .await?;
//! println!("Response: {} ({} loops, {} TNZO)",
//!     String::from_utf8_lossy(&resp.output),
//!     resp.metadata.loops_used,
//!     resp.price_tnzo);
//! # Ok(())
//! # }
//! ```

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use crate::types::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Tiers + budget
// ---------------------------------------------------------------------------

/// Service tier for a Cortex reasoning request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ReasoningTier {
    /// Shallow reasoning, 2-4 loops, no attestation.
    Fast,
    /// Default tier, 8 loops, no attestation.
    #[default]
    Standard,
    /// Deep reasoning, 16-32 loops, optional TEE attestation.
    Deep,
    /// Institutional tier: deep loops + TEE attestation + on-chain receipt.
    Institutional,
}

impl ReasoningTier {
    /// Returns the lowercase tier label as expected by the RPC.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Standard => "standard",
            Self::Deep => "deep",
            Self::Institutional => "institutional",
        }
    }
}

/// Attestation requirement for a Cortex inference request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AttestationRequirement {
    /// No attestation required. Cheapest.
    #[default]
    None,
    /// Require a TEE quote (Intel TDX / AMD SEV-SNP / AWS Nitro / NVIDIA CC).
    Tee,
    /// Require TEE quote AND a Plonky3 STARK inference-verification proof.
    /// Expensive; reserved for Institutional tier.
    TeeAndZk,
}

impl AttestationRequirement {
    /// Returns the lowercase attestation label as expected by the RPC.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Tee => "tee",
            Self::TeeAndZk => "tee_and_zk",
        }
    }
}

// ---------------------------------------------------------------------------
// Reasoning-specific metadata captured during a Cortex inference.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CortexMetadata {
    /// Input tokens consumed.
    #[serde(default)]
    pub input_tokens: u32,
    /// Output tokens produced.
    #[serde(default)]
    pub output_tokens: u32,
    /// Number of recurrent loops actually executed.
    #[serde(default)]
    pub loops_used: u32,
    /// Wall-clock inference latency in milliseconds.
    #[serde(default)]
    pub latency_ms: u64,
    /// Model version string reported by the worker.
    #[serde(default)]
    pub model_version: Option<String>,
    /// Generation finish reason ("stop", "length", "converged").
    #[serde(default)]
    pub finish_reason: Option<String>,
    /// Count of distinct MoE experts activated, if known.
    #[serde(default)]
    pub experts_activated: Option<u32>,
}

// ---------------------------------------------------------------------------
// Receipt
// ---------------------------------------------------------------------------

/// Signed execution receipt for a Cortex reasoning call.
///
/// Every Cortex response carries a receipt. Deep / Institutional receipts
/// also anchor on Tenzro Ledger and may carry TEE quotes or Plonky3 STARK
/// inference-verification proofs.
///
/// Receipts let auditors reconstruct:
/// 1. Which model weights were used (`weights_hash`, `runtime_hash`).
/// 2. What input produced what output (`input_commitment`, `output_commitment`).
/// 3. How much compute was spent (`loops_used`, `tokens_in`, `tokens_out`).
/// 4. Who executed it (`worker_did`, `signature`).
/// 5. Optional hardware / ZK evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CortexReceipt {
    /// Model id that executed the request.
    pub model_id: String,
    /// Hex-encoded SHA-256 hash of the model weights.
    #[serde(default)]
    pub weights_hash: String,
    /// Hex-encoded SHA-256 hash of the serving runtime (container / binary).
    #[serde(default)]
    pub runtime_hash: String,
    /// Number of loops the worker was asked to run at most.
    pub loops_requested: u32,
    /// Number of loops actually run.
    pub loops_used: u32,
    /// Hex-encoded hash of the canonical request payload.
    #[serde(default)]
    pub input_commitment: String,
    /// Hex-encoded hash of the canonical response payload.
    #[serde(default)]
    pub output_commitment: String,
    /// DID of the worker that executed the inference.
    pub worker_did: String,
    /// Worker wallet address (for settlement).
    pub worker_address: Address,
    /// Optional TEE attestation quote bytes (base64-encoded over the wire).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tee_quote: Option<Vec<u8>>,
    /// Optional Plonky3 STARK inference-verification proof bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zk_proof: Option<Vec<u8>>,
    /// Input tokens charged.
    pub tokens_in: u32,
    /// Output tokens charged.
    pub tokens_out: u32,
    /// Final settled price in smallest TNZO unit.
    pub price_tnzo: u64,
    /// Receipt creation timestamp (microseconds since UNIX epoch).
    #[serde(default)]
    pub timestamp: serde_json::Value,
    /// Worker signature over the canonical receipt preimage.
    #[serde(default)]
    pub signature: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Request + response
// ---------------------------------------------------------------------------

/// A Cortex reasoning request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexRequest {
    /// Unique request id (server fills if omitted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Target model id.
    pub model_id: String,
    /// Plain-text prompt input.
    pub input: String,
    /// Reasoning tier hint.
    #[serde(default)]
    pub tier: Option<ReasoningTier>,
    /// Override min_loops (defaults from tier).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_loops: Option<u32>,
    /// Override max_loops (defaults from tier).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_loops: Option<u32>,
    /// Hard cap on billed cost in smallest TNZO unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_cost_tnzo: Option<u64>,
    /// Wall-clock deadline in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_ms: Option<u64>,
    /// Attestation requirement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation: Option<AttestationRequirement>,
    /// Caller address (also the payer); defaults to zero address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester: Option<Address>,
    /// Free-form generation parameters forwarded to the sidecar.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, String>,
}

/// A Cortex reasoning response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexResponse {
    /// Echo of the originating request id.
    #[serde(default)]
    pub request_id: String,
    /// Unique response id.
    #[serde(default)]
    pub response_id: String,
    /// Model that served the request.
    #[serde(default)]
    pub model_id: String,
    /// Worker address that executed the inference.
    #[serde(default)]
    pub worker: Address,
    /// Raw response payload.
    #[serde(default, with = "bytes_or_string")]
    pub output: Vec<u8>,
    /// Reasoning-specific metadata.
    #[serde(default)]
    pub metadata: CortexMetadata,
    /// Final price charged in smallest TNZO unit.
    #[serde(default)]
    pub price_tnzo: u64,
    /// Receipt binding this response to its inputs, weights, and worker.
    pub receipt: CortexReceipt,
    /// Whether on-chain settlement of `price_tnzo` succeeded on this node.
    #[serde(default)]
    pub settled: bool,
    /// Response creation timestamp (server-side).
    #[serde(default)]
    pub timestamp: serde_json::Value,
}

mod bytes_or_string {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(bytes)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        Ok(match v {
            serde_json::Value::String(s) => s.into_bytes(),
            serde_json::Value::Array(arr) => arr
                .into_iter()
                .filter_map(|x| x.as_u64().map(|n| n as u8))
                .collect(),
            _ => Vec::new(),
        })
    }
}

// ---------------------------------------------------------------------------
// Worker advertisements (for `tenzro_listCortexWorkers` / Remote)
// ---------------------------------------------------------------------------

/// A locally-registered Cortex worker entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexWorkerEntry {
    /// Model id this worker serves.
    pub model_id: String,
    /// DID of the worker.
    #[serde(default)]
    pub worker_did: String,
    /// Cortex model family metadata.
    #[serde(default)]
    pub family: serde_json::Value,
    /// Loop-aware pricing config.
    #[serde(default)]
    pub pricing: serde_json::Value,
}

/// A snapshot of all advertised Cortex workers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexWorkerList {
    #[serde(default)]
    pub workers: Vec<serde_json::Value>,
    #[serde(default)]
    pub count: usize,
}

// ---------------------------------------------------------------------------
// Pricing (loop-aware)
// ---------------------------------------------------------------------------

/// Loop-aware pricing configuration for a Cortex model.
///
/// Total cost formula (smallest TNZO unit):
///
/// ```text
/// cost = base_request_fee
///      + tokens_in  * price_per_input_token
///      + tokens_out * price_per_output_token
///      + loops_used * price_per_loop
///      + tee_premium  (if attestation includes Tee)
///      + zk_premium   (if attestation includes Zk)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CortexPricing {
    /// Flat fee per request.
    pub base_request_fee: u64,
    /// Fee per input token.
    pub price_per_input_token: u64,
    /// Fee per output token.
    pub price_per_output_token: u64,
    /// Fee per recurrent loop executed.
    pub price_per_loop: u64,
    /// Premium for TEE-attested execution.
    pub tee_premium: u64,
    /// Premium for Plonky3 STARK inference-verification proof.
    pub zk_premium: u64,
}

impl Default for CortexPricing {
    fn default() -> Self {
        Self {
            base_request_fee: 100,
            price_per_input_token: 10,
            price_per_output_token: 20,
            price_per_loop: 1_000,
            tee_premium: 10_000,
            zk_premium: 100_000,
        }
    }
}

impl CortexPricing {
    /// Compute total price for a settled inference (mirror of the on-node logic).
    pub fn compute(
        &self,
        tokens_in: u32,
        tokens_out: u32,
        loops_used: u32,
        attestation: AttestationRequirement,
    ) -> u64 {
        let mut cost = self.base_request_fee;
        cost = cost.saturating_add((tokens_in as u64).saturating_mul(self.price_per_input_token));
        cost = cost.saturating_add((tokens_out as u64).saturating_mul(self.price_per_output_token));
        cost = cost.saturating_add((loops_used as u64).saturating_mul(self.price_per_loop));
        match attestation {
            AttestationRequirement::None => {}
            AttestationRequirement::Tee => cost = cost.saturating_add(self.tee_premium),
            AttestationRequirement::TeeAndZk => {
                cost = cost.saturating_add(self.tee_premium);
                cost = cost.saturating_add(self.zk_premium);
            }
        }
        cost
    }
}

// ---------------------------------------------------------------------------
// Cortex client
// ---------------------------------------------------------------------------

/// Cortex client for recurrent-depth reasoning.
#[derive(Clone)]
pub struct CortexClient {
    rpc: Arc<RpcClient>,
}

impl CortexClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Submit a Cortex reasoning request via `tenzro_cortexReason`.
    ///
    /// This is the simplified one-shot helper. For full control over budget,
    /// attestation, params, etc., construct a [`CortexRequest`] and call
    /// [`Self::reason_with_request`].
    pub async fn reason(
        &self,
        model_id: &str,
        input: &str,
        tier: ReasoningTier,
    ) -> SdkResult<CortexResponse> {
        let params = serde_json::json!({
            "model_id": model_id,
            "input": input,
            "tier": tier.as_str(),
        });
        self.rpc.call("tenzro_cortexReason", params).await
    }

    /// Submit a Cortex reasoning request with full request parameters.
    pub async fn reason_with_request(&self, req: &CortexRequest) -> SdkResult<CortexResponse> {
        let mut params = serde_json::json!({
            "model_id": req.model_id,
            "input": req.input,
        });
        if let Some(t) = req.tier {
            params["tier"] = serde_json::Value::String(t.as_str().to_string());
        }
        if let Some(min) = req.min_loops {
            params["min_loops"] = serde_json::json!(min);
        }
        if let Some(max) = req.max_loops {
            params["max_loops"] = serde_json::json!(max);
        }
        if let Some(cap) = req.max_cost_tnzo {
            params["max_cost_tnzo"] = serde_json::json!(cap);
        }
        if let Some(deadline) = req.deadline_ms {
            params["deadline_ms"] = serde_json::json!(deadline);
        }
        if let Some(att) = req.attestation {
            params["attestation"] = serde_json::Value::String(att.as_str().to_string());
        }
        if let Some(ref requester) = req.requester {
            params["requester"] = serde_json::Value::String(requester.to_hex());
        }
        if !req.params.is_empty() {
            params["params"] = serde_json::to_value(&req.params)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        }
        if let Some(ref id) = req.request_id {
            params["request_id"] = serde_json::Value::String(id.clone());
        }
        self.rpc.call("tenzro_cortexReason", params).await
    }

    /// List Cortex workers registered locally on the connected node.
    pub async fn list_workers(&self) -> SdkResult<CortexWorkerList> {
        self.rpc
            .call("tenzro_listCortexWorkers", serde_json::json!([]))
            .await
    }

    /// List remote Cortex workers learned via the `tenzro/cortex`
    /// gossip topic (signed advertisements still within their TTL).
    pub async fn list_remote_workers(&self) -> SdkResult<CortexWorkerList> {
        self.rpc
            .call("tenzro_listRemoteCortexWorkers", serde_json::json!([]))
            .await
    }

    /// Register a Cortex worker on the connected node. The worker forwards
    /// Cortex requests to a sidecar HTTP backend.
    ///
    /// `family_arch` is the architecture identifier (e.g. "rdt-moe", "rdt-dense").
    /// `max_loops` is the hard ceiling on recurrent loops this model supports.
    pub async fn register_worker(
        &self,
        model_id: &str,
        sidecar_url: &str,
        family_arch: &str,
        max_loops: u32,
    ) -> SdkResult<serde_json::Value> {
        let params = serde_json::json!({
            "model_id": model_id,
            "sidecar_url": sidecar_url,
            "arch": family_arch,
            "max_loops": max_loops,
        });
        self.rpc
            .call("tenzro_registerCortexWorker", params)
            .await
    }

    /// Register a Cortex worker with an optional bearer token for the sidecar.
    pub async fn register_worker_with_auth(
        &self,
        model_id: &str,
        sidecar_url: &str,
        bearer_token: Option<&str>,
        family_arch: &str,
        max_loops: u32,
        pricing: Option<CortexPricing>,
        supported_tiers: Option<Vec<ReasoningTier>>,
    ) -> SdkResult<serde_json::Value> {
        let mut params = serde_json::json!({
            "model_id": model_id,
            "sidecar_url": sidecar_url,
            "arch": family_arch,
            "max_loops": max_loops,
        });
        if let Some(token) = bearer_token {
            params["bearer_token"] = serde_json::Value::String(token.to_string());
        }
        if let Some(p) = pricing {
            params["pricing"] = serde_json::to_value(p).unwrap_or(serde_json::Value::Null);
        }
        if let Some(tiers) = supported_tiers {
            let tier_strs: Vec<String> = tiers.iter().map(|t| t.as_str().to_string()).collect();
            params["supported_tiers"] = serde_json::to_value(tier_strs).unwrap_or(serde_json::Value::Null);
        }
        self.rpc
            .call("tenzro_registerCortexWorker", params)
            .await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_labels_match_rpc_wire_format() {
        assert_eq!(ReasoningTier::Fast.as_str(), "fast");
        assert_eq!(ReasoningTier::Standard.as_str(), "standard");
        assert_eq!(ReasoningTier::Deep.as_str(), "deep");
        assert_eq!(ReasoningTier::Institutional.as_str(), "institutional");
    }

    #[test]
    fn attestation_labels_match_rpc_wire_format() {
        assert_eq!(AttestationRequirement::None.as_str(), "none");
        assert_eq!(AttestationRequirement::Tee.as_str(), "tee");
        assert_eq!(AttestationRequirement::TeeAndZk.as_str(), "tee_and_zk");
    }

    #[test]
    fn pricing_accounts_for_loops_and_premiums() {
        let p = CortexPricing::default();
        let base = p.compute(10, 20, 8, AttestationRequirement::None);
        let tee = p.compute(10, 20, 8, AttestationRequirement::Tee);
        let full = p.compute(10, 20, 8, AttestationRequirement::TeeAndZk);
        assert!(tee > base);
        assert!(full > tee);
        // 8 loops × 1000 = 8_000 difference vs 0 loops.
        let no_loops = p.compute(10, 20, 0, AttestationRequirement::None);
        assert_eq!(base - no_loops, 8_000);
    }
}
