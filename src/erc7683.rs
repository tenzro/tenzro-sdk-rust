//! ERC-7683 cross-chain intent settler SDK for Tenzro Network (Spec 4).
//!
//! Wraps the `tenzro_get7683Order` / `tenzro_list7683Orders` read RPCs on
//! the origin side and the `tenzro_recordFill7683` / `tenzro_getFill7683` /
//! `tenzro_listFills7683` RPCs on the destination side. The Tenzro ERC-7683
//! envelope is `Tenzro7683Order` persisted in `CF_SETTLEMENTS` under the
//! `7683_origin:` keyspace; fill records live under `7683_dest:`.
//!
//! Order state machine: `Open → AwaitingProof → Settled / Refunded /
//! ForceRefundEligible`. The settler precompiles, EIP-712 verification,
//! escrow integration, gossipsub indexing, and bridge-adapter glue land
//! in subsequent waves alongside their respective subsystems.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// ERC-7683 cross-chain intent client.
#[derive(Clone)]
pub struct Erc7683Client {
    rpc: Arc<RpcClient>,
}

impl Erc7683Client {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Fetch a single persisted `Tenzro7683Order` by 32-byte `order_id`
    /// (hex, with or without `0x` prefix). Returns the JSON envelope
    /// produced by the node's `tenzro_7683_order_to_json` projection.
    pub async fn get_order(&self, order_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_get7683Order",
                serde_json::json!({ "order_id": order_id }),
            )
            .await
    }

    /// Paginated scan over the `7683_origin:` keyspace. All filters are
    /// optional; pass `None` to skip a filter.
    ///
    /// * `state` — one of `open`, `awaiting_proof`, `settled`, `refunded`,
    ///   `force_refund_eligible`.
    /// * `dest_chain` — CAIP-2 numeric destination chain id.
    /// * `limit` — cap on returned envelopes (default 50).
    pub async fn list_orders(
        &self,
        state: Option<&str>,
        dest_chain: Option<u32>,
        limit: Option<usize>,
    ) -> SdkResult<Erc7683OrderList> {
        let mut params = serde_json::Map::new();
        if let Some(s) = state {
            params.insert("state".to_string(), Value::String(s.to_string()));
        }
        if let Some(dc) = dest_chain {
            params.insert("dest_chain".to_string(), Value::from(dc));
        }
        if let Some(l) = limit {
            params.insert("limit".to_string(), Value::from(l));
        }
        self.rpc
            .call("tenzro_list7683Orders", Value::Object(params))
            .await
    }

    /// Destination-side write: commit a `FillRecord` for an order that
    /// has been filled on the destination chain. Idempotency-guarded —
    /// the second call for the same `(order_id, origin_chain_id)` pair
    /// is rejected by the node.
    ///
    /// `proof_route` must be one of `layerzero`, `wormhole`, `debridge`,
    /// `hyperlane`.
    #[allow(clippy::too_many_arguments)]
    pub async fn record_fill(
        &self,
        order_id: &str,
        origin_chain_id: u32,
        origin_settler: &str,
        filler: &str,
        recipient: &str,
        fill_tx_hash: &str,
        filled_at_ms: i64,
        proof_route: &str,
        outputs: Vec<Erc7683Output>,
    ) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_recordFill7683",
                serde_json::json!({
                    "order_id": order_id,
                    "origin_chain_id": origin_chain_id,
                    "origin_settler": origin_settler,
                    "filler": filler,
                    "recipient": recipient,
                    "fill_tx_hash": fill_tx_hash,
                    "filled_at_ms": filled_at_ms,
                    "proof_route": proof_route,
                    "outputs": outputs,
                }),
            )
            .await
    }

    /// Fetch a single persisted `FillRecord` by `(order_id, origin_chain_id)`.
    pub async fn get_fill(&self, order_id: &str, origin_chain_id: u32) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_getFill7683",
                serde_json::json!({
                    "order_id": order_id,
                    "origin_chain_id": origin_chain_id,
                }),
            )
            .await
    }

    /// List every persisted `FillRecord` in the `7683_dest:` keyspace.
    pub async fn list_fills(&self) -> SdkResult<Value> {
        self.rpc
            .call("tenzro_listFills7683", serde_json::json!({}))
            .await
    }
}

/// A single ERC-7683 `Output` — chain-discriminated 32-byte recipient +
/// uint256 amount.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc7683Output {
    /// Token address (hex, chain-native — 20-byte EVM left-padded to 32
    /// or 32-byte SVM mint).
    pub token: String,
    /// uint256 amount in token's smallest unit, as 32-byte big-endian hex.
    pub amount: String,
    /// 32-byte recipient on the destination chain (hex).
    pub recipient: String,
    /// CAIP-2 numeric chain id of the destination chain.
    pub chain_id: u32,
}

/// Result of `tenzro_list7683Orders`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc7683OrderList {
    #[serde(default)]
    pub orders: Vec<Value>,
    #[serde(default)]
    pub count: usize,
}
