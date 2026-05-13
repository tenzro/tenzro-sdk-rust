use crate::error::SdkResult;
use crate::rpc::RpcClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct SeedAgentClient {
    rpc: Arc<RpcClient>,
}

impl SeedAgentClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    pub async fn get_treasury_earmark(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_getTreasuryEarmark", serde_json::json!([]))
            .await
    }

    pub async fn get_seed_agent_charter(&self, charter_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getSeedAgentCharter",
                serde_json::json!([charter_id]),
            )
            .await
    }

    pub async fn list_seed_agent_charters(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_listSeedAgentCharters", serde_json::json!([]))
            .await
    }

    pub async fn list_seed_agents(
        &self,
        charter_id: Option<&str>,
    ) -> SdkResult<serde_json::Value> {
        let params = match charter_id {
            Some(c) => serde_json::json!([c]),
            None => serde_json::json!([]),
        };
        self.rpc.call("tenzro_listSeedAgents", params).await
    }

    pub async fn get_network_activity(
        &self,
        window_blocks: Option<u64>,
    ) -> SdkResult<serde_json::Value> {
        let params = match window_blocks {
            Some(w) => serde_json::json!([w]),
            None => serde_json::json!([]),
        };
        self.rpc.call("tenzro_getNetworkActivity", params).await
    }
}
