//! Staking SDK for Tenzro Network
//!
//! This module provides staking and unstaking functionality for validators,
//! model providers, and TEE providers.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Staking client for TNZO staking operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let staking = client.staking();
///
/// // Stake as a validator
/// let result = staking.stake(100_000_000_000_000_000_000u128, "validator").await?;
/// println!("Staked: tx {}", result.tx_hash);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct StakingClient {
    rpc: Arc<RpcClient>,
}

impl StakingClient {
    /// Creates a new staking client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Stakes TNZO tokens for a specific role
    ///
    /// # Arguments
    ///
    /// * `amount` - Amount of TNZO to stake (in wei, 18 decimals)
    /// * `role` - Staking role: "validator", "model_provider", or "tee_provider"
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let staking = client.staking();
    /// let result = staking.stake(
    ///     100_000_000_000_000_000_000u128, // 100 TNZO
    ///     "validator",
    /// ).await?;
    /// println!("Staked successfully: {}", result.tx_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stake(&self, amount: u128, role: &str) -> SdkResult<StakeResult> {
        self.rpc
            .call(
                "tenzro_stake",
                serde_json::json!([{
                    "amount": amount.to_string(),
                    "role": role,
                }]),
            )
            .await
    }

    /// Unstakes TNZO tokens
    ///
    /// Initiates the unbonding period for staked tokens. Tokens are not
    /// immediately available; they enter a cooldown period before withdrawal.
    ///
    /// # Arguments
    ///
    /// * `amount` - Amount of TNZO to unstake (in wei, 18 decimals)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let staking = client.staking();
    /// let result = staking.unstake(
    ///     50_000_000_000_000_000_000u128, // 50 TNZO
    /// ).await?;
    /// println!("Unstake initiated: {}", result.tx_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unstake(&self, amount: u128) -> SdkResult<UnstakeResult> {
        self.rpc
            .call(
                "tenzro_unstake",
                serde_json::json!([{
                    "amount": amount.to_string(),
                }]),
            )
            .await
    }
    /// Stakes TNZO tokens (alias matching the MCP server tool name)
    ///
    /// This is a convenience alias for `stake()` using the `tenzro_stakeTokens` RPC method.
    ///
    /// # Arguments
    ///
    /// * `amount` - Amount of TNZO to stake (in wei)
    /// * `provider_type` - Provider type: "validator", "model_provider", or "tee_provider"
    pub async fn stake_tokens(&self, amount: u128, provider_type: &str) -> SdkResult<StakeResult> {
        self.rpc
            .call(
                "tenzro_stakeTokens",
                serde_json::json!([{
                    "amount": amount.to_string(),
                    "provider_type": provider_type,
                }]),
            )
            .await
    }

    /// Unstakes TNZO tokens (alias matching the MCP server tool name)
    ///
    /// This is a convenience alias for `unstake()` using the `tenzro_unstakeTokens` RPC method.
    ///
    /// # Arguments
    ///
    /// * `amount` - Amount of TNZO to unstake (in wei)
    /// * `provider_type` - Provider type: "validator", "model_provider", or "tee_provider"
    pub async fn unstake_tokens(
        &self,
        amount: u128,
        provider_type: &str,
    ) -> SdkResult<UnstakeResult> {
        self.rpc
            .call(
                "tenzro_unstakeTokens",
                serde_json::json!([{
                    "amount": amount.to_string(),
                    "provider_type": provider_type,
                }]),
            )
            .await
    }

    /// Gets the staking balance for an address
    ///
    /// Returns the total staked amount, role, and current status.
    ///
    /// # Arguments
    ///
    /// * `address` - Account address (hex)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let staking = client.staking();
    /// let balance = staking.get_staking_balance("0xaddress...").await?;
    /// println!("Staked: {} as {}", balance.total_staked, balance.role);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_staking_balance(&self, address: &str) -> SdkResult<StakingBalance> {
        self.rpc
            .call(
                "tenzro_getStakingBalance",
                serde_json::json!([{
                    "address": address,
                }]),
            )
            .await
    }

    /// Gets the accumulated staking rewards for an address
    ///
    /// # Arguments
    ///
    /// * `address` - Account address (hex)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let staking = client.staking();
    /// let rewards = staking.get_rewards("0xaddress...").await?;
    /// println!("Pending: {}, Claimed: {}", rewards.pending, rewards.claimed);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_rewards(&self, address: &str) -> SdkResult<StakingRewards> {
        self.rpc
            .call(
                "tenzro_getStakingRewards",
                serde_json::json!([{
                    "address": address,
                }]),
            )
            .await
    }

    /// Gets the unbonding entries for an address
    ///
    /// Returns a list of in-progress unstaking operations with their
    /// amounts and availability timestamps.
    ///
    /// # Arguments
    ///
    /// * `address` - Account address (hex)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let staking = client.staking();
    /// let entries = staking.get_unbonding("0xaddress...").await?;
    /// for entry in &entries {
    ///     println!("{} available at {}", entry.amount, entry.available_at);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_unbonding(&self, address: &str) -> SdkResult<Vec<UnbondingEntry>> {
        self.rpc
            .call(
                "tenzro_getUnbonding",
                serde_json::json!([{
                    "address": address,
                }]),
            )
            .await
    }
}

/// Result from a stake operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeResult {
    /// Transaction hash
    #[serde(default)]
    pub tx_hash: String,
    /// Amount staked (decimal string)
    #[serde(default)]
    pub amount: String,
    /// Role staked for
    #[serde(default)]
    pub role: String,
    /// Operation status
    #[serde(default)]
    pub status: String,
}

/// Result from an unstake operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnstakeResult {
    /// Transaction hash
    #[serde(default)]
    pub tx_hash: String,
    /// Amount being unstaked (decimal string)
    #[serde(default)]
    pub amount: String,
    /// Estimated time until tokens are available (Unix timestamp)
    #[serde(default)]
    pub available_at: u64,
    /// Operation status
    #[serde(default)]
    pub status: String,
}

/// Staking balance for an address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingBalance {
    /// Account address
    #[serde(default)]
    pub address: String,
    /// Total staked amount (decimal string)
    #[serde(default)]
    pub total_staked: String,
    /// Staking role (e.g., "validator", "model_provider", "tee_provider")
    #[serde(default)]
    pub role: String,
    /// Staking status (e.g., "active", "unbonding")
    #[serde(default)]
    pub status: String,
}

/// Staking rewards for an address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingRewards {
    /// Account address
    #[serde(default)]
    pub address: String,
    /// Pending unclaimed rewards (decimal string)
    #[serde(default)]
    pub pending: String,
    /// Total claimed rewards (decimal string)
    #[serde(default)]
    pub claimed: String,
    /// Current epoch
    #[serde(default)]
    pub current_epoch: u64,
}

/// An in-progress unbonding entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbondingEntry {
    /// Amount being unbonded (decimal string)
    #[serde(default)]
    pub amount: String,
    /// Timestamp when tokens become available (Unix seconds)
    #[serde(default)]
    pub available_at: u64,
    /// Unbonding status
    #[serde(default)]
    pub status: String,
}
