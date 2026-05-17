//! Adaptive-burn governance dial SDK for Tenzro Network.
//!
//! Surfaces the current `BurnRateConfig`, rolling `SupplyMetricsSnapshot`,
//! the recommended action computed from the targets and metrics, and the
//! list of in-flight adaptive-burn governance proposals.
//!
//! All endpoints are read-only — the auto-proposal generator and the
//! EIP-1559 fee-market consumer ship alongside the governance executor
//! wiring in a later wave.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde_json::Value;
use std::sync::Arc;

/// Adaptive-burn governance dial client.
#[derive(Clone)]
pub struct AdaptiveBurnClient {
    rpc: Arc<RpcClient>,
}

impl AdaptiveBurnClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Returns the current `BurnRateConfig` (base / local / paymaster burn bps).
    pub async fn get_burn_rate_config(&self) -> SdkResult<Value> {
        self.rpc
            .call("tenzro_getBurnRateConfig", serde_json::json!({}))
            .await
    }

    /// Returns the latest rolling supply metrics snapshot — circulating
    /// supply, epoch delta, burn breakdown, emission breakdown.
    pub async fn get_supply_metrics(&self) -> SdkResult<Value> {
        self.rpc
            .call("tenzro_getSupplyMetrics", serde_json::json!({}))
            .await
    }

    /// Computes the recommended adaptive-burn action from the current
    /// metrics and configured supply targets. May return `NoChange`,
    /// `IncreaseBurnPct`, `DecreaseBurnPct`, or alarm variants.
    pub async fn get_burn_rate_recommendation(&self) -> SdkResult<Value> {
        self.rpc
            .call("tenzro_getBurnRateRecommendation", serde_json::json!({}))
            .await
    }

    /// List in-flight adaptive-burn governance proposals.
    pub async fn list_adaptive_burn_proposals(&self) -> SdkResult<Vec<Value>> {
        self.rpc
            .call("tenzro_listAdaptiveBurnProposals", serde_json::json!({}))
            .await
    }
}
