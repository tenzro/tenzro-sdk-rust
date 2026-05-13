use crate::error::SdkResult;
use crate::rpc::RpcClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct AdaptiveBurnClient {
    rpc: Arc<RpcClient>,
}

impl AdaptiveBurnClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    pub async fn get_burn_rate_config(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_getBurnRateConfig", serde_json::json!([]))
            .await
    }

    pub async fn get_supply_metrics(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_getSupplyMetrics", serde_json::json!([]))
            .await
    }

    pub async fn get_burn_rate_recommendation(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_getBurnRateRecommendation", serde_json::json!([]))
            .await
    }

    pub async fn list_adaptive_burn_proposals(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_listAdaptiveBurnProposals", serde_json::json!([]))
            .await
    }
}
