//! Wormhole NTT (Native Token Transfers) catalog client.
//!
//! Surfaces the registered NttManager chain catalog + supported
//! Transceiver kinds. NTT lets a token's `NttManager` mint / burn the
//! native token directly on each chain with quorum-aggregated
//! Transceiver attestation.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct WormholeNttClient {
    rpc: Arc<RpcClient>,
}

impl WormholeNttClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Enumerate the Wormhole NTT chain catalog (chain ids + names)
    /// supported by this Tenzro deployment, plus the Transceiver
    /// kinds the relying party can compose into a quorum.
    pub async fn list_chains(&self) -> SdkResult<WormholeNttChains> {
        self.rpc
            .call("tenzro_wormholeNttListChains", serde_json::Value::Null)
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WormholeNttChains {
    pub chains: Vec<WormholeNttChain>,
    pub transceiver_kinds: Vec<String>,
    pub scaffolding: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WormholeNttChain {
    pub wormhole_chain_id: u16,
    pub name: String,
}
