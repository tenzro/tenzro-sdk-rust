//! AP2 Agentic Payment Protocol SDK for Tenzro Network
//!
//! This module provides the AP2 (Agentic Payment Protocol) for session-based
//! agent-to-provider payment flows via the A2A server.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// AP2 client for agentic payment protocol operations
///
/// Manages payment sessions between agents and providers, including
/// authorization, execution, and cancellation of payments.
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let ap2 = client.ap2();
///
/// // Create an AP2 payment session
/// let session = ap2.create_session(
///     "did:tenzro:machine:agent-1",
///     "did:tenzro:machine:provider-1",
///     "inference",
///     1_000_000,
///     "TNZO",
/// ).await?;
/// println!("Session: {}", session.session_id);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Ap2Client {
    rpc: Arc<RpcClient>,
}

impl Ap2Client {
    /// Creates a new AP2 client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Creates an AP2 payment session between an agent and a provider
    ///
    /// The session establishes a spending cap for the agent to authorize
    /// individual payments against.
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the paying agent
    /// * `provider_did` - DID of the service provider
    /// * `service` - Service type (e.g., "inference", "tee", "storage")
    /// * `max_amount` - Maximum amount the session can spend
    /// * `asset` - Asset symbol (e.g., "TNZO", "USDC")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let ap2 = client.ap2();
    /// let session = ap2.create_session(
    ///     "did:tenzro:machine:agent-1",
    ///     "did:tenzro:machine:provider-1",
    ///     "inference",
    ///     5_000_000,
    ///     "TNZO",
    /// ).await?;
    /// println!("Session created: {}", session.session_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_session(
        &self,
        agent_did: &str,
        provider_did: &str,
        service: &str,
        max_amount: u64,
        asset: &str,
    ) -> SdkResult<Ap2Session> {
        self.rpc
            .call(
                "tenzro_ap2CreateSession",
                serde_json::json!([{
                    "agent_did": agent_did,
                    "provider_did": provider_did,
                    "service": service,
                    "max_amount": max_amount,
                    "asset": asset,
                }]),
            )
            .await
    }

    /// Authorizes a payment within an existing AP2 session
    ///
    /// Creates an authorization for a specific amount that can then
    /// be executed. Authorizations expire if not executed promptly.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The AP2 session ID
    /// * `amount` - Amount to authorize
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let ap2 = client.ap2();
    /// let auth = ap2.authorize_payment("session-123", 100_000).await?;
    /// println!("Authorization: {}", auth.authorization_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn authorize_payment(
        &self,
        session_id: &str,
        amount: u64,
    ) -> SdkResult<Ap2Authorization> {
        self.rpc
            .call(
                "tenzro_ap2AuthorizePayment",
                serde_json::json!([{
                    "session_id": session_id,
                    "amount": amount,
                }]),
            )
            .await
    }

    /// Executes a previously authorized payment
    ///
    /// Settles the authorized payment on-chain and returns a receipt.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The AP2 session ID
    /// * `authorization_id` - The authorization to execute
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let ap2 = client.ap2();
    /// let receipt = ap2.execute_payment("session-123", "auth-456").await?;
    /// println!("Payment settled: {}", receipt.receipt_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_payment(
        &self,
        session_id: &str,
        authorization_id: &str,
    ) -> SdkResult<crate::payment::PaymentReceipt> {
        self.rpc
            .call(
                "tenzro_ap2ExecutePayment",
                serde_json::json!([{
                    "session_id": session_id,
                    "authorization_id": authorization_id,
                }]),
            )
            .await
    }

    /// Cancels an AP2 session and refunds any unspent funds
    ///
    /// # Arguments
    ///
    /// * `session_id` - The AP2 session ID to cancel
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let ap2 = client.ap2();
    /// let result = ap2.cancel_session("session-123").await?;
    /// println!("Refunded: {}", result.refunded);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel_session(&self, session_id: &str) -> SdkResult<CancelResult> {
        self.rpc
            .call(
                "tenzro_ap2CancelSession",
                serde_json::json!([{
                    "session_id": session_id,
                }]),
            )
            .await
    }

    /// Gets the current state of an AP2 session
    ///
    /// # Arguments
    ///
    /// * `session_id` - The AP2 session ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let ap2 = client.ap2();
    /// let session = ap2.get_session("session-123").await?;
    /// println!("Spent: {} / {}", session.spent, session.max_amount);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_session(&self, session_id: &str) -> SdkResult<Ap2Session> {
        self.rpc
            .call(
                "tenzro_ap2GetSession",
                serde_json::json!([{
                    "session_id": session_id,
                }]),
            )
            .await
    }

    /// Lists all AP2 sessions for an agent
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the agent
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let ap2 = client.ap2();
    /// let sessions = ap2.list_agent_sessions("did:tenzro:machine:agent-1").await?;
    /// println!("Agent has {} sessions", sessions.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_agent_sessions(&self, agent_did: &str) -> SdkResult<Vec<Ap2Session>> {
        self.rpc
            .call(
                "tenzro_ap2ListSessions",
                serde_json::json!([{
                    "agent_did": agent_did,
                }]),
            )
            .await
    }

    // ─── AP2 Mandate Verification (Google AP2 spec) ──────────────────────

    /// Verifies a single AP2 mandate (Verifiable Digital Credential)
    ///
    /// Checks the VDC proof, issuer, and schema for Intent, Cart, or Payment
    /// mandates per Google's AP2 specification.
    ///
    /// # Arguments
    ///
    /// * `vdc` - The full JSON-LD VC envelope with proof
    pub async fn verify_mandate(&self, vdc: serde_json::Value) -> SdkResult<Ap2MandateVerification> {
        self.rpc
            .call(
                "tenzro_ap2VerifyMandate",
                serde_json::json!([{ "vdc": vdc }]),
            )
            .await
    }

    /// Validates an AP2 Intent+Cart mandate pair for consistency
    ///
    /// Ensures the cart references the intent, amounts/items match the intent's
    /// constraints, and both VDCs verify.
    pub async fn validate_mandate_pair(
        &self,
        intent_vdc: serde_json::Value,
        cart_vdc: serde_json::Value,
    ) -> SdkResult<Ap2MandatePairValidation> {
        self.rpc
            .call(
                "tenzro_ap2ValidateMandatePair",
                serde_json::json!([{
                    "intent_vdc": intent_vdc,
                    "cart_vdc": cart_vdc,
                }]),
            )
            .await
    }

    /// Returns AP2 protocol metadata (version, supported mandate types, VC formats)
    pub async fn protocol_info(&self) -> SdkResult<Ap2ProtocolInfo> {
        self.rpc
            .call("tenzro_ap2ProtocolInfo", serde_json::json!([]))
            .await
    }
}

/// An AP2 payment session between an agent and provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ap2Session {
    /// Unique session identifier
    #[serde(default)]
    pub session_id: String,
    /// DID of the paying agent
    #[serde(default)]
    pub agent_did: String,
    /// DID of the service provider
    #[serde(default)]
    pub provider_did: String,
    /// Service type (e.g., "inference", "tee")
    #[serde(default)]
    pub service: String,
    /// Maximum amount the session can spend
    #[serde(default)]
    pub max_amount: u64,
    /// Total amount spent so far
    #[serde(default)]
    pub spent: u64,
    /// Asset symbol (e.g., "TNZO")
    #[serde(default)]
    pub asset: String,
    /// Session status (e.g., "active", "cancelled", "completed")
    #[serde(default)]
    pub status: String,
    /// Session creation timestamp (Unix seconds)
    #[serde(default)]
    pub created_at: u64,
}

/// An authorization for a payment within an AP2 session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ap2Authorization {
    /// Unique authorization identifier
    #[serde(default)]
    pub authorization_id: String,
    /// Session this authorization belongs to
    #[serde(default)]
    pub session_id: String,
    /// Authorized amount
    #[serde(default)]
    pub amount: u64,
    /// Asset symbol
    #[serde(default)]
    pub asset: String,
    /// Expiration timestamp (Unix seconds)
    #[serde(default)]
    pub expires_at: u64,
}

/// Result from cancelling an AP2 session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelResult {
    /// Session that was cancelled
    #[serde(default)]
    pub session_id: String,
    /// Amount refunded to the agent
    #[serde(default)]
    pub refunded: u64,
}

/// Result of verifying a single AP2 mandate (Intent / Cart / Payment VDC).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ap2MandateVerification {
    /// Whether the VDC proof is valid
    #[serde(default)]
    pub valid: bool,
    /// Mandate type: "Intent" | "Cart" | "Payment"
    #[serde(default)]
    pub mandate_type: String,
    /// Issuer DID
    #[serde(default)]
    pub issuer: String,
    /// Subject DID (agent/merchant)
    #[serde(default)]
    pub subject: String,
    /// Expiration timestamp (Unix seconds)
    #[serde(default)]
    pub expires_at: u64,
    /// Reason for failure if `valid` is false
    #[serde(default)]
    pub error: Option<String>,
}

/// Result of validating an Intent+Cart mandate pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ap2MandatePairValidation {
    /// Whether the pair is mutually consistent and both VDCs verify
    #[serde(default)]
    pub valid: bool,
    /// Verification result for the intent mandate
    #[serde(default)]
    pub intent: Option<Ap2MandateVerification>,
    /// Verification result for the cart mandate
    #[serde(default)]
    pub cart: Option<Ap2MandateVerification>,
    /// Reason for failure if `valid` is false
    #[serde(default)]
    pub error: Option<String>,
}

/// AP2 protocol metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ap2ProtocolInfo {
    /// AP2 protocol version
    #[serde(default)]
    pub version: String,
    /// Supported mandate types (e.g. ["Intent", "Cart", "Payment"])
    #[serde(default)]
    pub supported_mandate_types: Vec<String>,
    /// Supported VC formats (e.g. ["jwt_vc", "ldp_vc"])
    #[serde(default)]
    pub supported_vc_formats: Vec<String>,
    /// Recognized issuer DID methods (e.g. ["did:tenzro", "did:web"])
    #[serde(default)]
    pub supported_did_methods: Vec<String>,
}
