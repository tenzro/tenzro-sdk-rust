use crate::error::SdkResult;
use crate::rpc::RpcClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct QuotaClient {
    rpc: Arc<RpcClient>,
}

impl QuotaClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    pub async fn get_burn_quota(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_getBurnQuota", serde_json::json!([])).await
    }

    pub async fn get_mempool_stats(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_getMempoolStats", serde_json::json!([]))
            .await
    }

    pub async fn get_mempool_lane(&self, lane: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getMempoolLane",
                serde_json::json!([{ "lane": lane }]),
            )
            .await
    }

    pub async fn get_account_contention(&self, address: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_getAccountContention", serde_json::json!([address]))
            .await
    }

    pub async fn get_da_backends(&self) -> SdkResult<Vec<serde_json::Value>> {
        self.rpc
            .call("tenzro_getDaBackends", serde_json::json!([]))
            .await
    }

    pub async fn verify_da_pointer(
        &self,
        pointer: serde_json::Value,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_verifyDaPointer", serde_json::json!([pointer]))
            .await
    }
}
