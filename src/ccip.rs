//! Chainlink CCIP client — first-class regulated-rail SDK surface.
//!
//! CCIP is Tenzro's institutional cross-chain rail: a Chainlink-operated
//! OCR commit-store committee plus an independent RMN (Risk Management
//! Network) ARM that must co-bless every inbound message. Use this
//! client when the bridge leg must ride a regulated, attested rail
//! rather than a generic permissionless protocol.
//!
//! The 9 methods mirror the `tenzro_ccip*` JSON-RPC namespace on the
//! node:
//!
//! - [`get_fee`](CcipClient::get_fee) — Router.getFee() eth_call
//! - [`send`](CcipClient::send) — Router.ccipSend() calldata + msg.value
//! - [`track`](CcipClient::track) — OffRamp.getExecutionState()
//! - [`supported_chains`](CcipClient::supported_chains) — docs API
//! - [`supported_tokens`](CcipClient::supported_tokens) — docs API
//! - [`lanes`](CcipClient::lanes) — available lane pairs
//! - [`token_pool`](CcipClient::token_pool) — CCT v1.6+ pool info
//! - [`rate_limits`](CcipClient::rate_limits) — pool rate-limiter state
//! - [`bridge`](CcipClient::bridge) — router-mediated transfer that
//!   pins the route to the CCIP adapter

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Token amount payload for CCIP message construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcipTokenAmount {
    /// ERC-20 token address on the source chain (hex with `0x`).
    pub token: String,
    /// Amount in token base units as a decimal string.
    pub amount: String,
}

/// `Router.getFee()` quote, in source-chain native units.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcipFeeQuote {
    #[serde(default)]
    pub source_chain: String,
    #[serde(default)]
    pub router_address: String,
    #[serde(default)]
    pub dest_chain_selector: String,
    #[serde(default)]
    pub fee_token: String,
    /// Native fee in wei as a decimal string.
    #[serde(default)]
    pub fee_wei: String,
    /// Native fee in the native unit (e.g. ETH) for display.
    #[serde(default)]
    pub fee_native: String,
}

/// `Router.ccipSend()` envelope — calldata + msg.value ready for the
/// caller to sign and broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcipSendEnvelope {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub source_chain: String,
    #[serde(default)]
    pub router_address: String,
    #[serde(default)]
    pub dest_chain_selector: String,
    /// Hex-encoded calldata to attach to the `to=router_address` tx.
    #[serde(default)]
    pub calldata: String,
    /// Native value to attach as `msg.value`, in wei (decimal string).
    #[serde(default)]
    pub msg_value_wei: String,
    #[serde(default)]
    pub gas_limit_destination: u64,
    #[serde(default)]
    pub note: String,
}

/// `OffRamp.getExecutionState()` result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcipExecutionState {
    #[serde(default)]
    pub message_id: String,
    #[serde(default)]
    pub dest_chain: String,
    #[serde(default)]
    pub offramp_address: String,
    #[serde(default)]
    pub execution_state: u8,
    #[serde(default)]
    pub state_name: String,
    #[serde(default)]
    pub description: String,
}

/// Router-mediated CCIP transfer receipt. On failure, `status` and
/// `error` are set instead of the success fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcipTransferResult {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub registered_adapters: Option<Vec<String>>,
    #[serde(default)]
    pub transfer_id: String,
    #[serde(default)]
    pub source_chain: String,
    #[serde(default)]
    pub dest_chain: String,
    #[serde(default)]
    pub tx_hash: String,
    #[serde(default)]
    pub fee_paid: String,
    #[serde(default)]
    pub estimated_arrival_ms: i64,
    #[serde(default)]
    pub adapter: String,
}

/// Client for the Chainlink CCIP regulated rail. Mirrors the
/// `tenzro_ccip*` RPC family on the node.
#[derive(Clone)]
pub struct CcipClient {
    rpc: Arc<RpcClient>,
}

impl CcipClient {
    /// Creates a new CCIP client.
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Quote a CCIP fee via `Router.getFee()` eth_call against the
    /// source-chain Router.
    pub async fn get_fee(
        &self,
        source_chain: &str,
        dest_chain: &str,
        receiver: &str,
        data_hex: &str,
        token_amounts: &[CcipTokenAmount],
        fee_token: Option<&str>,
    ) -> SdkResult<CcipFeeQuote> {
        self.rpc
            .call(
                "tenzro_ccipGetFee",
                serde_json::json!([{
                    "source_chain": source_chain,
                    "dest_chain": dest_chain,
                    "receiver": receiver,
                    "data_hex": data_hex,
                    "token_amounts": token_amounts,
                    "fee_token": fee_token,
                }]),
            )
            .await
    }

    /// Prepare a `Router.ccipSend()` envelope. Signing and broadcasting
    /// are left to the caller — the returned `calldata` and
    /// `msg_value_wei` go into an `eth_sendRawTransaction` call.
    pub async fn send(
        &self,
        source_chain: &str,
        dest_chain: &str,
        receiver: &str,
        data_hex: &str,
        token_amounts: &[CcipTokenAmount],
        fee_token: Option<&str>,
        gas_limit: Option<u64>,
    ) -> SdkResult<CcipSendEnvelope> {
        self.rpc
            .call(
                "tenzro_ccipSend",
                serde_json::json!([{
                    "source_chain": source_chain,
                    "dest_chain": dest_chain,
                    "receiver": receiver,
                    "data_hex": data_hex,
                    "token_amounts": token_amounts,
                    "fee_token": fee_token,
                    "gas_limit": gas_limit,
                }]),
            )
            .await
    }

    /// Track a CCIP message via `OffRamp.getExecutionState(bytes32)`.
    pub async fn track(
        &self,
        message_id: &str,
        dest_chain: &str,
        offramp_address: &str,
    ) -> SdkResult<CcipExecutionState> {
        self.rpc
            .call(
                "tenzro_ccipTrack",
                serde_json::json!([{
                    "message_id": message_id,
                    "dest_chain": dest_chain,
                    "offramp_address": offramp_address,
                }]),
            )
            .await
    }

    /// List CCIP-supported chains (passed through from the Chainlink
    /// docs API). `environment` is `"mainnet"` or `"testnet"`.
    pub async fn supported_chains(&self, environment: Option<&str>) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_ccipSupportedChains",
                serde_json::json!([{ "environment": environment }]),
            )
            .await
    }

    /// List CCIP-supported tokens.
    pub async fn supported_tokens(&self, environment: Option<&str>) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_ccipSupportedTokens",
                serde_json::json!([{ "environment": environment }]),
            )
            .await
    }

    /// List CCIP lanes (source-destination pairs). Both selector
    /// filters are optional.
    pub async fn lanes(
        &self,
        environment: Option<&str>,
        source_chain_selector: Option<&str>,
        dest_chain_selector: Option<&str>,
    ) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_ccipLanes",
                serde_json::json!([{
                    "environment": environment,
                    "source_chain_selector": source_chain_selector,
                    "dest_chain_selector": dest_chain_selector,
                }]),
            )
            .await
    }

    /// Inspect a CCIP CCT v1.6+ token-pool contract.
    pub async fn token_pool(&self, chain: &str, pool_address: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_ccipTokenPool",
                serde_json::json!([{
                    "chain": chain,
                    "pool_address": pool_address,
                }]),
            )
            .await
    }

    /// Read inbound + outbound rate-limiter state for a (pool,
    /// remote-chain) pair.
    pub async fn rate_limits(
        &self,
        chain: &str,
        pool_address: &str,
        remote_chain: &str,
    ) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_ccipRateLimits",
                serde_json::json!([{
                    "chain": chain,
                    "pool_address": pool_address,
                    "remote_chain": remote_chain,
                }]),
            )
            .await
    }

    /// Bridge tokens through the node's BridgeRouter, pinned to the
    /// CCIP regulated rail. This is the institutional-leg entry point:
    /// the router refuses the call if no CCIP adapter is registered
    /// rather than silently falling back to a generic adapter.
    pub async fn bridge(
        &self,
        source_chain: &str,
        dest_chain: &str,
        asset: &str,
        amount: &str,
        sender: &str,
        recipient: &str,
    ) -> SdkResult<CcipTransferResult> {
        self.rpc
            .call(
                "tenzro_ccipBridge",
                serde_json::json!([{
                    "source_chain": source_chain,
                    "dest_chain": dest_chain,
                    "asset": asset,
                    "amount": amount,
                    "sender": sender,
                    "recipient": recipient,
                }]),
            )
            .await
    }
}
