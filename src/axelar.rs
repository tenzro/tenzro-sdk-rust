//! Axelar GMP cross-chain messaging client.
//!
//! Axelar provides reach into 30+ chains spanning EVM, Cosmos
//! (Osmosis, Cosmos Hub, Juno, Neutron, Injective, Kujira, Crescent,
//! Evmos), Move (Aptos, Sui), Stellar, XRP Ledger, Hyperliquid,
//! Filecoin EVM, and Kava. Tenzro's Axelar adapter uses the canonical
//! `call_contract` GMP entrypoint with a Gas Service pre-pay; the
//! correlation id is `keccak256(payload)`.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct AxelarClient {
    rpc: Arc<RpcClient>,
}

impl AxelarClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    pub async fn list_chains(&self) -> SdkResult<Vec<AxelarChain>> {
        self.rpc
            .call("tenzro_axelarListChains", serde_json::json!([]))
            .await
    }

    pub async fn call_contract(
        &self,
        req: AxelarCallContractRequest,
    ) -> SdkResult<AxelarCallContractResult> {
        self.rpc
            .call("tenzro_axelarCallContract", serde_json::json!([req]))
            .await
    }

    pub async fn pay_gas(&self, req: AxelarPayGasRequest) -> SdkResult<AxelarPayGasResult> {
        self.rpc
            .call("tenzro_axelarPayGas", serde_json::json!([req]))
            .await
    }

    pub async fn get_message(&self, payload_hash: &str) -> SdkResult<Option<AxelarMessage>> {
        self.rpc
            .call(
                "tenzro_axelarGetMessage",
                serde_json::json!([{ "payload_hash": payload_hash }]),
            )
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxelarChain {
    #[serde(default)]
    pub chain_id: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub gateway: Option<String>,
    #[serde(default)]
    pub gas_service: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AxelarCallContractRequest {
    pub source_chain: String,
    pub destination_chain: String,
    pub destination_address: String,
    pub payload_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_amount: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxelarCallContractResult {
    #[serde(default)]
    pub payload_hash: String,
    #[serde(default)]
    pub source_chain: String,
    #[serde(default)]
    pub destination_chain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AxelarPayGasRequest {
    pub payload_hash: String,
    pub source_chain: String,
    pub destination_chain: String,
    pub destination_address: String,
    pub gas_token: String,
    pub gas_amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxelarPayGasResult {
    #[serde(default)]
    pub paid: bool,
    #[serde(default)]
    pub gas_token: String,
    #[serde(default)]
    pub gas_amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxelarMessage {
    #[serde(default)]
    pub payload_hash: String,
    #[serde(default)]
    pub source_chain: String,
    #[serde(default)]
    pub destination_chain: String,
    #[serde(default)]
    pub destination_address: String,
    #[serde(default)]
    pub status: String,
}
