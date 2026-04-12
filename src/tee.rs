//! TEE (Trusted Execution Environment) SDK for Tenzro Network
//!
//! This module provides TEE hardware detection, remote attestation, and
//! enclave data sealing/unsealing across Intel TDX, AMD SEV-SNP, AWS Nitro,
//! and NVIDIA GPU Confidential Computing.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Client for Trusted Execution Environment operations
///
/// Supports Intel TDX, AMD SEV-SNP, AWS Nitro, and NVIDIA GPU TEE providers.
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let tee = client.tee();
///
/// // Detect available TEE hardware
/// let info = tee.detect_tee().await?;
/// println!("TEE available: {} ({})", info.available, info.vendor);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct TeeClient {
    rpc: Arc<RpcClient>,
}

impl TeeClient {
    /// Creates a new TEE client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Detects available TEE hardware on the node
    ///
    /// Returns information about the TEE vendor and capabilities.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let tee = client.tee();
    /// let info = tee.detect_tee().await?;
    /// if info.available {
    ///     println!("TEE vendor: {}", info.vendor);
    ///     println!("Capabilities: {:?}", info.capabilities);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn detect_tee(&self) -> SdkResult<TeeInfo> {
        self.rpc
            .call("tenzro_detectTee", serde_json::json!([]))
            .await
    }

    /// Generates a TEE attestation report
    ///
    /// # Arguments
    ///
    /// * `tee_type` - TEE type: "intel-tdx", "amd-sev-snp", "aws-nitro", "nvidia-gpu"
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let tee = client.tee();
    /// let attestation = tee.get_attestation("intel-tdx").await?;
    /// println!("Report: {}", attestation.report);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_attestation(&self, tee_type: &str) -> SdkResult<AttestationResult> {
        self.rpc
            .call(
                "tenzro_getAttestation",
                serde_json::json!([{ "tee_type": tee_type }]),
            )
            .await
    }

    /// Verifies a TEE attestation report
    ///
    /// Performs full certificate chain verification against the vendor's root CA.
    ///
    /// # Arguments
    ///
    /// * `attestation` - Hex-encoded attestation report
    /// * `tee_type` - TEE type used to generate the attestation
    pub async fn verify_attestation(
        &self,
        attestation: &str,
        tee_type: &str,
    ) -> SdkResult<TeeVerifyResult> {
        self.rpc
            .call(
                "tenzro_verifyTeeAttestation",
                serde_json::json!([{
                    "attestation": attestation,
                    "tee_type": tee_type,
                }]),
            )
            .await
    }

    /// Seals data within a TEE enclave
    ///
    /// Data is encrypted with a key derived from the TEE hardware.
    /// It can only be unsealed on the same or compatible TEE.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw data to seal
    /// * `key_id` - Key identifier for the sealing key
    pub async fn seal_data(&self, data: &[u8], key_id: &str) -> SdkResult<SealedData> {
        self.rpc
            .call(
                "tenzro_sealData",
                serde_json::json!([{
                    "data": hex::encode(data),
                    "key_id": key_id,
                }]),
            )
            .await
    }

    /// Unseals TEE-protected data
    ///
    /// Decrypts data that was previously sealed within a TEE enclave.
    ///
    /// # Arguments
    ///
    /// * `sealed` - Sealed ciphertext bytes
    /// * `key_id` - Key identifier used during sealing
    pub async fn unseal_data(&self, sealed: &[u8], key_id: &str) -> SdkResult<UnsealedData> {
        self.rpc
            .call(
                "tenzro_unsealData",
                serde_json::json!([{
                    "sealed": hex::encode(sealed),
                    "key_id": key_id,
                }]),
            )
            .await
    }

    /// Lists available TEE providers on the network
    ///
    /// Returns providers with their vendor type and availability status.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let tee = client.tee();
    /// let providers = tee.list_tee_providers().await?;
    /// for p in &providers {
    ///     println!("{} ({}) - available: {}", p.address, p.vendor, p.available);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_tee_providers(&self) -> SdkResult<Vec<TeeProvider>> {
        self.rpc
            .call("tenzro_listTeeProviders", serde_json::json!([]))
            .await
    }
}

/// TEE hardware information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeeInfo {
    /// Whether TEE hardware is available
    pub available: bool,
    /// TEE vendor name (e.g., "intel-tdx", "amd-sev-snp", "aws-nitro", "nvidia-gpu", "none")
    #[serde(default)]
    pub vendor: String,
    /// List of TEE capabilities
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// TEE attestation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationResult {
    /// Hex-encoded attestation report
    pub report: String,
    /// Hex-encoded signature over the report
    #[serde(default)]
    pub signature: String,
    /// X.509 certificate chain (PEM or hex-encoded)
    #[serde(default)]
    pub certificate_chain: Vec<String>,
}

/// TEE attestation verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeeVerifyResult {
    /// Whether the attestation is valid
    pub valid: bool,
    /// Verification details or error message
    #[serde(default)]
    pub message: String,
}

/// Data sealed within a TEE enclave
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedData {
    /// Hex-encoded sealed ciphertext
    #[serde(default)]
    pub ciphertext: String,
    /// Key identifier used for sealing
    #[serde(default)]
    pub key_id: String,
}

/// Unsealed plaintext data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsealedData {
    /// Hex-encoded plaintext data
    #[serde(default)]
    pub data: String,
}

/// TEE provider on the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeeProvider {
    /// Provider address (hex)
    #[serde(default)]
    pub address: String,
    /// TEE vendor type
    #[serde(default)]
    pub vendor: String,
    /// Whether the provider is currently available
    #[serde(default)]
    pub available: bool,
}
