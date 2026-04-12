//! Nanopayment Batching SDK for Tenzro Network
//!
//! This module provides micropayment channel operations for efficient
//! per-token billing with batched on-chain settlement.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Nanopayment client for micropayment channel operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let nano = client.nanopayment();
///
/// // Open a payment channel
/// let channel = nano.open_channel(
///     "0xpayer...",
///     "0xpayee...",
///     10_000_000,
///     "TNZO",
/// ).await?;
/// println!("Channel: {}", channel.channel_id);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct NanopaymentClient {
    rpc: Arc<RpcClient>,
}

impl NanopaymentClient {
    /// Creates a new nanopayment client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Opens a new payment channel with a deposit
    ///
    /// # Arguments
    ///
    /// * `payer` - Payer address (hex)
    /// * `payee` - Payee address (hex)
    /// * `deposit` - Initial deposit amount
    /// * `asset` - Asset symbol (e.g., "TNZO")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let nano = client.nanopayment();
    /// let channel = nano.open_channel(
    ///     "0xpayer...",
    ///     "0xpayee...",
    ///     10_000_000,
    ///     "TNZO",
    /// ).await?;
    /// println!("Opened channel: {}", channel.channel_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open_channel(
        &self,
        payer: &str,
        payee: &str,
        deposit: u64,
        asset: &str,
    ) -> SdkResult<ChannelInfo> {
        self.rpc
            .call(
                "tenzro_openNanopaymentChannel",
                serde_json::json!([{
                    "payer": payer,
                    "payee": payee,
                    "deposit": deposit,
                    "asset": asset,
                }]),
            )
            .await
    }

    /// Sends a nanopayment within a channel
    ///
    /// The payment is batched off-chain and settled when the batch is
    /// flushed or the channel is closed.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - The payment channel ID
    /// * `amount` - Payment amount
    /// * `memo` - Optional memo for the payment
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let nano = client.nanopayment();
    /// let receipt = nano.send_nanopayment(
    ///     "channel-123",
    ///     100,
    ///     "token-42",
    /// ).await?;
    /// println!("Payment #{} in batch #{}", receipt.payment_id, receipt.batch_index);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_nanopayment(
        &self,
        channel_id: &str,
        amount: u64,
        memo: &str,
    ) -> SdkResult<NanopaymentReceipt> {
        self.rpc
            .call(
                "tenzro_sendNanopayment",
                serde_json::json!([{
                    "channel_id": channel_id,
                    "amount": amount,
                    "memo": memo,
                }]),
            )
            .await
    }

    /// Flushes the current batch and settles on-chain
    ///
    /// All pending nanopayments in the channel are aggregated into
    /// a single on-chain settlement transaction.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - The payment channel ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let nano = client.nanopayment();
    /// let settlement = nano.flush_batch("channel-123").await?;
    /// println!("Settled {} payments for {} total (tx: {})",
    ///     settlement.batch_count, settlement.total_amount, settlement.tx_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn flush_batch(&self, channel_id: &str) -> SdkResult<BatchSettlement> {
        self.rpc
            .call(
                "tenzro_flushNanopaymentBatch",
                serde_json::json!([{
                    "channel_id": channel_id,
                }]),
            )
            .await
    }

    /// Closes a payment channel and settles remaining balance
    ///
    /// # Arguments
    ///
    /// * `channel_id` - The payment channel ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let nano = client.nanopayment();
    /// let result = nano.close_channel("channel-123").await?;
    /// println!("Payer balance: {}, Payee balance: {}",
    ///     result.final_payer_balance, result.final_payee_balance);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn close_channel(&self, channel_id: &str) -> SdkResult<CloseResult> {
        self.rpc
            .call(
                "tenzro_closeNanopaymentChannel",
                serde_json::json!([{
                    "channel_id": channel_id,
                }]),
            )
            .await
    }

    /// Gets the current state of a payment channel
    ///
    /// # Arguments
    ///
    /// * `channel_id` - The payment channel ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let nano = client.nanopayment();
    /// let channel = nano.get_channel("channel-123").await?;
    /// println!("Deposit: {}, Spent: {}", channel.deposit, channel.spent);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_channel(&self, channel_id: &str) -> SdkResult<ChannelInfo> {
        self.rpc
            .call(
                "tenzro_getNanopaymentChannel",
                serde_json::json!([{
                    "channel_id": channel_id,
                }]),
            )
            .await
    }

    /// Lists all payment channels for an address
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
    /// let nano = client.nanopayment();
    /// let channels = nano.list_channels("0xaddress...").await?;
    /// println!("Found {} channels", channels.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_channels(&self, address: &str) -> SdkResult<Vec<ChannelInfo>> {
        self.rpc
            .call(
                "tenzro_listNanopaymentChannels",
                serde_json::json!([{
                    "address": address,
                }]),
            )
            .await
    }
}

/// Payment channel information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    /// Unique channel identifier
    #[serde(default)]
    pub channel_id: String,
    /// Payer address
    #[serde(default)]
    pub payer: String,
    /// Payee address
    #[serde(default)]
    pub payee: String,
    /// Initial deposit amount
    #[serde(default)]
    pub deposit: u64,
    /// Total amount spent
    #[serde(default)]
    pub spent: u64,
    /// Number of payments pending in the current batch
    #[serde(default)]
    pub pending_batch: u32,
    /// Channel status (e.g., "open", "closed", "disputed")
    #[serde(default)]
    pub status: String,
}

/// Receipt for a single nanopayment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NanopaymentReceipt {
    /// Payment identifier
    #[serde(default)]
    pub payment_id: String,
    /// Channel this payment belongs to
    #[serde(default)]
    pub channel_id: String,
    /// Payment amount
    #[serde(default)]
    pub amount: u64,
    /// Index within the current batch
    #[serde(default)]
    pub batch_index: u32,
}

/// Result from flushing a batch of nanopayments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSettlement {
    /// Channel identifier
    #[serde(default)]
    pub channel_id: String,
    /// Number of payments in the settled batch
    #[serde(default)]
    pub batch_count: u32,
    /// Total amount settled
    #[serde(default)]
    pub total_amount: u64,
    /// On-chain transaction hash
    #[serde(default)]
    pub tx_hash: String,
    /// Settlement fee deducted
    #[serde(default)]
    pub settlement_fee: u64,
}

/// Result from closing a payment channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseResult {
    /// Channel identifier
    #[serde(default)]
    pub channel_id: String,
    /// Final balance returned to the payer
    #[serde(default)]
    pub final_payer_balance: u64,
    /// Final balance sent to the payee
    #[serde(default)]
    pub final_payee_balance: u64,
    /// On-chain transaction hash
    #[serde(default)]
    pub tx_hash: String,
}
