//! Payment SDK for Tenzro Network
//!
//! This module provides payment protocol functionality for MPP, x402,
//! and direct settlement.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Payment client for MPP/x402 payment operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let payment_client = client.payment();
///
/// // Get gateway info
/// let info = payment_client.gateway_info().await?;
/// println!("Protocols: {:?}", info.protocols);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct PaymentClient {
    rpc: Arc<RpcClient>,
}

impl PaymentClient {
    /// Creates a new payment client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Creates a payment challenge for a resource (server-side)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let payment_client = client.payment();
    /// let challenge = payment_client.create_challenge(
    ///     "/api/inference/gemma4-9b",
    ///     1000,
    ///     "USDC",
    ///     "mpp",
    /// ).await?;
    /// println!("Challenge ID: {}", challenge.challenge_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_challenge(
        &self,
        resource: &str,
        amount: u64,
        asset: &str,
        protocol: &str,
    ) -> SdkResult<PaymentChallenge> {
        self.rpc
            .call(
                "tenzro_createPaymentChallenge",
                serde_json::json!([{
                    "resource": resource,
                    "amount": amount,
                    "asset": asset,
                    "protocol": protocol,
                }]),
            )
            .await
    }

    /// Pays for a resource using MPP (auto-402 flow)
    ///
    /// The MPP flow automatically handles the HTTP 402 challenge/credential exchange.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let payment = client.payment();
    /// let receipt = payment.pay_mpp(
    ///     "https://api.tenzro.network/inference",
    ///     Some("did:tenzro:human:abc123"),
    /// ).await?;
    /// println!("Paid: {} {}", receipt.amount, receipt.asset);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pay_mpp(
        &self,
        url: &str,
        payer_did: Option<&str>,
    ) -> SdkResult<PaymentReceipt> {
        self.rpc
            .call(
                "tenzro_payMpp",
                serde_json::json!([{
                    "url": url,
                    "payer_did": payer_did,
                }]),
            )
            .await
    }

    /// Pays for a resource using x402
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let payment = client.payment();
    /// let receipt = payment.pay_x402(
    ///     "https://api.tenzro.network/inference",
    ///     Some("did:tenzro:human:abc123"),
    /// ).await?;
    /// println!("Receipt ID: {}", receipt.receipt_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pay_x402(
        &self,
        url: &str,
        payer_did: Option<&str>,
    ) -> SdkResult<PaymentReceipt> {
        self.rpc
            .call(
                "tenzro_payX402",
                serde_json::json!([{
                    "url": url,
                    "payer_did": payer_did,
                }]),
            )
            .await
    }

    /// Lists active payment sessions
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let payment = client.payment();
    /// let sessions = payment.list_sessions().await?;
    /// println!("Active sessions: {}", sessions.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_sessions(&self) -> SdkResult<Vec<PaymentSession>> {
        self.rpc
            .call(
                "tenzro_listPaymentSessions",
                serde_json::json!([{ "include_inactive": false }]),
            )
            .await
    }

    /// Gets a payment receipt by ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let payment = client.payment();
    /// let receipt = payment.get_receipt("receipt-123").await?;
    /// println!("Amount: {}", receipt.amount);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_receipt(&self, receipt_id: &str) -> SdkResult<PaymentReceipt> {
        self.rpc
            .call(
                "tenzro_getPaymentReceipt",
                serde_json::json!([{ "receipt_id": receipt_id }]),
            )
            .await
    }

    /// Pays via Visa TAP
    pub async fn pay_visa_tap(&self, credential: serde_json::Value) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_payVisaTap", serde_json::json!(credential))
            .await
    }

    /// Pays via Mastercard Agent Pay
    pub async fn pay_mastercard(&self, credential: serde_json::Value) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_payMastercard", serde_json::json!(credential))
            .await
    }

    /// Pays for a resource using the AP2 agentic payment protocol
    ///
    /// AP2 combines session-based authorization with automatic settlement,
    /// designed for agent-to-provider payments.
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the paying agent
    /// * `url` - Resource URL to pay for
    /// * `amount` - Payment amount
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let payment = client.payment();
    /// let receipt = payment.pay_ap2(
    ///     "did:tenzro:machine:agent-1",
    ///     "https://api.tenzro.network/inference",
    ///     100_000,
    /// ).await?;
    /// println!("AP2 receipt: {}", receipt.receipt_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pay_ap2(
        &self,
        agent_did: &str,
        url: &str,
        amount: u64,
    ) -> SdkResult<PaymentReceipt> {
        self.rpc
            .call(
                "tenzro_payAp2",
                serde_json::json!([{
                    "agent_did": agent_did,
                    "url": url,
                    "amount": amount,
                }]),
            )
            .await
    }

    /// Verifies a payment credential against a challenge and settles on-chain
    ///
    /// # Arguments
    ///
    /// * `challenge_id` - The challenge ID to verify against
    /// * `params` - Verification parameters (protocol-specific)
    pub async fn verify_payment(
        &self,
        challenge_id: &str,
        params: serde_json::Value,
    ) -> SdkResult<PaymentReceipt> {
        let mut p = params;
        p["challenge_id"] = serde_json::json!(challenge_id);
        self.rpc
            .call("tenzro_verifyPayment", serde_json::json!([p]))
            .await
    }

    /// Settles a payment between two parties on-chain
    ///
    /// # Arguments
    ///
    /// * `from` - Payer address (hex)
    /// * `to` - Payee address (hex)
    /// * `amount` - Payment amount (decimal string)
    /// * `service_type` - Type of service being paid for (e.g., "inference", "tee")
    pub async fn settle_payment(
        &self,
        from: &str,
        to: &str,
        amount: &str,
        service_type: &str,
    ) -> SdkResult<PaymentReceipt> {
        self.rpc
            .call(
                "tenzro_settlePayment",
                serde_json::json!([{
                    "from": from,
                    "to": to,
                    "amount": amount,
                    "service_type": service_type,
                }]),
            )
            .await
    }

    /// Lists supported payment protocols
    ///
    /// Returns the list of available payment protocols (MPP, x402, native).
    pub async fn list_payment_protocols(&self) -> SdkResult<Vec<PaymentProtocolInfo>> {
        self.rpc
            .call("tenzro_listPaymentProtocols", serde_json::json!([]))
            .await
    }

    /// Gets payment gateway information
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let payment = client.payment();
    /// let info = payment.gateway_info().await?;
    /// println!("Status: {}, Protocols: {:?}", info.status, info.protocols);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn gateway_info(&self) -> SdkResult<GatewayInfo> {
        self.rpc
            .call("tenzro_paymentGatewayInfo", serde_json::json!([]))
            .await
    }
}

/// A payment challenge (402 response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentChallenge {
    /// Challenge ID
    #[serde(default)]
    pub challenge_id: String,
    /// Protocol (mpp or x402)
    #[serde(default)]
    pub protocol: String,
    /// Resource being accessed
    #[serde(default)]
    pub resource: String,
    /// Required payment amount
    #[serde(default)]
    pub amount: u64,
    /// Asset to pay in
    #[serde(default)]
    pub asset: String,
}

/// A payment receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentReceipt {
    /// Receipt ID
    #[serde(default)]
    pub receipt_id: String,
    /// Protocol used (mpp or x402)
    #[serde(default)]
    pub protocol: String,
    /// Amount paid
    #[serde(default)]
    pub amount: u64,
    /// Asset used
    #[serde(default)]
    pub asset: String,
    /// MPP session ID (if applicable)
    #[serde(default)]
    pub session_id: Option<String>,
}

/// An active payment session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentSession {
    /// Session ID
    #[serde(default)]
    pub session_id: String,
    /// Protocol (mpp)
    #[serde(default)]
    pub protocol: String,
    /// Resource being accessed
    #[serde(default)]
    pub resource: String,
    /// Total spent in this session
    #[serde(default)]
    pub total_spent: u64,
    /// Asset
    #[serde(default)]
    pub asset: String,
    /// Whether the session is active
    #[serde(default)]
    pub active: bool,
}

/// Information about a supported payment protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentProtocolInfo {
    /// Protocol name (e.g., "mpp", "x402", "native")
    #[serde(default)]
    pub name: String,
    /// Protocol description
    #[serde(default)]
    pub description: String,
    /// Whether the protocol is currently available
    #[serde(default)]
    pub available: bool,
    /// Supported assets for this protocol
    #[serde(default)]
    pub supported_assets: Vec<String>,
}

/// Payment gateway information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayInfo {
    /// Gateway status
    #[serde(default)]
    pub status: String,
    /// Supported protocols
    #[serde(default)]
    pub protocols: Vec<String>,
    /// Supported assets
    #[serde(default)]
    pub supported_assets: Vec<String>,
}
