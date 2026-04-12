//! ERC-7802 Cross-Chain Token Interface SDK for Tenzro Network
//!
//! This module provides the ERC-7802 crosschain mint and burn interface
//! for managing token supply across multiple chains.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// ERC-7802 client for cross-chain token mint and burn operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let erc7802 = client.erc7802();
///
/// // Get cross-chain supply for a token
/// let supply = erc7802.get_cross_chain_supply("TNZO").await?;
/// println!("Local supply: {}, Total: {}", supply.local_supply, supply.total_cross_chain);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Erc7802Client {
    rpc: Arc<RpcClient>,
}

impl Erc7802Client {
    /// Creates a new ERC-7802 client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Performs a cross-chain mint of tokens
    ///
    /// Mints tokens on the local chain in response to a verified
    /// burn on the source chain, following the ERC-7802 standard.
    ///
    /// # Arguments
    ///
    /// * `token` - Token symbol or address
    /// * `recipient` - Recipient address on the local chain
    /// * `amount` - Amount to mint as a decimal string
    /// * `source_chain` - Chain where the corresponding burn occurred
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let erc7802 = client.erc7802();
    /// let result = erc7802.crosschain_mint(
    ///     "TNZO",
    ///     "0xrecipient...",
    ///     "1000000000000000000",
    ///     "ethereum",
    /// ).await?;
    /// println!("Minted on tx: {}", result.tx_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn crosschain_mint(
        &self,
        token: &str,
        recipient: &str,
        amount: &str,
        source_chain: &str,
    ) -> SdkResult<MintResult> {
        self.rpc
            .call(
                "tenzro_erc7802CrosschainMint",
                serde_json::json!([{
                    "token": token,
                    "recipient": recipient,
                    "amount": amount,
                    "source_chain": source_chain,
                }]),
            )
            .await
    }

    /// Performs a cross-chain burn of tokens
    ///
    /// Burns tokens on the local chain, enabling a corresponding mint
    /// on the target chain per the ERC-7802 standard.
    ///
    /// # Arguments
    ///
    /// * `token` - Token symbol or address
    /// * `from` - Address to burn tokens from
    /// * `amount` - Amount to burn as a decimal string
    /// * `target_chain` - Chain where the corresponding mint will occur
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let erc7802 = client.erc7802();
    /// let result = erc7802.crosschain_burn(
    ///     "TNZO",
    ///     "0xsender...",
    ///     "1000000000000000000",
    ///     "solana",
    /// ).await?;
    /// println!("Burned on tx: {}", result.tx_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn crosschain_burn(
        &self,
        token: &str,
        from: &str,
        amount: &str,
        target_chain: &str,
    ) -> SdkResult<BurnResult> {
        self.rpc
            .call(
                "tenzro_erc7802CrosschainBurn",
                serde_json::json!([{
                    "token": token,
                    "from": from,
                    "amount": amount,
                    "target_chain": target_chain,
                }]),
            )
            .await
    }

    /// Gets the cross-chain supply distribution for a token
    ///
    /// Returns the local supply, total cross-chain supply, and per-chain
    /// breakdown.
    ///
    /// # Arguments
    ///
    /// * `token` - Token symbol or address
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let erc7802 = client.erc7802();
    /// let supply = erc7802.get_cross_chain_supply("TNZO").await?;
    /// for (chain, amount) in &supply.chain_supplies {
    ///     println!("{}: {}", chain, amount);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_cross_chain_supply(&self, token: &str) -> SdkResult<CrossChainSupply> {
        self.rpc
            .call(
                "tenzro_erc7802GetCrossChainSupply",
                serde_json::json!([{
                    "token": token,
                }]),
            )
            .await
    }
}

/// Result from a cross-chain mint operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintResult {
    /// On-chain transaction hash
    #[serde(default)]
    pub tx_hash: String,
    /// Token minted
    #[serde(default)]
    pub token: String,
    /// Recipient address
    #[serde(default)]
    pub recipient: String,
    /// Amount minted (decimal string)
    #[serde(default)]
    pub amount: String,
    /// Source chain of the burn
    #[serde(default)]
    pub source_chain: String,
}

/// Result from a cross-chain burn operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnResult {
    /// On-chain transaction hash
    #[serde(default)]
    pub tx_hash: String,
    /// Token burned
    #[serde(default)]
    pub token: String,
    /// Address tokens were burned from
    #[serde(default)]
    pub from: String,
    /// Amount burned (decimal string)
    #[serde(default)]
    pub amount: String,
    /// Target chain for the corresponding mint
    #[serde(default)]
    pub target_chain: String,
}

/// Cross-chain supply distribution for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainSupply {
    /// Token symbol or address
    #[serde(default)]
    pub token: String,
    /// Supply on the local (Tenzro) chain (decimal string)
    #[serde(default)]
    pub local_supply: String,
    /// Total supply across all chains (decimal string)
    #[serde(default)]
    pub total_cross_chain: String,
    /// Per-chain supply breakdown (chain name -> decimal string)
    #[serde(default)]
    pub chain_supplies: HashMap<String, String>,
}
