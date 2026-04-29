//! Wallet SDK for Tenzro Network
//!
//! This module provides wallet management functionality for the SDK.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use crate::types::Address;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Wallet client for managing wallets and balances
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let wallet = client.wallet();
///
/// // Create a new wallet
/// let info = wallet.create_wallet().await?;
/// println!("Address: {}", info.address);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct WalletClient {
    rpc: Arc<RpcClient>,
}

impl WalletClient {
    /// Creates a new wallet client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Creates a new MPC wallet
    ///
    /// Returns wallet info including the address and public key.
    pub async fn create_wallet(&self) -> SdkResult<WalletInfo> {
        self.rpc
            .call("tenzro_createWallet", serde_json::json!([]))
            .await
    }

    /// Creates a new account with a keypair
    ///
    /// # Arguments
    /// * `key_type` - Key algorithm: "ed25519" (default) or "secp256k1"
    pub async fn create_account(&self, key_type: Option<&str>) -> SdkResult<AccountInfo> {
        let params = match key_type {
            Some(kt) => serde_json::json!([{ "key_type": kt }]),
            None => serde_json::json!([{}]),
        };
        self.rpc.call("tenzro_createAccount", params).await
    }

    /// Gets the TNZO balance of an address (in wei)
    pub async fn get_balance(&self, address: Address) -> SdkResult<u128> {
        let hex: String = self
            .rpc
            .call(
                "tenzro_getBalance",
                serde_json::json!([format!("0x{}", hex::encode(address.as_bytes()))]),
            )
            .await?;
        let stripped = hex.strip_prefix("0x").unwrap_or(&hex);
        u128::from_str_radix(stripped, 16)
            .map_err(|e| crate::error::SdkError::RpcError(format!("Bad hex: {}", e)))
    }

    /// Gets the token balance of an address (decimal string)
    pub async fn get_token_balance(&self, address: Address) -> SdkResult<String> {
        self.rpc
            .call(
                "tenzro_tokenBalance",
                serde_json::json!([format!("0x{}", hex::encode(address.as_bytes()))]),
            )
            .await
    }

    /// Gets all asset balances for a wallet address
    pub async fn get_all_balances(&self, address: Address) -> SdkResult<WalletBalance> {
        let balance = self.get_balance(address.clone()).await?;

        Ok(WalletBalance {
            address,
            balances: vec![AssetBalance {
                symbol: "TNZO".to_string(),
                balance,
                decimals: 18,
            }],
        })
    }

    /// Signs and sends a TNZO transfer atomically via the node's hybrid
    /// signing path (`tenzro_signAndSendTransaction`).
    ///
    /// The node identifies the signing wallet from the ambient auth
    /// context (DPoP-bound bearer JWT), constructs the canonical
    /// `Transaction::hash()` preimage including the PQ public key, signs
    /// both the Ed25519 and ML-DSA-65 legs, verifies them against the
    /// preimage, and submits to the mempool. Private keys never travel
    /// over the wire.
    pub async fn send(
        &self,
        from: Address,
        to: Address,
        amount: u64,
    ) -> SdkResult<String> {
        let from_hex = format!("0x{}", hex::encode(from.as_bytes()));
        let to_hex = format!("0x{}", hex::encode(to.as_bytes()));

        let nonce_hex: String = self
            .rpc
            .call(
                "tenzro_getNonce",
                serde_json::json!([from_hex.clone()]),
            )
            .await?;
        let nonce = u64::from_str_radix(
            nonce_hex.strip_prefix("0x").unwrap_or(&nonce_hex),
            16,
        )
        .unwrap_or(0);

        let chain_hex: String = self
            .rpc
            .call("eth_chainId", serde_json::json!([]))
            .await?;
        let chain_id = u64::from_str_radix(
            chain_hex.strip_prefix("0x").unwrap_or(&chain_hex),
            16,
        )
        .unwrap_or(1337);

        self.rpc
            .call(
                "tenzro_signAndSendTransaction",
                serde_json::json!({
                    "from": from_hex,
                    "to": to_hex,
                    "value": amount,
                    "gas_limit": 21000,
                    "gas_price": 1_000_000_000u64,
                    "nonce": nonce,
                    "chain_id": chain_id,
                }),
            )
            .await
    }

    /// Gets the nonce for an address
    pub async fn get_nonce(&self, address: Address) -> SdkResult<u64> {
        let hex: String = self
            .rpc
            .call(
                "tenzro_getNonce",
                serde_json::json!([format!("0x{}", hex::encode(address.as_bytes()))]),
            )
            .await?;
        let stripped = hex.strip_prefix("0x").unwrap_or(&hex);
        u64::from_str_radix(stripped, 16)
            .map_err(|e| crate::error::SdkError::RpcError(format!("Bad hex: {}", e)))
    }
}

/// Wallet information returned from `create_wallet`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    /// Wallet ID
    #[serde(default)]
    pub wallet_id: String,
    /// Wallet address
    #[serde(default)]
    pub address: String,
    /// Public key (hex)
    #[serde(default)]
    pub public_key: String,
    /// Key type
    #[serde(default)]
    pub key_type: String,
    /// Threshold (MPC)
    #[serde(default)]
    pub threshold: u32,
    /// Total key shares
    #[serde(default)]
    pub total_shares: u32,
}

/// Account information returned from `create_account`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Account address
    #[serde(default)]
    pub address: String,
    /// Public key (hex)
    #[serde(default)]
    pub public_key: String,
    /// Private key (hex) -- store securely!
    #[serde(default)]
    pub private_key: String,
    /// Key type
    #[serde(default)]
    pub key_type: String,
}

/// Wallet balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    /// Wallet address
    pub address: Address,
    /// All asset balances
    pub balances: Vec<AssetBalance>,
}

/// Balance information for a specific asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalance {
    /// Asset symbol
    pub symbol: String,
    /// Balance in smallest unit (wei)
    pub balance: u128,
    /// Number of decimals
    pub decimals: u8,
}

impl AssetBalance {
    /// Returns the balance as a human-readable decimal value
    pub fn as_decimal(&self) -> f64 {
        self.balance as f64 / 10_f64.powi(self.decimals as i32)
    }
}
