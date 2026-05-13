use crate::error::SdkResult;
use crate::rpc::RpcClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct InsuranceClient {
    rpc: Arc<RpcClient>,
}

impl InsuranceClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    pub async fn file_insurance_claim(
        &self,
        params: serde_json::Value,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_fileInsuranceClaim", serde_json::json!([params]))
            .await
    }

    pub async fn list_insurance_claims(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_listInsuranceClaims", serde_json::json!([]))
            .await
    }

    pub async fn get_insurance_claim(&self, claim_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getInsuranceClaim",
                serde_json::json!([{ "claim_id": claim_id }]),
            )
            .await
    }

    pub async fn get_insurance_pool_balance(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_getInsurancePoolBalance", serde_json::json!([]))
            .await
    }
}
