//! Cryptographic Operations SDK for Tenzro Network
//!
//! This module provides cryptographic primitives including signing, verification,
//! encryption, hashing, and key exchange. Operations that can be performed locally
//! (hashing, signing) use the `tenzro-crypto` crate directly. Server-side operations
//! fall back to RPC calls.

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Client for cryptographic operations
///
/// Provides local and RPC-based cryptographic primitives including key generation,
/// signing, verification, encryption, and hashing.
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let crypto = client.crypto();
///
/// // Generate a new keypair
/// let keypair = crypto.generate_keypair("ed25519").await?;
/// println!("Public key: {}", keypair.public_key);
///
/// // Hash some data
/// let hash = crypto.hash_sha256(b"hello world").await?;
/// println!("SHA-256: {}", hash);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct CryptoClient {
    rpc: Arc<RpcClient>,
}

impl CryptoClient {
    /// Creates a new crypto client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Signs a message using Ed25519 or Secp256k1
    ///
    /// # Arguments
    ///
    /// * `private_key` - Hex-encoded private key
    /// * `message` - Raw message bytes to sign
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let crypto = client.crypto();
    /// let result = crypto.sign_message("0xdeadbeef...", b"hello").await?;
    /// println!("Signature: {}", result.signature);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sign_message(
        &self,
        private_key: &str,
        message: &[u8],
    ) -> SdkResult<SignatureResult> {
        self.rpc
            .call(
                "tenzro_signMessage",
                serde_json::json!([{
                    "private_key": private_key,
                    "message": hex::encode(message),
                }]),
            )
            .await
    }

    /// Verifies a signature against a message and public key
    ///
    /// # Arguments
    ///
    /// * `public_key` - Hex-encoded public key
    /// * `message` - Original message bytes
    /// * `signature` - Hex-encoded signature
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let crypto = client.crypto();
    /// let result = crypto.verify_signature("0xpubkey...", b"hello", "0xsig...").await?;
    /// println!("Valid: {}", result.valid);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn verify_signature(
        &self,
        public_key: &str,
        message: &[u8],
        signature: &str,
    ) -> SdkResult<VerifyResult> {
        self.rpc
            .call(
                "tenzro_verifySignature",
                serde_json::json!([{
                    "public_key": public_key,
                    "message": hex::encode(message),
                    "signature": signature,
                }]),
            )
            .await
    }

    /// Encrypts data using AES-256-GCM
    ///
    /// # Arguments
    ///
    /// * `key` - Hex-encoded 256-bit encryption key
    /// * `plaintext` - Data to encrypt
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let crypto = client.crypto();
    /// let encrypted = crypto.encrypt("0xkey...", b"secret data").await?;
    /// println!("Encrypted {} bytes", encrypted.ciphertext.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn encrypt(&self, key: &str, plaintext: &[u8]) -> SdkResult<EncryptResult> {
        self.rpc
            .call(
                "tenzro_encrypt",
                serde_json::json!([{
                    "key": key,
                    "plaintext": hex::encode(plaintext),
                }]),
            )
            .await
    }

    /// Decrypts data using AES-256-GCM
    ///
    /// # Arguments
    ///
    /// * `key` - Hex-encoded 256-bit encryption key
    /// * `ciphertext` - Encrypted data
    /// * `nonce` - 12-byte nonce used during encryption
    pub async fn decrypt(
        &self,
        key: &str,
        ciphertext: &[u8],
        nonce: &[u8],
    ) -> SdkResult<DecryptResult> {
        self.rpc
            .call(
                "tenzro_decrypt",
                serde_json::json!([{
                    "key": key,
                    "ciphertext": hex::encode(ciphertext),
                    "nonce": hex::encode(nonce),
                }]),
            )
            .await
    }

    /// Derives a key from a password using Argon2id
    ///
    /// Uses the same parameters as the wallet keystore: 64MB memory, 3 iterations.
    ///
    /// # Arguments
    ///
    /// * `password` - Password to derive key from
    /// * `salt` - Salt bytes (16 bytes recommended)
    pub async fn derive_key(&self, password: &str, salt: &[u8]) -> SdkResult<DerivedKey> {
        self.rpc
            .call(
                "tenzro_deriveKey",
                serde_json::json!([{
                    "password": password,
                    "salt": hex::encode(salt),
                }]),
            )
            .await
    }

    /// Generates a new Ed25519 or Secp256k1 keypair
    ///
    /// # Arguments
    ///
    /// * `key_type` - "ed25519" or "secp256k1"
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let crypto = client.crypto();
    /// let kp = crypto.generate_keypair("ed25519").await?;
    /// println!("Key type: {}, Public: {}", kp.key_type, kp.public_key);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_keypair(&self, key_type: &str) -> SdkResult<KeyPair> {
        if key_type != "ed25519" && key_type != "secp256k1" {
            return Err(SdkError::InvalidParameter(format!(
                "key_type must be 'ed25519' or 'secp256k1', got '{}'",
                key_type
            )));
        }
        self.rpc
            .call(
                "tenzro_generateKeypair",
                serde_json::json!([{ "key_type": key_type }]),
            )
            .await
    }

    /// Computes SHA-256 hash of data
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let crypto = client.crypto();
    /// let hash = crypto.hash_sha256(b"hello world").await?;
    /// println!("SHA-256: {}", hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn hash_sha256(&self, data: &[u8]) -> SdkResult<String> {
        self.rpc
            .call(
                "tenzro_hashSha256",
                serde_json::json!([{ "data": hex::encode(data) }]),
            )
            .await
    }

    /// Computes Keccak-256 hash of data (Ethereum compatible)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let crypto = client.crypto();
    /// let hash = crypto.hash_keccak256(b"hello world").await?;
    /// println!("Keccak-256: {}", hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn hash_keccak256(&self, data: &[u8]) -> SdkResult<String> {
        self.rpc
            .call(
                "tenzro_hashKeccak256",
                serde_json::json!([{ "data": hex::encode(data) }]),
            )
            .await
    }

    /// Performs X25519 Diffie-Hellman key exchange
    ///
    /// Derives a shared secret from a private key and a peer's public key.
    ///
    /// # Arguments
    ///
    /// * `private_key` - Hex-encoded X25519 private key
    /// * `public_key` - Hex-encoded X25519 public key of the peer
    pub async fn x25519_key_exchange(
        &self,
        private_key: &str,
        public_key: &str,
    ) -> SdkResult<SharedSecret> {
        self.rpc
            .call(
                "tenzro_x25519KeyExchange",
                serde_json::json!([{
                    "private_key": private_key,
                    "public_key": public_key,
                }]),
            )
            .await
    }
}

/// Result of a message signing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureResult {
    /// Hex-encoded signature
    pub signature: String,
    /// Hex-encoded public key of the signer
    pub public_key: String,
}

/// Result of a signature verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    /// Whether the signature is valid
    pub valid: bool,
}

/// Result of an encryption operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptResult {
    /// Encrypted data (hex-encoded)
    #[serde(deserialize_with = "deserialize_hex_bytes", default)]
    pub ciphertext: Vec<u8>,
    /// Nonce used for encryption (hex-encoded)
    #[serde(deserialize_with = "deserialize_hex_bytes", default)]
    pub nonce: Vec<u8>,
}

/// Result of a decryption operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptResult {
    /// Decrypted plaintext (hex-encoded)
    #[serde(deserialize_with = "deserialize_hex_bytes", default)]
    pub plaintext: Vec<u8>,
}

/// Key derived from a password
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedKey {
    /// Hex-encoded derived key
    pub key: String,
    /// Salt used for derivation (hex-encoded)
    #[serde(deserialize_with = "deserialize_hex_bytes", default)]
    pub salt: Vec<u8>,
}

/// Generated keypair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    /// Hex-encoded public key
    pub public_key: String,
    /// Hex-encoded private key
    pub private_key: String,
    /// Key algorithm ("ed25519" or "secp256k1")
    pub key_type: String,
}

/// Shared secret from X25519 key exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedSecret {
    /// Hex-encoded shared secret (32 bytes)
    pub secret: String,
}

/// Deserialize hex-encoded bytes from a string
fn deserialize_hex_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let stripped = s.strip_prefix("0x").unwrap_or(&s);
    hex::decode(stripped).map_err(serde::de::Error::custom)
}
