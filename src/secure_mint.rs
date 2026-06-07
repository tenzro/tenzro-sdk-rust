//! Secure-Mint registry client.
//!
//! Enforces a per-token 1:1 reserve-attestation invariant for
//! tokenized real-world assets (xStocks-class equities, treasuries,
//! stablecoins): `circulating + amount ≤ reserve` at every mint, with
//! `reserve` updated through a `SubmitReserveAttestation` flow gated
//! on a Proof-of-Reserve feed + attester DID.
//!
//! Wraps the node-side RPCs:
//! - `tenzro_setSecureMintPolicy`, `tenzro_getSecureMintPolicy`,
//!   `tenzro_clearSecureMintPolicy`
//! - `tenzro_secureMintCheck` (read-only invariant check),
//!   `tenzro_secureMintApply` (atomic check + circulating increment),
//!   `tenzro_secureMintRecordBurn` (decrement on redemption).

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct SecureMintClient {
    rpc: Arc<RpcClient>,
}

impl SecureMintClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    pub async fn set_policy(&self, policy: SecureMintPolicy) -> SdkResult<SecureMintPolicy> {
        self.rpc
            .call("tenzro_setSecureMintPolicy", serde_json::json!([policy]))
            .await
    }

    pub async fn get_policy(&self, asset_id: &str) -> SdkResult<Option<SecureMintPolicy>> {
        self.rpc
            .call(
                "tenzro_getSecureMintPolicy",
                serde_json::json!([{ "asset_id": asset_id }]),
            )
            .await
    }

    pub async fn clear_policy(&self, asset_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_clearSecureMintPolicy",
                serde_json::json!([{ "asset_id": asset_id }]),
            )
            .await
    }

    pub async fn check(&self, asset_id: &str, amount: &str) -> SdkResult<SecureMintCheck> {
        self.rpc
            .call(
                "tenzro_secureMintCheck",
                serde_json::json!([{ "asset_id": asset_id, "amount": amount }]),
            )
            .await
    }

    pub async fn apply(&self, asset_id: &str, amount: &str) -> SdkResult<SecureMintApply> {
        self.rpc
            .call(
                "tenzro_secureMintApply",
                serde_json::json!([{ "asset_id": asset_id, "amount": amount }]),
            )
            .await
    }

    pub async fn record_burn(&self, asset_id: &str, amount: &str) -> SdkResult<SecureMintApply> {
        self.rpc
            .call(
                "tenzro_secureMintRecordBurn",
                serde_json::json!([{ "asset_id": asset_id, "amount": amount }]),
            )
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecureMintPolicy {
    pub asset_id: String,
    pub reserve: String,
    #[serde(default)]
    pub circulating: String,
    pub por_feed_id: String,
    pub attester_did: String,
    pub attestation_hash: String,
    pub attested_at: u64,
    pub ttl_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureMintCheck {
    #[serde(default)]
    pub would_succeed: bool,
    #[serde(default)]
    pub reserve: String,
    #[serde(default)]
    pub circulating: String,
    #[serde(default)]
    pub headroom: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureMintApply {
    #[serde(default)]
    pub asset_id: String,
    #[serde(default)]
    pub new_circulating: String,
    #[serde(default)]
    pub reserve: String,
}
