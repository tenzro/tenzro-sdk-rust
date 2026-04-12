//! Settlement SDK for Tenzro Network
//!
//! This module provides payment settlement and escrow functionality.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Settlement client for payment operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let settlement = client.settlement();
///
/// // Get settlement by receipt ID
/// let settlement = settlement.get_settlement("receipt-123").await?;
/// println!("Settlement: {:?}", settlement);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct SettlementClient {
    rpc: Arc<RpcClient>,
}

impl SettlementClient {
    /// Creates a new settlement client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Submits a settlement request
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig, SettlementRequest};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let settlement_client = client.settlement();
    ///
    /// let request = SettlementRequest {
    ///     request_id: "req-123".to_string(),
    ///     provider: "0xprovider...".to_string(),
    ///     customer: "0xcustomer...".to_string(),
    ///     amount: 1000000,
    ///     asset: "TNZO".to_string(),
    /// };
    ///
    /// let response = settlement_client.settle(request).await?;
    /// println!("Settlement receipt: {}", response.receipt_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn settle(&self, request: SettlementRequest) -> SdkResult<SettleResponse> {
        self.rpc
            .call(
                "tenzro_settle",
                serde_json::json!([{
                    "request_id": request.request_id,
                    "provider": request.provider,
                    "customer": request.customer,
                    "amount": request.amount,
                    "asset": request.asset,
                }]),
            )
            .await
    }

    /// Gets a settlement by receipt ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let settlement = client.settlement();
    /// let result = settlement.get_settlement("receipt-123").await?;
    /// println!("Settlement: {:?}", result);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_settlement(&self, receipt_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getSettlement",
                serde_json::json!([receipt_id]),
            )
            .await
    }

    /// Creates an escrow for conditional payment
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let settlement = client.settlement();
    ///
    /// let escrow_id = settlement.create_escrow(
    ///     "0xpayee...",
    ///     1000000,
    ///     "TNZO",
    ///     "both_signatures",
    /// ).await?;
    /// println!("Escrow created: {}", escrow_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_escrow(
        &self,
        payee: &str,
        amount: u64,
        asset: &str,
        conditions: &str,
    ) -> SdkResult<String> {
        self.rpc
            .call(
                "tenzro_createEscrow",
                serde_json::json!([{
                    "payee": payee,
                    "amount": amount,
                    "asset": asset,
                    "conditions": conditions,
                }]),
            )
            .await
    }

    /// Releases funds from escrow
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let settlement = client.settlement();
    ///
    /// let proof = vec![0u8; 32]; // ZK proof bytes
    /// let tx_hash = settlement.release_escrow("escrow-123", proof).await?;
    /// println!("Release transaction: {}", tx_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn release_escrow(
        &self,
        escrow_id: &str,
        proof: Vec<u8>,
    ) -> SdkResult<String> {
        self.rpc
            .call(
                "tenzro_releaseEscrow",
                serde_json::json!([{
                    "escrow_id": escrow_id,
                    "proof": hex::encode(proof),
                }]),
            )
            .await
    }

    /// Opens a micropayment channel
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let settlement = client.settlement();
    ///
    /// let channel_id = settlement.open_payment_channel(
    ///     "0xpayee...",
    ///     10000000,
    /// ).await?;
    /// println!("Channel opened: {}", channel_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open_payment_channel(
        &self,
        payee: &str,
        deposit: u64,
    ) -> SdkResult<String> {
        self.rpc
            .call(
                "tenzro_openPaymentChannel",
                serde_json::json!([{
                    "payee": payee,
                    "deposit": deposit,
                }]),
            )
            .await
    }

    /// Closes a micropayment channel
    pub async fn close_payment_channel(&self, channel_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_closePaymentChannel",
                serde_json::json!({"channel_id": channel_id}),
            )
            .await
    }
}

/// Settlement request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRequest {
    /// Unique request ID
    pub request_id: String,
    /// Provider address
    pub provider: String,
    /// Customer address
    pub customer: String,
    /// Settlement amount
    pub amount: u64,
    /// Asset symbol (e.g., "TNZO", "USDC")
    pub asset: String,
}

/// Settlement response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettleResponse {
    /// Receipt ID
    #[serde(default)]
    pub receipt_id: String,
    /// Transaction hash
    #[serde(default)]
    pub tx_hash: String,
    /// Settlement status
    #[serde(default)]
    pub status: String,
}
