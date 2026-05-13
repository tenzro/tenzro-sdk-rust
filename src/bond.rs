use crate::error::SdkResult;
use crate::rpc::RpcClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct BondClient {
    rpc: Arc<RpcClient>,
}

impl BondClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    pub async fn get_agent_bond(&self, agent_did: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getAgentBond",
                serde_json::json!([{ "agent_did": agent_did }]),
            )
            .await
    }

    pub async fn list_agent_bonds_by_controller(
        &self,
        controller_did: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_listAgentBondsByController",
                serde_json::json!([{ "controller_did": controller_did }]),
            )
            .await
    }

    pub async fn post_agent_bond(
        &self,
        controller: &str,
        agent_did: &str,
        controller_did: &str,
        amount: u128,
    ) -> SdkResult<serde_json::Value> {
        let (nonce, chain_id) = self.fetch_nonce_and_chain_id(controller).await;
        let tx_type = serde_json::json!({
            "PostAgentBond": {
                "agent_did": agent_did,
                "controller_did": controller_did,
                "amount": amount,
            },
        });
        self.rpc
            .call(
                "tenzro_signAndSendTransaction",
                serde_json::json!([{
                    "from": controller,
                    "to": "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "value": 0,
                    "gas_limit": 75_000u64,
                    "gas_price": 1_000_000_000u64,
                    "nonce": nonce,
                    "chain_id": chain_id,
                    "tx_type": tx_type,
                }]),
            )
            .await
    }

    pub async fn increase_agent_bond(
        &self,
        controller: &str,
        agent_did: &str,
        amount: u128,
    ) -> SdkResult<serde_json::Value> {
        let (nonce, chain_id) = self.fetch_nonce_and_chain_id(controller).await;
        let tx_type = serde_json::json!({
            "IncreaseAgentBond": { "agent_did": agent_did, "amount": amount },
        });
        self.rpc
            .call(
                "tenzro_signAndSendTransaction",
                serde_json::json!([{
                    "from": controller,
                    "to": "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "value": 0,
                    "gas_limit": 60_000u64,
                    "gas_price": 1_000_000_000u64,
                    "nonce": nonce,
                    "chain_id": chain_id,
                    "tx_type": tx_type,
                }]),
            )
            .await
    }

    pub async fn withdraw_agent_bond(
        &self,
        controller: &str,
        agent_did: &str,
    ) -> SdkResult<serde_json::Value> {
        let (nonce, chain_id) = self.fetch_nonce_and_chain_id(controller).await;
        let tx_type = serde_json::json!({
            "WithdrawAgentBond": { "agent_did": agent_did },
        });
        self.rpc
            .call(
                "tenzro_signAndSendTransaction",
                serde_json::json!([{
                    "from": controller,
                    "to": "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "value": 0,
                    "gas_limit": 50_000u64,
                    "gas_price": 1_000_000_000u64,
                    "nonce": nonce,
                    "chain_id": chain_id,
                    "tx_type": tx_type,
                }]),
            )
            .await
    }

    async fn fetch_nonce_and_chain_id(&self, address: &str) -> (u64, u64) {
        let nonce = self
            .rpc
            .call::<String>(
                "eth_getTransactionCount",
                serde_json::json!([address, "latest"]),
            )
            .await
            .ok()
            .and_then(|h| u64::from_str_radix(h.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0);
        let chain_id = self
            .rpc
            .call::<String>("eth_chainId", serde_json::json!([]))
            .await
            .ok()
            .and_then(|h| u64::from_str_radix(h.trim_start_matches("0x"), 16).ok())
            .unwrap_or(1337);
        (nonce, chain_id)
    }
}
