//! Wormhole cross-chain SDK for Tenzro Network
//!
//! Wraps the `tenzro_wormhole*` RPC family: chain id lookup, VAA id parsing,
//! and token bridging via the Wormhole adapter registered on the node's
//! BridgeRouter.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Wormhole cross-chain client.
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::TenzroClient;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = TenzroClient::new("https://rpc.tenzro.network").await?;
/// let wormhole = client.wormhole();
/// let id = wormhole.chain_id("ethereum").await?;
/// assert_eq!(id.wormhole_chain_id, 2);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct WormholeClient {
    rpc: Arc<RpcClient>,
}

impl WormholeClient {
    /// Creates a new Wormhole client.
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Look up the Wormhole numeric chain id for a chain name
    /// (e.g. ethereum=2, solana=1, base=30, arbitrum=23, optimism=24).
    pub async fn chain_id(&self, chain: &str) -> SdkResult<WormholeChainId> {
        self.rpc
            .call(
                "tenzro_wormholeChainId",
                serde_json::json!({ "chain": chain }),
            )
            .await
    }

    /// Parse a canonical Wormhole VAA id of the form
    /// `{chain}/{emitter}/{sequence}` into its components.
    pub async fn parse_vaa_id(&self, vaa_id: &str) -> SdkResult<WormholeVaaId> {
        self.rpc
            .call(
                "tenzro_wormholeParseVaaId",
                serde_json::json!({ "vaa_id": vaa_id }),
            )
            .await
    }

    /// Bridge tokens through the Wormhole adapter on the BridgeRouter.
    ///
    /// `amount` is a decimal string in the smallest asset units.
    pub async fn bridge(
        &self,
        source_chain: &str,
        dest_chain: &str,
        asset: &str,
        amount: &str,
        sender: &str,
        recipient: &str,
    ) -> SdkResult<WormholeTransferResult> {
        self.rpc
            .call(
                "tenzro_wormholeBridge",
                serde_json::json!({
                    "source_chain": source_chain,
                    "dest_chain": dest_chain,
                    "asset": asset,
                    "amount": amount,
                    "sender": sender,
                    "recipient": recipient,
                }),
            )
            .await
    }
}

/// Wormhole chain id lookup result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WormholeChainId {
    /// Chain name echoed back.
    #[serde(default)]
    pub chain: String,
    /// Wormhole-assigned numeric chain id.
    #[serde(default)]
    pub wormhole_chain_id: u16,
}

/// Parsed components of a `{chain}/{emitter}/{sequence}` VAA id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WormholeVaaId {
    /// Emitter chain id.
    #[serde(default)]
    pub emitter_chain: u16,
    /// Emitter address (hex for EVM, base58 for Solana).
    #[serde(default)]
    pub emitter_address: String,
    /// Monotonic sequence number.
    #[serde(default)]
    pub sequence: u64,
}

/// Wormhole bridge transfer result. On failure, `status` and `error` are set
/// instead of the success fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WormholeTransferResult {
    /// Status string ("failed" on error; unset on success).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Error detail, populated when the transfer was rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Adapters registered on the router (set on error for diagnostics).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registered_adapters: Option<Vec<String>>,

    /// Unique transfer identifier assigned by the adapter.
    #[serde(default)]
    pub transfer_id: String,
    /// Source chain name echoed back.
    #[serde(default)]
    pub source_chain: String,
    /// Destination chain name echoed back.
    #[serde(default)]
    pub dest_chain: String,
    /// On-chain transaction hash.
    #[serde(default)]
    pub tx_hash: String,
    /// Fee paid (decimal string).
    #[serde(default)]
    pub fee_paid: String,
    /// Estimated arrival time on the destination chain, in milliseconds.
    #[serde(default)]
    pub estimated_arrival_ms: u64,
}
