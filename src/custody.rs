//! Key Custody & Wallet Security SDK for Tenzro Network
//!
//! This module provides MPC threshold wallet management, encrypted keystore
//! import/export, key share rotation, spending limits, and session key
//! authorization.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Client for key custody and wallet security operations
///
/// Provides MPC wallet creation, keystore management, key rotation,
/// spending policies, and session key authorization.
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let custody = client.custody();
///
/// // Create a 2-of-3 MPC wallet
/// let wallet = custody.create_mpc_wallet(2, 3, "ed25519").await?;
/// println!("Wallet: {} ({})", wallet.address, wallet.wallet_id);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct CustodyClient {
    rpc: Arc<RpcClient>,
}

impl CustodyClient {
    /// Creates a new custody client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Creates a new MPC threshold wallet
    ///
    /// # Arguments
    ///
    /// * `threshold` - Minimum number of shares required to sign (e.g., 2)
    /// * `total_shares` - Total number of key shares (e.g., 3)
    /// * `key_type` - Key algorithm: "ed25519" or "secp256k1"
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let custody = client.custody();
    /// let wallet = custody.create_mpc_wallet(2, 3, "ed25519").await?;
    /// println!("Address: {}", wallet.address);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_mpc_wallet(
        &self,
        threshold: u8,
        total_shares: u8,
        key_type: &str,
    ) -> SdkResult<MpcWallet> {
        self.rpc
            .call(
                "tenzro_createMpcWallet",
                serde_json::json!([{
                    "threshold": threshold,
                    "total_shares": total_shares,
                    "key_type": key_type,
                }]),
            )
            .await
    }

    /// Exports an encrypted keystore
    ///
    /// The keystore is encrypted using Argon2id KDF + AES-256-GCM.
    ///
    /// # Arguments
    ///
    /// * `wallet_id` - Wallet identifier
    /// * `password` - Password to encrypt the keystore
    pub async fn export_keystore(
        &self,
        wallet_id: &str,
        password: &str,
    ) -> SdkResult<EncryptedKeystore> {
        self.rpc
            .call(
                "tenzro_exportKeystore",
                serde_json::json!([{
                    "wallet_id": wallet_id,
                    "password": password,
                }]),
            )
            .await
    }

    /// Imports a wallet from an encrypted keystore
    ///
    /// # Arguments
    ///
    /// * `keystore` - Encrypted keystore JSON string
    /// * `password` - Password to decrypt the keystore
    pub async fn import_keystore(
        &self,
        keystore: &str,
        password: &str,
    ) -> SdkResult<MpcWallet> {
        self.rpc
            .call(
                "tenzro_importKeystore",
                serde_json::json!([{
                    "keystore": keystore,
                    "password": password,
                }]),
            )
            .await
    }

    /// Gets key share metadata for a wallet
    ///
    /// Returns metadata about each key share (index, creation time).
    /// Does NOT return the actual key share material.
    ///
    /// # Arguments
    ///
    /// * `wallet_id` - Wallet identifier
    pub async fn get_key_shares(&self, wallet_id: &str) -> SdkResult<Vec<KeyShare>> {
        self.rpc
            .call(
                "tenzro_getKeyShares",
                serde_json::json!([{ "wallet_id": wallet_id }]),
            )
            .await
    }

    /// Rotates MPC key shares
    ///
    /// Generates new key shares while preserving the same public key and address.
    /// Old shares are invalidated.
    ///
    /// # Arguments
    ///
    /// * `wallet_id` - Wallet identifier
    pub async fn rotate_keys(&self, wallet_id: &str) -> SdkResult<RotationResult> {
        self.rpc
            .call(
                "tenzro_rotateKeys",
                serde_json::json!([{ "wallet_id": wallet_id }]),
            )
            .await
    }

    /// Sets spending limits for a wallet
    ///
    /// Configures daily and per-transaction spending limits.
    ///
    /// # Arguments
    ///
    /// * `wallet_id` - Wallet identifier
    /// * `daily_limit` - Maximum daily spending (in smallest unit)
    /// * `per_tx_limit` - Maximum per-transaction spending (in smallest unit)
    pub async fn set_spending_limits(
        &self,
        wallet_id: &str,
        daily_limit: u128,
        per_tx_limit: u128,
    ) -> SdkResult<SpendingPolicy> {
        self.rpc
            .call(
                "tenzro_setSpendingLimits",
                serde_json::json!([{
                    "wallet_id": wallet_id,
                    "daily_limit": daily_limit.to_string(),
                    "per_tx_limit": per_tx_limit.to_string(),
                }]),
            )
            .await
    }

    /// Revokes an active session key
    ///
    /// Immediately invalidates a session key, preventing any further
    /// operations using that session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session key identifier to revoke
    pub async fn revoke_session(&self, session_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_revokeSession",
                serde_json::json!([{ "session_id": session_id }]),
            )
            .await
    }

    /// Gets current spending limits for a wallet
    ///
    /// # Arguments
    ///
    /// * `wallet_id` - Wallet identifier
    pub async fn get_spending_limits(&self, wallet_id: &str) -> SdkResult<SpendingPolicy> {
        self.rpc
            .call(
                "tenzro_getSpendingLimits",
                serde_json::json!([{ "wallet_id": wallet_id }]),
            )
            .await
    }

    /// Creates a session key with scoped permissions
    ///
    /// Session keys allow temporary, limited access to wallet operations
    /// without exposing the master key shares.
    ///
    /// # Arguments
    ///
    /// * `wallet_id` - Wallet identifier
    /// * `duration_secs` - Session validity duration in seconds
    /// * `operations` - Allowed operations (e.g., "transfer", "stake", "governance")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let custody = client.custody();
    /// let session = custody.authorize_session(
    ///     "wallet-123",
    ///     3600, // 1 hour
    ///     vec!["transfer".to_string(), "stake".to_string()],
    /// ).await?;
    /// println!("Session: {} (expires: {})", session.session_id, session.expires_at);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn authorize_session(
        &self,
        wallet_id: &str,
        duration_secs: u64,
        operations: Vec<String>,
    ) -> SdkResult<SessionKey> {
        self.rpc
            .call(
                "tenzro_authorizeSession",
                serde_json::json!([{
                    "wallet_id": wallet_id,
                    "duration_secs": duration_secs,
                    "operations": operations,
                }]),
            )
            .await
    }
}

/// MPC threshold wallet information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpcWallet {
    /// Wallet identifier
    #[serde(default)]
    pub wallet_id: String,
    /// Wallet address (hex)
    #[serde(default)]
    pub address: String,
    /// Signing threshold (e.g., 2)
    #[serde(default)]
    pub threshold: u8,
    /// Total number of key shares (e.g., 3)
    #[serde(default)]
    pub total_shares: u8,
}

/// Encrypted keystore export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKeystore {
    /// Encrypted keystore data (JSON string)
    #[serde(default)]
    pub encrypted: String,
    /// Key derivation function used ("argon2id")
    #[serde(default)]
    pub kdf: String,
    /// Cipher used ("aes-256-gcm")
    #[serde(default)]
    pub cipher: String,
}

/// Key share metadata (not the actual share material)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyShare {
    /// Share index (1-based)
    #[serde(default)]
    pub index: u8,
    /// When this share was created
    #[serde(default)]
    pub created_at: String,
}

/// Result of a key rotation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationResult {
    /// Whether the rotation succeeded
    #[serde(default)]
    pub success: bool,
    /// Number of shares rotated
    #[serde(default)]
    pub shares_rotated: u8,
    /// New rotation epoch
    #[serde(default)]
    pub epoch: u64,
}

/// Wallet spending policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingPolicy {
    /// Maximum daily spending (in smallest unit)
    #[serde(default)]
    pub daily_limit: u128,
    /// Maximum per-transaction spending (in smallest unit)
    #[serde(default)]
    pub per_tx_limit: u128,
    /// Amount already spent today (in smallest unit)
    #[serde(default)]
    pub daily_spent: u128,
}

/// Scoped session key for temporary wallet access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionKey {
    /// Session key identifier
    #[serde(default)]
    pub session_id: String,
    /// When the session expires (ISO 8601)
    #[serde(default)]
    pub expires_at: String,
    /// Allowed operations
    #[serde(default)]
    pub operations: Vec<String>,
}
