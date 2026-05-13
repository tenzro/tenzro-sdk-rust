use crate::error::SdkResult;
use crate::rpc::RpcClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct PrincipalChainClient {
    rpc: Arc<RpcClient>,
}

impl PrincipalChainClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    pub async fn get_receipt_principal_chain(
        &self,
        receipt_id: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getReceiptPrincipalChain",
                serde_json::json!([{ "receipt_id": receipt_id }]),
            )
            .await
    }

    pub async fn list_receipts_by_actor(
        &self,
        actor_did: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_listReceiptsByActor",
                serde_json::json!([{ "did": actor_did }]),
            )
            .await
    }

    pub async fn list_receipts_by_controller(
        &self,
        controller_did: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_listReceiptsByController",
                serde_json::json!([{ "did": controller_did }]),
            )
            .await
    }

    pub async fn summarize_controller(
        &self,
        controller_did: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_summarizeController",
                serde_json::json!([{ "did": controller_did }]),
            )
            .await
    }
}
