//! Wallet SDK for Tenzro Network
//!
//! This module provides wallet management functionality for the SDK.

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use crate::types::Address;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// A self-custody hybrid signer: holds an Ed25519 + ML-DSA-65 keypair
/// locally and signs both legs of a Tenzro transaction without the node
/// ever seeing the secret. This is the client side of the self-custody
/// path — TEE-equipped or key-holding runners implement it (the CLI's
/// sealed `~/.tenzro/hybrid_key.json` keystore is one such implementation)
/// so `WalletClient::send_self_custody` can build the canonical
/// `Transaction::hash()` preimage, obtain both signatures, and submit via
/// `eth_sendRawTransaction`.
///
/// The server-custodial `WalletClient::send` path remains the default for
/// runners that bring no local key; this trait is purely additive.
#[async_trait]
pub trait HybridSigner: Send + Sync {
    /// Raw 32-byte Ed25519 public key. This IS the account address on the
    /// Tenzro native convention — the node's `eth_sendRawTransaction`
    /// verifier accepts a raw 32-byte pubkey placed in `from`.
    fn ed25519_public_key(&self) -> Vec<u8>;

    /// ML-DSA-65 verifying key bytes (FIPS 204, exactly 1952) for the
    /// mandatory `pq_public_key` field.
    fn ml_dsa_verifying_key(&self) -> Vec<u8>;

    /// Sign `message` with both legs. Returns
    /// `(ed25519_sig_64, ml_dsa_sig_3309)`.
    async fn sign_hybrid(&self, message: &[u8]) -> SdkResult<(Vec<u8>, Vec<u8>)>;
}

/// Builds the canonical `Transaction::hash()` preimage for a native TNZO
/// transfer, byte-identical to `tenzro_types::transaction::Transaction::hash`.
///
/// Preimage order (all integers little-endian):
/// `chain_id ‖ from(32) ‖ to(32) ‖ nonce ‖ gas_limit ‖ gas_price ‖
/// timestamp ‖ tx_type_json ‖ (no memo) ‖ pq_len(u32) ‖ pq_public_key`.
///
/// The SDK is workspace-isolated (empty `[workspace]`, standalone git
/// mirror) so it cannot depend on `tenzro-types`; the preimage is
/// reproduced here. `tx_type_json` reproduces serde's externally-tagged
/// `{"Transfer":{"amount":<N>}}` with `serde_json`'s exact formatting (no
/// whitespace, bare integer).
#[allow(clippy::too_many_arguments)]
fn transfer_tx_hash(
    chain_id: u64,
    from: &[u8; 32],
    to: &[u8; 32],
    nonce: u64,
    gas_limit: u64,
    gas_price: u64,
    timestamp_ms: i64,
    amount_wei: u128,
    pq_public_key: &[u8],
) -> [u8; 32] {
    let tx_type_json = format!("{{\"Transfer\":{{\"amount\":{}}}}}", amount_wei);
    let mut hasher = Sha256::new();
    hasher.update(chain_id.to_le_bytes());
    hasher.update(from);
    hasher.update(to);
    hasher.update(nonce.to_le_bytes());
    hasher.update(gas_limit.to_le_bytes());
    hasher.update(gas_price.to_le_bytes());
    hasher.update(timestamp_ms.to_le_bytes());
    hasher.update(tx_type_json.as_bytes());
    // memo is None on native transfers — nothing appended.
    hasher.update((pq_public_key.len() as u32).to_le_bytes());
    hasher.update(pq_public_key);
    hasher.finalize().into()
}

/// Current wall-clock time in milliseconds since the Unix epoch, matching
/// `tenzro_types::primitives::Timestamp::now()`.
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

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

    /// Creates a new chain-agnostic 2-of-3 Ed25519 MPC wallet.
    ///
    /// Tenzro wallets are chain-agnostic by design — a single wallet projects
    /// into EVM, SVM, and Canton via the pointer-token model, so there is no
    /// per-chain parameter. Use `cross_vm_transfer` / `wrap_tnzo` for
    /// VM-specific operations and the bridge clients (LayerZero V2, Chainlink
    /// CCIP, deBridge, Wormhole NTT) for sends to external chains. Returns
    /// wallet info including the canonical Tenzro address and public key.
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
                serde_json::json!({
                    "address": format!("0x{}", hex::encode(address.as_bytes()))
                }),
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

    /// Signs a TNZO transfer locally with a self-custody [`HybridSigner`]
    /// and submits it via `eth_sendRawTransaction` — the node never holds
    /// the secret.
    ///
    /// The signer's raw 32-byte Ed25519 public key is the `from` account.
    /// This method fetches the nonce and chain id, builds the canonical
    /// `Transaction::hash()` preimage (including the PQ verifying key),
    /// asks the signer for both the Ed25519 and ML-DSA-65 legs over that
    /// hash, and submits the pre-signed transaction. Both legs are
    /// mandatory; the node rejects a raw send that omits either.
    ///
    /// Amount is in wei (smallest TNZO unit). Returns the transaction hash.
    pub async fn send_self_custody(
        &self,
        signer: &Arc<dyn HybridSigner>,
        to: Address,
        amount_wei: u128,
    ) -> SdkResult<String> {
        let from_bytes = signer.ed25519_public_key();
        if from_bytes.len() != 32 {
            return Err(SdkError::WalletError(format!(
                "self-custody Ed25519 public key must be 32 bytes, got {}",
                from_bytes.len()
            )));
        }
        let mut from_arr = [0u8; 32];
        from_arr.copy_from_slice(&from_bytes);
        let from_hex = format!("0x{}", hex::encode(from_arr));
        let to_hex = format!("0x{}", hex::encode(to.as_bytes()));
        let mut to_arr = [0u8; 32];
        to_arr.copy_from_slice(to.as_bytes());

        let nonce_hex: String = self
            .rpc
            .call("tenzro_getNonce", serde_json::json!([from_hex.clone()]))
            .await?;
        let nonce =
            u64::from_str_radix(nonce_hex.strip_prefix("0x").unwrap_or(&nonce_hex), 16)
                .unwrap_or(0);

        let chain_hex: String = self
            .rpc
            .call("eth_chainId", serde_json::json!([]))
            .await?;
        let chain_id =
            u64::from_str_radix(chain_hex.strip_prefix("0x").unwrap_or(&chain_hex), 16)
                .unwrap_or(1337);

        let pq_public_key = signer.ml_dsa_verifying_key();
        let gas_limit = 21_000u64;
        let gas_price = 1_000_000_000u64;
        let timestamp_ms = now_ms();

        let hash = transfer_tx_hash(
            chain_id,
            &from_arr,
            &to_arr,
            nonce,
            gas_limit,
            gas_price,
            timestamp_ms,
            amount_wei,
            &pq_public_key,
        );
        let (ed_sig, ml_dsa_sig) = signer.sign_hybrid(&hash).await?;

        self.rpc
            .call(
                "eth_sendRawTransaction",
                serde_json::json!({
                    "from": from_hex,
                    "to": to_hex,
                    "value": amount_wei.to_string(),
                    "gas_limit": gas_limit,
                    "gas_price": gas_price,
                    "nonce": nonce,
                    "chain_id": chain_id,
                    "timestamp": timestamp_ms,
                    "public_key": hex::encode(from_arr),
                    "signature": hex::encode(&ed_sig),
                    "pq_public_key": hex::encode(&pq_public_key),
                    "pq_signature": hex::encode(&ml_dsa_sig),
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
