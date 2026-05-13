use crate::error::SdkResult;
use crate::rpc::RpcClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct LifecycleClient {
    rpc: Arc<RpcClient>,
}

impl LifecycleClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    pub async fn get_agent_lifecycle(&self, agent_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getAgentLifecycle",
                serde_json::json!([{ "agent_id": agent_id }]),
            )
            .await
    }

    pub async fn list_kill_switch_by_agent(
        &self,
        agent_id: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_listKillSwitchByAgent",
                serde_json::json!([{ "agent_did": agent_id }]),
            )
            .await
    }

    pub async fn list_kill_switch_by_controller(
        &self,
        controller_did: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_listKillSwitchByController",
                serde_json::json!([{ "controller_did": controller_did }]),
            )
            .await
    }

    pub async fn get_kill_switch_receipt(&self, receipt_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getKillSwitchReceipt",
                serde_json::json!([receipt_id]),
            )
            .await
    }
}
