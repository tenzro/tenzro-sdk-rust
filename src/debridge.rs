//! deBridge DLN SDK for Tenzro Network
//!
//! This module provides cross-chain swap and bridging functionality via the
//! deBridge Liquidity Network (DLN). DLN uses an intent-based model where
//! makers fill cross-chain orders, enabling fast and capital-efficient transfers.
//!
//! Order status is tracked via `stats-api.dln.trade`. Status mapping:
//! `ClaimedUnlock`/`SentUnlock` map to `Filled` (not `Created`).

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// deBridge DLN client for cross-chain swaps and bridging
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let debridge = client.debridge();
///
/// // List supported chains
/// let chains = debridge.get_chains().await?;
/// println!("Supported chains: {}", chains.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct DebridgeClient {
    rpc: Arc<RpcClient>,
}

impl DebridgeClient {
    /// Creates a new deBridge client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Searches for tokens available on deBridge
    ///
    /// # Arguments
    ///
    /// * `query` - Token name or symbol to search for (e.g., "USDC", "ETH")
    /// * `chain_id` - Optional chain ID to filter results to a specific chain
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let debridge = client.debridge();
    /// let tokens = debridge.search_tokens("USDC", Some(1)).await?;
    /// for token in &tokens {
    ///     println!("{}: {} (chain {})", token.symbol, token.address, token.chain_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_tokens(
        &self,
        query: &str,
        chain_id: Option<u64>,
    ) -> SdkResult<Vec<DebridgeTokenInfo>> {
        let mut params = serde_json::json!({
            "query": query,
        });

        if let Some(cid) = chain_id {
            params["chain_id"] = serde_json::json!(cid);
        }

        self.rpc
            .call("tenzro_debridgeSearchTokens", serde_json::json!([params]))
            .await
    }

    /// Gets the list of chains supported by deBridge DLN
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let debridge = client.debridge();
    /// let chains = debridge.get_chains().await?;
    /// for chain in &chains {
    ///     println!("{}: {} (chain_id: {})", chain.name, chain.chain_type, chain.chain_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_chains(&self) -> SdkResult<Vec<DebridgeChain>> {
        self.rpc
            .call("tenzro_debridgeGetChains", serde_json::json!([]))
            .await
    }

    /// Gets deBridge integration instructions and supported parameters
    ///
    /// Returns protocol-level information about DLN order creation,
    /// supported token pairs, and fee structures.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let debridge = client.debridge();
    /// let info = debridge.get_instructions().await?;
    /// println!("Protocol version: {}", info.protocol_version);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_instructions(&self) -> SdkResult<DebridgeInstructions> {
        self.rpc
            .call("tenzro_debridgeGetInstructions", serde_json::json!([]))
            .await
    }

    /// Creates a cross-chain swap transaction via deBridge DLN
    ///
    /// Builds a DLN order-creation transaction that can be signed and submitted.
    /// The order is filled by DLN makers on the destination chain.
    ///
    /// # Arguments
    ///
    /// * `src_chain` - Source chain ID (e.g., 1 for Ethereum, 56 for BSC)
    /// * `dst_chain` - Destination chain ID
    /// * `src_token` - Source token address (hex)
    /// * `dst_token` - Destination token address (hex)
    /// * `amount` - Amount of source token to swap (decimal string)
    /// * `recipient` - Recipient address on the destination chain (hex)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let debridge = client.debridge();
    /// let tx = debridge.create_tx(
    ///     1,           // Ethereum
    ///     56,          // BSC
    ///     "0xA0b8...", // USDC on Ethereum
    ///     "0x8AC7...", // USDC on BSC
    ///     "1000000000", // 1000 USDC (6 decimals)
    ///     "0xrecipient...",
    /// ).await?;
    /// println!("Order ID: {}", tx.order_id);
    /// println!("Estimated output: {}", tx.estimated_output);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_tx(
        &self,
        src_chain: u64,
        dst_chain: u64,
        src_token: &str,
        dst_token: &str,
        amount: &str,
        recipient: &str,
    ) -> SdkResult<DebridgeTxData> {
        self.rpc
            .call(
                "tenzro_debridgeCreateTx",
                serde_json::json!([{
                    "src_chain": src_chain,
                    "dst_chain": dst_chain,
                    "src_token": src_token,
                    "dst_token": dst_token,
                    "amount": amount,
                    "recipient": recipient,
                }]),
            )
            .await
    }

    /// Performs a same-chain token swap via deBridge's DEX aggregation
    ///
    /// Uses deBridge's integrated DEX aggregator to find the best swap
    /// route on a single chain without cross-chain bridging.
    ///
    /// # Arguments
    ///
    /// * `chain_id` - Chain ID to execute the swap on
    /// * `token_in` - Input token address (hex)
    /// * `token_out` - Output token address (hex)
    /// * `amount` - Amount of input token (decimal string)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let debridge = client.debridge();
    /// let result = debridge.same_chain_swap(
    ///     1,           // Ethereum
    ///     "0xA0b8...", // USDC
    ///     "0xC02a...", // WETH
    ///     "1000000000", // 1000 USDC
    /// ).await?;
    /// println!("Output amount: {}", result.output_amount);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn same_chain_swap(
        &self,
        chain_id: u64,
        token_in: &str,
        token_out: &str,
        amount: &str,
    ) -> SdkResult<DebridgeSwapResult> {
        self.rpc
            .call(
                "tenzro_debridgeSameChainSwap",
                serde_json::json!([{
                    "chain_id": chain_id,
                    "token_in": token_in,
                    "token_out": token_out,
                    "amount": amount,
                }]),
            )
            .await
    }
}

/// Token information from deBridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebridgeTokenInfo {
    /// Token contract address (hex)
    #[serde(default)]
    pub address: String,
    /// Token symbol
    #[serde(default)]
    pub symbol: String,
    /// Token name
    #[serde(default)]
    pub name: String,
    /// Number of decimals
    #[serde(default)]
    pub decimals: u8,
    /// Chain ID the token is on
    #[serde(default)]
    pub chain_id: u64,
}

/// A chain supported by deBridge DLN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebridgeChain {
    /// Chain ID
    #[serde(default)]
    pub chain_id: u64,
    /// Chain name (e.g., "Ethereum", "BSC", "Polygon")
    #[serde(default)]
    pub name: String,
    /// Chain type (e.g., "evm", "solana")
    #[serde(default)]
    pub chain_type: String,
    /// Whether the chain is currently active
    #[serde(default)]
    pub active: bool,
}

/// deBridge integration instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebridgeInstructions {
    /// Protocol version
    #[serde(default)]
    pub protocol_version: String,
    /// DLN contract address
    #[serde(default)]
    pub dln_address: String,
    /// Supported token pair count
    #[serde(default)]
    pub supported_pairs: u64,
    /// Fee structure description
    #[serde(default)]
    pub fee_info: String,
    /// Additional instructions or notes
    #[serde(default)]
    pub notes: String,
}

/// Transaction data for a deBridge cross-chain swap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebridgeTxData {
    /// DLN order identifier
    #[serde(default)]
    pub order_id: String,
    /// Source chain ID
    #[serde(default)]
    pub src_chain: u64,
    /// Destination chain ID
    #[serde(default)]
    pub dst_chain: u64,
    /// Estimated output amount on the destination chain (decimal string)
    #[serde(default)]
    pub estimated_output: String,
    /// Transaction calldata to sign and submit (hex)
    #[serde(default)]
    pub tx_data: String,
    /// Contract address to send the transaction to (hex)
    #[serde(default)]
    pub to: String,
    /// Value to send with the transaction (decimal string, for native token)
    #[serde(default)]
    pub value: String,
    /// Estimated fee (decimal string)
    #[serde(default)]
    pub fee: String,
    /// Order status
    #[serde(default)]
    pub status: String,
}

/// Result of a same-chain swap via deBridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebridgeSwapResult {
    /// Output token amount (decimal string)
    #[serde(default)]
    pub output_amount: String,
    /// Transaction calldata to sign and submit (hex)
    #[serde(default)]
    pub tx_data: String,
    /// Contract address to send the transaction to (hex)
    #[serde(default)]
    pub to: String,
    /// Value to send with the transaction (decimal string)
    #[serde(default)]
    pub value: String,
    /// Estimated gas cost
    #[serde(default)]
    pub estimated_gas: String,
    /// Operation status
    #[serde(default)]
    pub status: String,
}
