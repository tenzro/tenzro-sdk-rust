//! ERC-7943 (uRWA) client — universal Real-World Asset compliance.
//!
//! Tenzro implements ERC-7943, the canonical 2026 standard for
//! tokenized real-world assets that must respect regulator orders,
//! legal-entity mandates, and routine compliance (KYC-refresh-pending
//! sub-balance freeze).
//!
//! Read-only RPCs:
//! - `tenzro_urwaIsKillSwitched` — global kill-switch state per token.
//! - `tenzro_urwaGetFrozenTokens` — per-(token, account) frozen amount.
//!
//! Admin-gated mutation RPCs (these REQUIRE the `X-Tenzro-Admin-Token`
//! header set on the underlying `RpcClient`):
//! - `tenzro_urwaSetFrozenTokens` — freeze a specific amount.
//! - `tenzro_urwaTriggerKillSwitch` — activate the kill-switch.
//! - `tenzro_urwaClearKillSwitch` — clear the kill-switch.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct UrwaClient {
    rpc: Arc<RpcClient>,
}

impl UrwaClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Read-only check whether the kill-switch is active for the
    /// given 32-byte `token_id`.
    pub async fn is_kill_switched(
        &self,
        token_id_hex: impl Into<String>,
    ) -> SdkResult<UrwaKillSwitchState> {
        self.rpc
            .call(
                "tenzro_urwaIsKillSwitched",
                serde_json::json!([{
                    "token_id_hex": token_id_hex.into(),
                }]),
            )
            .await
    }

    /// Read-only frozen-amount lookup for a `(token_id, account)` pair.
    pub async fn get_frozen_tokens(
        &self,
        token_id_hex: impl Into<String>,
        account_hex: impl Into<String>,
    ) -> SdkResult<UrwaFrozenAmount> {
        self.rpc
            .call(
                "tenzro_urwaGetFrozenTokens",
                serde_json::json!([{
                    "token_id_hex": token_id_hex.into(),
                    "account_hex": account_hex.into(),
                }]),
            )
            .await
    }

    /// Admin-gated: freeze a specific amount of `token_id` on the
    /// given account. Optional `reason` is recorded for audit.
    pub async fn set_frozen_tokens(&self, req: UrwaSetFrozenRequest) -> SdkResult<UrwaFrozenRecord> {
        self.rpc
            .call("tenzro_urwaSetFrozenTokens", serde_json::json!([req]))
            .await
    }

    /// Admin-gated: activate the kill-switch on `token_id`.
    pub async fn trigger_kill_switch(
        &self,
        req: UrwaTriggerKillSwitchRequest,
    ) -> SdkResult<UrwaKillSwitchTriggered> {
        self.rpc
            .call("tenzro_urwaTriggerKillSwitch", serde_json::json!([req]))
            .await
    }

    /// Admin-gated: clear the kill-switch on `token_id`.
    pub async fn clear_kill_switch(
        &self,
        token_id_hex: impl Into<String>,
    ) -> SdkResult<UrwaKillSwitchCleared> {
        self.rpc
            .call(
                "tenzro_urwaClearKillSwitch",
                serde_json::json!([{
                    "token_id_hex": token_id_hex.into(),
                }]),
            )
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrwaKillSwitchState {
    pub token_id_hex: String,
    pub active: bool,
    pub selectors: serde_json::Value,
    pub precompile_addresses: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrwaFrozenAmount {
    pub token_id_hex: String,
    pub account_hex: String,
    pub frozen_amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrwaSetFrozenRequest {
    pub token_id_hex: String,
    pub account_hex: String,
    pub amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrwaFrozenRecord {
    pub token_id_hex: String,
    pub account_hex: String,
    pub amount: String,
    pub reason: Option<String>,
    pub set_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrwaTriggerKillSwitchRequest {
    pub token_id_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggered_by_did: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrwaKillSwitchTriggered {
    pub token_id_hex: String,
    pub active: bool,
    pub triggered_by_did: Option<String>,
    pub reason: Option<String>,
    pub triggered_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrwaKillSwitchCleared {
    pub token_id_hex: String,
    pub active: bool,
}
