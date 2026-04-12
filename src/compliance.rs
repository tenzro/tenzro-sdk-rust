//! ERC-3643 Compliance SDK for Tenzro Network
//!
//! This module provides compliance management for regulated tokens on Tenzro Network.
//! It implements the ERC-3643 (T-REX) standard for permissioned token transfers,
//! enabling KYC enforcement, holder limits, country restrictions, and address freezing.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Compliance client for ERC-3643 regulated token operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let compliance = client.compliance();
///
/// // Check if a transfer is compliant
/// let result = compliance.check_compliance(
///     "0xtoken...",
///     "0xsender...",
///     "0xrecipient...",
///     "1000000000000000000",
/// ).await?;
/// println!("Compliant: {}", result.compliant);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct ComplianceClient {
    rpc: Arc<RpcClient>,
}

impl ComplianceClient {
    /// Creates a new compliance client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Registers compliance rules for a token
    ///
    /// Attaches ERC-3643 compliance rules to a token in the unified registry.
    /// Once registered, all transfers of this token are subject to the specified
    /// rules.
    ///
    /// # Arguments
    ///
    /// * `token_id` - Token registry ID or symbol
    /// * `kyc_required` - Whether KYC verification is required for holders
    /// * `holder_limit` - Maximum number of token holders (0 for unlimited)
    /// * `country_restrictions` - Optional list of ISO 3166-1 alpha-2 country codes to block
    /// * `balance_cap` - Optional maximum balance per holder (decimal string)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let compliance = client.compliance();
    /// let rules = compliance.register_compliance(
    ///     "MTK",
    ///     true,
    ///     500,
    ///     Some(&["US", "KP"]),
    ///     Some("1000000000000000000000"), // 1000 tokens max per holder
    /// ).await?;
    /// println!("Compliance registered: {}", rules.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register_compliance(
        &self,
        token_id: &str,
        kyc_required: bool,
        holder_limit: u64,
        country_restrictions: Option<&[&str]>,
        balance_cap: Option<&str>,
    ) -> SdkResult<ComplianceRules> {
        let mut params = serde_json::json!({
            "token_id": token_id,
            "kyc_required": kyc_required,
            "holder_limit": holder_limit,
        });

        if let Some(countries) = country_restrictions {
            params["country_restrictions"] = serde_json::json!(countries);
        }
        if let Some(cap) = balance_cap {
            params["balance_cap"] = serde_json::json!(cap);
        }

        self.rpc
            .call("tenzro_registerCompliance", serde_json::json!([params]))
            .await
    }

    /// Checks whether a transfer complies with the token's rules
    ///
    /// Evaluates the transfer against all registered compliance rules (KYC,
    /// holder limits, country restrictions, balance caps, and frozen addresses)
    /// without executing it.
    ///
    /// # Arguments
    ///
    /// * `token_id` - Token registry ID or symbol
    /// * `from` - Sender address (hex)
    /// * `to` - Recipient address (hex)
    /// * `amount` - Transfer amount (decimal string)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let compliance = client.compliance();
    /// let result = compliance.check_compliance(
    ///     "MTK",
    ///     "0xsender...",
    ///     "0xrecipient...",
    ///     "500000000000000000000",
    /// ).await?;
    ///
    /// if result.compliant {
    ///     println!("Transfer is compliant");
    /// } else {
    ///     println!("Transfer blocked: {}", result.reason);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn check_compliance(
        &self,
        token_id: &str,
        from: &str,
        to: &str,
        amount: &str,
    ) -> SdkResult<ComplianceResult> {
        self.rpc
            .call(
                "tenzro_checkCompliance",
                serde_json::json!([{
                    "token_id": token_id,
                    "from": from,
                    "to": to,
                    "amount": amount,
                }]),
            )
            .await
    }

    /// Freezes an address for a specific token
    ///
    /// Prevents the specified address from sending or receiving the token.
    /// Requires the caller to be the token's compliance agent.
    ///
    /// # Arguments
    ///
    /// * `token_id` - Token registry ID or symbol
    /// * `address` - Address to freeze (hex)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let compliance = client.compliance();
    /// let result = compliance.freeze_address("MTK", "0xsuspicious...").await?;
    /// println!("Freeze status: {}", result.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn freeze_address(
        &self,
        token_id: &str,
        address: &str,
    ) -> SdkResult<FreezeResult> {
        self.rpc
            .call(
                "tenzro_freezeAddress",
                serde_json::json!([{
                    "token_id": token_id,
                    "address": address,
                }]),
            )
            .await
    }
}

/// Compliance rules registered for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRules {
    /// Token registry ID or symbol
    #[serde(default)]
    pub token_id: String,
    /// Whether KYC is required for holders
    #[serde(default)]
    pub kyc_required: bool,
    /// Maximum number of token holders (0 = unlimited)
    #[serde(default)]
    pub holder_limit: u64,
    /// Blocked country codes (ISO 3166-1 alpha-2)
    #[serde(default)]
    pub country_restrictions: Vec<String>,
    /// Maximum balance per holder (decimal string, empty = unlimited)
    #[serde(default)]
    pub balance_cap: String,
    /// Operation status (e.g., "registered", "updated")
    #[serde(default)]
    pub status: String,
}

/// Result of a compliance check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceResult {
    /// Whether the transfer is compliant
    #[serde(default)]
    pub compliant: bool,
    /// Reason for non-compliance (empty if compliant)
    #[serde(default)]
    pub reason: String,
    /// Which rule was violated (e.g., "kyc", "holder_limit", "country", "frozen", "balance_cap")
    #[serde(default)]
    pub violated_rule: String,
    /// Token registry ID
    #[serde(default)]
    pub token_id: String,
}

/// Result from freezing an address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeResult {
    /// Token registry ID
    #[serde(default)]
    pub token_id: String,
    /// Frozen address (hex)
    #[serde(default)]
    pub address: String,
    /// Whether the address is now frozen
    #[serde(default)]
    pub frozen: bool,
    /// Operation status
    #[serde(default)]
    pub status: String,
}
