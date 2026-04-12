//! Cross-Chain Bridge SDK for Tenzro Network
//!
//! This module provides cross-chain bridge operations for transferring tokens
//! between Tenzro, Ethereum, Solana, Base, and other supported chains.
//!
//! ## Supported Bridge Adapters
//!
//! - **LayerZero V2** — Omnichain messaging via EndpointV2. OFT transfers use `uint64 amountSD`
//!   (shared decimals, not `uint256`) and TYPE_3 options encoding. Supported chain EIDs:
//!   Ethereum (30101), BSC (30102), Avalanche (30106), Polygon (30109), Arbitrum (30110),
//!   Optimism (30111), zkSync (30165), Base (30184), Solana (30168), Sei (30280),
//!   Sonic (30332), Berachain (30362), Story (30364), Monad (30390), MegaETH (30398),
//!   Tron (30420).
//! - **Chainlink CCIP** — Cross-chain interoperability with `allowOutOfOrderExecution = true`.
//!   Router addresses: BSC (`0x34B03Cb9086d7D758AC55af71584F81A598759FE`),
//!   Base (`0x881e3A65B4d4a04dD529061dd0071cf975F58bCD`).
//! - **deBridge DLN** — Intent-based cross-chain swaps. Order status tracked via
//!   `https://stats-api.dln.trade/api/Orders/{id}`. Status mapping: `ClaimedUnlock`/`SentUnlock`
//!   map to `Filled` (not `Created`).
//! - **Canton** — Enterprise DAML ledger bridge via Canton 3.x JSON Ledger API v2.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Bridge client for cross-chain token transfer operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let bridge = client.bridge();
///
/// // Get available routes from Tenzro to Ethereum
/// let routes = bridge.get_routes("tenzro", "ethereum", "TNZO").await?;
/// for route in &routes {
///     println!("{}: fee {} (est. {} secs)", route.adapter, route.fee, route.estimated_time);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct BridgeClient {
    rpc: Arc<RpcClient>,
}

impl BridgeClient {
    /// Creates a new bridge client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Bridges tokens between chains
    ///
    /// Initiates a cross-chain token transfer using the specified adapter
    /// (LayerZero V2, Chainlink CCIP, deBridge DLN, or Canton).
    ///
    /// For LayerZero OFT transfers, amounts are encoded as `uint64 amountSD` (shared
    /// decimals) with TYPE_3 options. For deBridge, order status is tracked via the
    /// stats API at `stats-api.dln.trade`.
    ///
    /// # Arguments
    ///
    /// * `from_chain` - Source chain (e.g., "tenzro", "ethereum", "solana")
    /// * `to_chain` - Destination chain
    /// * `token` - Token symbol (e.g., "TNZO", "USDC")
    /// * `amount` - Amount to bridge as a decimal string
    /// * `recipient` - Recipient address on the destination chain
    /// * `adapter` - Bridge adapter to use (e.g., "layerzero", "ccip", "debridge")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let bridge = client.bridge();
    /// let transfer = bridge.bridge_tokens(
    ///     "tenzro",
    ///     "ethereum",
    ///     "TNZO",
    ///     "1000000000000000000",
    ///     "0xrecipient...",
    ///     "layerzero",
    /// ).await?;
    /// println!("Transfer: {} (status: {})", transfer.transfer_id, transfer.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bridge_tokens(
        &self,
        from_chain: &str,
        to_chain: &str,
        token: &str,
        amount: &str,
        recipient: &str,
        adapter: &str,
    ) -> SdkResult<BridgeTransfer> {
        self.rpc
            .call(
                "tenzro_bridgeTokens",
                serde_json::json!([{
                    "from_chain": from_chain,
                    "to_chain": to_chain,
                    "token": token,
                    "amount": amount,
                    "recipient": recipient,
                    "adapter": adapter,
                }]),
            )
            .await
    }

    /// Gets available bridge routes between two chains for a token
    ///
    /// # Arguments
    ///
    /// * `from_chain` - Source chain
    /// * `to_chain` - Destination chain
    /// * `token` - Token symbol
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let bridge = client.bridge();
    /// let routes = bridge.get_routes("tenzro", "solana", "USDC").await?;
    /// println!("Found {} routes", routes.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_routes(
        &self,
        from_chain: &str,
        to_chain: &str,
        token: &str,
    ) -> SdkResult<Vec<BridgeRoute>> {
        self.rpc
            .call(
                "tenzro_getBridgeRoutes",
                serde_json::json!([{
                    "from_chain": from_chain,
                    "to_chain": to_chain,
                    "token": token,
                }]),
            )
            .await
    }

    /// Lists all registered bridge adapters
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let bridge = client.bridge();
    /// let adapters = bridge.list_adapters().await?;
    /// for a in &adapters {
    ///     println!("{}: {} ({})", a.name, a.adapter_type, a.status);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_adapters(&self) -> SdkResult<Vec<BridgeAdapter>> {
        self.rpc
            .call("tenzro_listBridgeAdapters", serde_json::json!([]))
            .await
    }

    /// Gets the status of a bridge transfer
    ///
    /// # Arguments
    ///
    /// * `transfer_id` - The bridge transfer ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let bridge = client.bridge();
    /// let status = bridge.get_transfer_status("transfer-123").await?;
    /// println!("Status: {} ({} confirmations)", status.status, status.confirmations);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_transfer_status(&self, transfer_id: &str) -> SdkResult<TransferStatus> {
        self.rpc
            .call(
                "tenzro_getBridgeTransferStatus",
                serde_json::json!([{
                    "transfer_id": transfer_id,
                }]),
            )
            .await
    }

    /// Quotes a bridge fee for a specific token and route
    ///
    /// Returns a fee quote for bridging a token between two chains.
    ///
    /// # Arguments
    ///
    /// * `token` - Token symbol (e.g., "TNZO", "USDC")
    /// * `from_chain` - Source chain
    /// * `to_chain` - Destination chain
    /// * `amount` - Amount to bridge as a decimal string
    pub async fn bridge_quote(
        &self,
        token: &str,
        from_chain: &str,
        to_chain: &str,
        amount: &str,
    ) -> SdkResult<BridgeFee> {
        self.rpc
            .call(
                "tenzro_bridgeQuote",
                serde_json::json!([{
                    "token": token,
                    "from_chain": from_chain,
                    "to_chain": to_chain,
                    "amount": amount,
                }]),
            )
            .await
    }

    /// Bridges tokens with a post-bridge hook contract call
    ///
    /// Initiates a cross-chain transfer and executes an arbitrary contract call
    /// on the destination chain after the tokens arrive.
    ///
    /// # Arguments
    ///
    /// * `token` - Token symbol
    /// * `from_chain` - Source chain
    /// * `to_chain` - Destination chain
    /// * `amount` - Amount to bridge as a decimal string
    /// * `hook_target` - Contract address to call on the destination chain
    /// * `hook_calldata` - ABI-encoded calldata for the hook call (hex)
    pub async fn bridge_with_hook(
        &self,
        token: &str,
        from_chain: &str,
        to_chain: &str,
        amount: &str,
        hook_target: &str,
        hook_calldata: &str,
    ) -> SdkResult<BridgeTransfer> {
        self.rpc
            .call(
                "tenzro_bridgeWithHook",
                serde_json::json!([{
                    "token": token,
                    "from_chain": from_chain,
                    "to_chain": to_chain,
                    "amount": amount,
                    "hook_target": hook_target,
                    "hook_calldata": hook_calldata,
                }]),
            )
            .await
    }

    /// Authorizes a cross-chain bridge with mint/burn limits
    ///
    /// Registers a bridge contract address and sets daily mint and burn
    /// limits for ERC-7802 cross-chain token supply management.
    ///
    /// # Arguments
    ///
    /// * `bridge_address` - The bridge contract address to authorize
    /// * `daily_mint_limit` - Maximum tokens that can be minted per day (decimal string)
    /// * `daily_burn_limit` - Maximum tokens that can be burned per day (decimal string)
    pub async fn authorize_crosschain_bridge(
        &self,
        bridge_address: &str,
        daily_mint_limit: &str,
        daily_burn_limit: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_authorizeCrosschainBridge",
                serde_json::json!([{
                    "bridge_address": bridge_address,
                    "daily_mint_limit": daily_mint_limit,
                    "daily_burn_limit": daily_burn_limit,
                }]),
            )
            .await
    }

    /// Estimates the fee for a bridge transfer
    ///
    /// # Arguments
    ///
    /// * `from_chain` - Source chain
    /// * `to_chain` - Destination chain
    /// * `token` - Token symbol
    /// * `amount` - Amount to bridge as a decimal string
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let bridge = client.bridge();
    /// let fee = bridge.estimate_fee("tenzro", "ethereum", "TNZO", "1000000000000000000").await?;
    /// println!("Fee: {} USD", fee.total_usd);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn estimate_fee(
        &self,
        from_chain: &str,
        to_chain: &str,
        token: &str,
        amount: &str,
    ) -> SdkResult<BridgeFee> {
        self.rpc
            .call(
                "tenzro_estimateBridgeFee",
                serde_json::json!([{
                    "from_chain": from_chain,
                    "to_chain": to_chain,
                    "token": token,
                    "amount": amount,
                }]),
            )
            .await
    }
}

/// A bridge transfer result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeTransfer {
    /// Unique transfer identifier
    #[serde(default)]
    pub transfer_id: String,
    /// Transaction hash on the source chain
    #[serde(default)]
    pub tx_hash: String,
    /// Source chain
    #[serde(default)]
    pub from_chain: String,
    /// Destination chain
    #[serde(default)]
    pub to_chain: String,
    /// Transfer status (e.g., "pending", "confirmed", "delivered")
    #[serde(default)]
    pub status: String,
    /// Bridge adapter used
    #[serde(default)]
    pub adapter: String,
}

/// A bridge route between two chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeRoute {
    /// Bridge adapter name
    #[serde(default)]
    pub adapter: String,
    /// Source chain
    #[serde(default)]
    pub from_chain: String,
    /// Destination chain
    #[serde(default)]
    pub to_chain: String,
    /// Estimated fee (decimal string)
    #[serde(default)]
    pub fee: String,
    /// Estimated time in seconds
    #[serde(default)]
    pub estimated_time: u64,
}

/// A registered bridge adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeAdapter {
    /// Adapter name (e.g., "LayerZero V2")
    #[serde(default)]
    pub name: String,
    /// Adapter type (e.g., "layerzero", "ccip", "debridge", "canton")
    #[serde(default)]
    pub adapter_type: String,
    /// Supported chains
    #[serde(default)]
    pub supported_chains: Vec<String>,
    /// Adapter status (e.g., "active", "degraded")
    #[serde(default)]
    pub status: String,
}

/// Status of a bridge transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStatus {
    /// Transfer identifier
    #[serde(default)]
    pub transfer_id: String,
    /// Current status (e.g., "pending", "confirmed", "delivered", "failed").
    /// For deBridge DLN orders, `ClaimedUnlock`/`SentUnlock` map to `"delivered"`.
    #[serde(default)]
    pub status: String,
    /// Number of confirmations on the destination chain
    #[serde(default)]
    pub confirmations: u64,
}

/// Bridge fee estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeFee {
    /// Native token fee on the source chain (decimal string)
    #[serde(default)]
    pub native_fee: String,
    /// Token fee deducted from the transfer (decimal string)
    #[serde(default)]
    pub token_fee: String,
    /// Total fee in USD equivalent
    #[serde(default)]
    pub total_usd: String,
}
