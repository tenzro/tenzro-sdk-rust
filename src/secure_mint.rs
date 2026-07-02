//! Secure-Mint registry client.
//!
//! Enforces a per-token 1:1 reserve-attestation invariant for
//! tokenized real-world assets (tokenized-equity-class assets, treasuries,
//! stablecoins): `circulating + amount ≤ reserve` at every mint, with
//! `reserve` updated through a `SubmitReserveAttestation` flow gated
//! on a Proof-of-Reserve feed + attester DID.
//!
//! Beyond the static floor the policy carries operational guards:
//! `heartbeat_secs` (live-feed gate, distinct from `ttl_secs` staleness),
//! `mint_window_cap` / `mint_window_secs` (rolling issuance-velocity
//! ceiling), and per-token (`paused`) plus global circuit breakers.
//!
//! Policies are keyed by the 20-byte `token` address (0x-hex). The
//! free-form `asset_id` records the CAIP-19 reserve asset.
//!
//! Wraps the node-side RPCs:
//! - `tenzro_setSecureMintPolicy`, `tenzro_getSecureMintPolicy`,
//!   `tenzro_clearSecureMintPolicy`
//! - `tenzro_secureMintCheck` (read-only invariant check),
//!   `tenzro_secureMintApply` (atomic check + circulating increment),
//!   `tenzro_secureMintRecordBurn` (decrement on redemption)
//! - `tenzro_setSecureMintPaused`, `tenzro_setGlobalIssuancePause`
//!   (admin-token gated circuit breakers).

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

    pub async fn set_policy(&self, policy: SecureMintPolicy) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_setSecureMintPolicy", serde_json::json!([policy]))
            .await
    }

    pub async fn get_policy(&self, token: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getSecureMintPolicy",
                serde_json::json!([{ "token": token }]),
            )
            .await
    }

    pub async fn clear_policy(&self, token: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_clearSecureMintPolicy",
                serde_json::json!([{ "token": token }]),
            )
            .await
    }

    pub async fn check(&self, token: &str, amount: &str) -> SdkResult<SecureMintCheck> {
        self.rpc
            .call(
                "tenzro_secureMintCheck",
                serde_json::json!([{ "token": token, "amount": amount }]),
            )
            .await
    }

    pub async fn apply(&self, token: &str, amount: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_secureMintApply",
                serde_json::json!([{ "token": token, "amount": amount }]),
            )
            .await
    }

    pub async fn record_burn(&self, token: &str, amount: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_secureMintRecordBurn",
                serde_json::json!([{ "token": token, "amount": amount }]),
            )
            .await
    }

    /// Trip or clear the per-token issuance circuit breaker. Admin-gated.
    pub async fn set_paused(&self, token: &str, paused: bool) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_setSecureMintPaused",
                serde_json::json!([{ "token": token, "paused": paused }]),
            )
            .await
    }

    /// Trip or clear the global issuance circuit breaker, halting mint
    /// across every token at once. Admin-gated.
    pub async fn set_global_pause(&self, paused: bool) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_setGlobalIssuancePause",
                serde_json::json!([{ "paused": paused }]),
            )
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecureMintPolicy {
    /// 20-byte token address (0x-hex) — the policy key.
    pub token: String,
    /// CAIP-19 reserve asset id (e.g. iso4217:USD).
    pub asset_id: String,
    pub reserve: String,
    #[serde(default)]
    pub circulating: String,
    pub por_feed_id: String,
    pub attester_did: String,
    pub attestation_hash: String,
    pub attested_at: u64,
    pub ttl_secs: u64,
    /// PoR feed-liveness window in seconds (0 = disabled).
    #[serde(default)]
    pub heartbeat_secs: u64,
    /// Max amount mintable per rolling window (0 = uncapped).
    #[serde(default)]
    pub mint_window_cap: String,
    /// Length of the velocity window in seconds (0 = disabled).
    #[serde(default)]
    pub mint_window_secs: u64,
    /// Install the policy already tripped.
    #[serde(default)]
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureMintCheck {
    #[serde(default)]
    pub allowed: bool,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub reason: Option<String>,
}
