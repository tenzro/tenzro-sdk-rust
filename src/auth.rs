//! Onboarding key management for Tenzro Network
//!
//! This module provides onboarding key issuance, listing, revocation, and
//! validation for network participants.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use crate::identity::IdentityType;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Client for onboarding key operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let auth = client.auth();
///
/// // Issue an onboarding key for a new participant
/// let key = auth.issue_onboarding_key(
///     "Alice",
///     "did:tenzro:human:abc123",
///     "0x1234abcd",
///     tenzro_sdk::identity::IdentityType::Human,
/// ).await?;
/// println!("Onboarding key: {}", key.key);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct AuthClient {
    rpc: Arc<RpcClient>,
}

impl AuthClient {
    /// Creates a new auth client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Issues an onboarding key for a new participant
    ///
    /// An onboarding key allows a participant to join the network without
    /// going through the full identity registration flow. The key is tied
    /// to a specific DID, address, and identity type.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable name for the participant
    /// * `did` - The DID to associate with the key
    /// * `address` - The wallet address to associate with the key
    /// * `identity_type` - Whether this is a `Human` or `Machine` identity
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig, identity::IdentityType};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let auth = client.auth();
    /// let key = auth.issue_onboarding_key(
    ///     "Alice",
    ///     "did:tenzro:human:abc123",
    ///     "0xdeadbeef",
    ///     IdentityType::Human,
    /// ).await?;
    /// println!("Key: {}", key.key);
    /// println!("Expires: {}", key.expires_at);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn issue_onboarding_key(
        &self,
        name: &str,
        did: &str,
        address: &str,
        identity_type: IdentityType,
    ) -> SdkResult<OnboardingKey> {
        self.rpc
            .call(
                "tenzro_issueOnboardingKey",
                serde_json::json!([{
                    "name": name,
                    "did": did,
                    "address": address,
                    "identity_type": identity_type,
                }]),
            )
            .await
    }

    /// Lists all active onboarding keys
    ///
    /// Returns all onboarding keys that have been issued and not yet revoked.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let auth = client.auth();
    /// let keys = auth.list_onboarding_keys().await?;
    /// for key in &keys {
    ///     println!("{} — {} ({})", key.name, key.did, key.status);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_onboarding_keys(&self) -> SdkResult<Vec<OnboardingKey>> {
        self.rpc
            .call("tenzro_listOnboardingKeys", serde_json::json!([]))
            .await
    }

    /// Revokes an onboarding key by DID or key hash
    ///
    /// Once revoked, the key can no longer be used to join the network.
    ///
    /// # Arguments
    ///
    /// * `did_or_hash` - Either the DID associated with the key, or the key hash
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let auth = client.auth();
    /// let result = auth.revoke_onboarding_key("did:tenzro:human:abc123").await?;
    /// println!("Revoked: {}", result.revoked);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn revoke_onboarding_key(
        &self,
        did_or_hash: &str,
    ) -> SdkResult<RevokeKeyResponse> {
        self.rpc
            .call(
                "tenzro_revokeOnboardingKey",
                serde_json::json!([{ "did_or_hash": did_or_hash }]),
            )
            .await
    }

    /// Validates an onboarding key
    ///
    /// Checks that the key is valid, not expired, and not revoked.
    ///
    /// # Arguments
    ///
    /// * `key` - The onboarding key string to validate
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let auth = client.auth();
    /// let result = auth.validate_onboarding_key("tnzo_key_abc123").await?;
    /// if result.valid {
    ///     println!("Key is valid for DID: {}", result.did.unwrap_or_default());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate_onboarding_key(
        &self,
        key: &str,
    ) -> SdkResult<ValidateKeyResponse> {
        self.rpc
            .call(
                "tenzro_validateOnboardingKey",
                serde_json::json!([{ "key": key }]),
            )
            .await
    }
}

/// An onboarding key issued to a network participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingKey {
    /// The key string (opaque token)
    pub key: String,
    /// Human-readable name for the participant
    #[serde(default)]
    pub name: String,
    /// The DID associated with this key
    #[serde(default)]
    pub did: String,
    /// The wallet address associated with this key
    #[serde(default)]
    pub address: String,
    /// Identity type ("Human" or "Machine")
    #[serde(default)]
    pub identity_type: String,
    /// Key status ("active", "revoked", "expired")
    #[serde(default)]
    pub status: String,
    /// ISO-8601 timestamp when the key was issued
    #[serde(default)]
    pub issued_at: String,
    /// ISO-8601 timestamp when the key expires (if applicable)
    #[serde(default)]
    pub expires_at: String,
    /// SHA-256 hash of the key (safe to store/log)
    #[serde(default)]
    pub key_hash: String,
}

/// Response from revoking an onboarding key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeKeyResponse {
    /// Whether the key was successfully revoked
    pub revoked: bool,
    /// The DID whose key was revoked
    #[serde(default)]
    pub did: String,
    /// Status message
    #[serde(default)]
    pub message: String,
}

/// Response from validating an onboarding key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateKeyResponse {
    /// Whether the key is currently valid
    pub valid: bool,
    /// The DID associated with the key (if valid)
    pub did: Option<String>,
    /// The wallet address associated with the key (if valid)
    pub address: Option<String>,
    /// Identity type (if valid)
    pub identity_type: Option<String>,
    /// Reason the key is invalid (if not valid)
    #[serde(default)]
    pub reason: String,
}
