//! Bridge fee in TNZO client.
//!
//! Cross-chain bridge fees are payable in TNZO instead of needing
//! destination-chain gas. The `BridgeFeeOracle` quotes the
//! destination-native fee in TNZO; the `BridgeFeeSponsor` debits the
//! user's account and credits a deterministic per-adapter sponsorship
//! pool. A registered solver / relayer fronts the destination-native
//! fee against the pool.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct BridgeFeeClient {
    rpc: Arc<RpcClient>,
}

impl BridgeFeeClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Quote a destination-native bridge fee in TNZO.
    /// `adapter` is one of `layerzero | ccip | wormhole | debridge |
    /// hyperlane | axelar | lifi | canton`; `dest_chain` is the
    /// CAIP-2 identifier (e.g. `eip155:1`).
    pub async fn quote(&self, req: QuoteBridgeFeeRequest) -> SdkResult<QuoteBridgeFeeResponse> {
        self.rpc
            .call("tenzro_quoteBridgeFeeInTnzo", serde_json::json!([req]))
            .await
    }

    /// Enumerate the canonical per-adapter sponsorship-pool vault
    /// addresses. Vault addresses are deterministic SHA-256 over
    /// `"tenzro/bridge/sponsorship-vault" || adapter` (first 20 bytes).
    pub async fn list_sponsorship_pools(&self) -> SdkResult<BridgeSponsorshipPools> {
        self.rpc
            .call("tenzro_listBridgeSponsorshipPools", serde_json::Value::Null)
            .await
    }

    /// (Admin-token-gated) Register a (adapter, dest_chain, rate_q18,
    /// markup_bps, valid_window_ms) row on the governance-set oracle.
    pub async fn set_rate(&self, req: SetBridgeFeeRateRequest) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_setBridgeFeeRate", serde_json::json!([req]))
            .await
    }

    /// Sponsor a previously-quoted destination-native fee. The caller's
    /// `payer_did` is debited and the per-adapter pool credited.
    pub async fn sponsor(
        &self,
        req: SponsorBridgeFeeRequest,
    ) -> SdkResult<BridgeSponsorshipReceipt> {
        self.rpc
            .call("tenzro_sponsorBridgeFee", serde_json::json!([req]))
            .await
    }

    /// (Admin-token-gated) Set the refill-threshold bps for an adapter's
    /// sponsorship pool.
    pub async fn set_refill_threshold(
        &self,
        adapter: impl Into<String>,
        refill_threshold_bps: u32,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_setSponsorshipRefillThreshold",
                serde_json::json!([{
                    "adapter": adapter.into(),
                    "refill_threshold_bps": refill_threshold_bps,
                }]),
            )
            .await
    }

    /// Subject self-read of the caller's own Chainlink/bridge analytics
    /// (CU consumption, per-method counters, error counts, rate-limit
    /// rejections).
    pub async fn get_analytics(&self) -> SdkResult<BridgeKeyAnalytics> {
        self.rpc
            .call("tenzro_getBridgeAnalytics", serde_json::Value::Null)
            .await
    }

    /// (Admin-token-gated) Operator cross-tenant read of every per-key
    /// Chainlink/bridge analytics aggregate.
    pub async fn list_analytics(&self) -> SdkResult<BridgeAnalyticsList> {
        self.rpc
            .call(
                "tenzro_listBridgeAnalytics",
                serde_json::Value::Null,
            )
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteBridgeFeeRequest {
    pub adapter: String,
    pub dest_chain: String,
    pub native_fee_smallest_unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteBridgeFeeResponse {
    pub adapter: String,
    pub dest_chain: String,
    pub native_fee_smallest_unit: String,
    pub tnzo_amount_wei: String,
    pub oracle_backing: String,
    pub note: Option<String>,
    pub supported_adapters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeSponsorshipPools {
    pub pools: Vec<BridgeSponsorshipPool>,
    pub total: usize,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeSponsorshipPool {
    pub adapter: String,
    pub vault_address_hex: String,
    pub tnzo_balance_wei: String,
    pub native_outstanding_smallest_unit: String,
    #[serde(default)]
    pub refill_threshold_bps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBridgeFeeRateRequest {
    pub adapter: String,
    pub dest_chain: String,
    /// Q18 fixed-point rate as decimal string.
    pub rate_q18: String,
    pub markup_bps: u32,
    pub valid_window_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SponsorBridgeFeeRequest {
    pub quote_id_hex: String,
    pub adapter: String,
    pub dest_chain: String,
    pub native_fee_smallest_unit: String,
    pub tnzo_amount_wei: String,
    pub rate_q18_hex: String,
    pub issued_at_ms: u64,
    pub valid_until_ms: u64,
    /// `governance` | `chainlink_feed` | `fallback`.
    #[serde(default)]
    pub oracle_backing: Option<String>,
    pub payer_did: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeSponsorshipReceipt {
    pub sponsorship_id_hex: String,
    pub quote_id_hex: String,
    pub adapter: String,
    pub dest_chain: String,
    pub payer_did: String,
    pub tnzo_paid_wei: String,
    pub native_committed_smallest_unit: String,
    pub sponsored_at_ms: u64,
    pub pool_address_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeKeyAnalytics {
    pub key_id: String,
    pub calls_total: u64,
    pub errors_total: u64,
    pub calls_by_method: std::collections::HashMap<String, u64>,
    pub errors_by_method: std::collections::HashMap<String, u64>,
    /// Alchemy-style Compute Units consumed (sum of per-method CU
    /// costs on the success path).
    pub cu_consumed_total: u64,
    pub first_seen_at: Option<i64>,
    pub last_called_at: Option<i64>,
    pub rate_limit_rejections: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeAnalyticsList {
    pub analytics: Vec<BridgeKeyAnalytics>,
}
