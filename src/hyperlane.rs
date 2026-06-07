//! Hyperlane V3 cross-chain messaging client.
//!
//! Hyperlane is a permissionless interchain messaging protocol.
//! Tenzro runs a sovereign Tenzro-validator-set ISM
//! (Interchain Security Module): inbound Hyperlane messages are
//! verified against the active Tenzro validator BLS / ML-DSA set, and
//! outbound messages are dispatched through the canonical Mailbox.
//!
//! Coverage: Ethereum, Polygon, Arbitrum, Optimism, Base, Avalanche,
//! BSC, Mantle, Blast, Scroll, Linea, Manta, zkSync, Celo, Moonbeam,
//! Mode, Fraxtal, Tenzro (and growing).

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct HyperlaneClient {
    rpc: Arc<RpcClient>,
}

impl HyperlaneClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    pub async fn list_chains(&self) -> SdkResult<Vec<HyperlaneChain>> {
        self.rpc
            .call("tenzro_hyperlaneListChains", serde_json::json!([]))
            .await
    }

    pub async fn quote_dispatch(
        &self,
        req: HyperlaneDispatchRequest,
    ) -> SdkResult<HyperlaneDispatchQuote> {
        self.rpc
            .call("tenzro_hyperlaneQuoteDispatch", serde_json::json!([req]))
            .await
    }

    pub async fn dispatch(
        &self,
        req: HyperlaneDispatchRequest,
    ) -> SdkResult<HyperlaneDispatchResult> {
        self.rpc
            .call("tenzro_hyperlaneDispatch", serde_json::json!([req]))
            .await
    }

    pub async fn get_message(&self, message_id: &str) -> SdkResult<Option<HyperlaneMessage>> {
        self.rpc
            .call(
                "tenzro_hyperlaneGetMessage",
                serde_json::json!([{ "message_id": message_id }]),
            )
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperlaneChain {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub domain_id: u32,
    #[serde(default)]
    pub mailbox: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HyperlaneDispatchRequest {
    pub origin_domain: u32,
    pub destination_domain: u32,
    pub recipient: String,
    pub body_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interchain_gas_payment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperlaneDispatchQuote {
    #[serde(default)]
    pub gas_payment: String,
    #[serde(default)]
    pub gas_payment_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperlaneDispatchResult {
    #[serde(default)]
    pub message_id: String,
    #[serde(default)]
    pub nonce: u32,
    #[serde(default)]
    pub origin_domain: u32,
    #[serde(default)]
    pub destination_domain: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperlaneMessage {
    #[serde(default)]
    pub message_id: String,
    #[serde(default)]
    pub nonce: u32,
    #[serde(default)]
    pub origin_domain: u32,
    #[serde(default)]
    pub destination_domain: u32,
    #[serde(default)]
    pub sender: String,
    #[serde(default)]
    pub recipient: String,
    #[serde(default)]
    pub body_hex: String,
    #[serde(default)]
    pub status: String,
}
