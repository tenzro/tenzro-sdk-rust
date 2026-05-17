//! Zero-Knowledge Proof SDK for Tenzro Network
//!
//! This module provides ZK proof generation and verification using Groth16
//! and PlonK circuits on BN254. Supports inference verification, settlement
//! proofs, and identity proofs.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Client for zero-knowledge proof operations
///
/// Supports Groth16 SNARKs on BN254, PlonK, and hybrid ZK-in-TEE proofs.
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let zk = client.zk();
///
/// // List available circuits
/// let circuits = zk.list_circuits().await?;
/// for c in &circuits {
///     println!("{}: {} ({} constraints)", c.name, c.circuit_type, c.constraints);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct ZkClient {
    rpc: Arc<RpcClient>,
}

impl ZkClient {
    /// Creates a new ZK client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Creates a zero-knowledge proof
    ///
    /// Generates a Groth16 or PlonK proof for the specified circuit type
    /// using the provided private and public inputs.
    ///
    /// # Arguments
    ///
    /// * `circuit_type` - Circuit type: "inference", "settlement", or "identity"
    /// * `private_inputs` - Private witness values (JSON object)
    /// * `public_inputs` - Public input values (hex-encoded field elements)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let zk = client.zk();
    /// let proof = zk.create_proof(
    ///     "inference",
    ///     serde_json::json!({"model_hash": "0xabc...", "input_hash": "0xdef..."}),
    ///     vec!["0x1234...".to_string()],
    /// ).await?;
    /// println!("Proof type: {}", proof.proof_type);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_proof(
        &self,
        circuit_type: &str,
        private_inputs: serde_json::Value,
        public_inputs: Vec<String>,
    ) -> SdkResult<ZkProof> {
        self.rpc
            .call(
                "tenzro_createZkProof",
                serde_json::json!([{
                    "circuit_type": circuit_type,
                    "private_inputs": private_inputs,
                    "public_inputs": public_inputs,
                }]),
            )
            .await
    }

    /// Verifies a zero-knowledge proof
    ///
    /// # Arguments
    ///
    /// * `proof` - Hex-encoded proof data
    /// * `proof_type` - Proof system: "groth16", "plonk", "halo2", or "stark"
    /// * `public_inputs` - Public input values used during proof generation
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let zk = client.zk();
    /// let result = zk.verify_proof(
    ///     "0xproof...",
    ///     "groth16",
    ///     vec!["0x1234...".to_string()],
    /// ).await?;
    /// println!("Proof valid: {}", result.valid);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn verify_proof(
        &self,
        proof: &str,
        proof_type: &str,
        public_inputs: Vec<String>,
    ) -> SdkResult<ZkVerifyResult> {
        self.rpc
            .call(
                "tenzro_verifyZkProof",
                serde_json::json!([{
                    "proof": proof,
                    "proof_type": proof_type,
                    "public_inputs": public_inputs,
                }]),
            )
            .await
    }

    /// Generates a proving key for a circuit
    ///
    /// The proving key is required to create proofs for a specific circuit.
    /// This involves the trusted setup ceremony parameters.
    ///
    /// # Arguments
    ///
    /// * `circuit_type` - Circuit type: "inference", "settlement", or "identity"
    pub async fn generate_proving_key(&self, circuit_type: &str) -> SdkResult<ProvingKey> {
        self.rpc
            .call(
                "tenzro_generateProvingKey",
                serde_json::json!([{ "circuit_type": circuit_type }]),
            )
            .await
    }

    /// Lists available ZK circuits
    ///
    /// Returns pre-built circuits including inference verification,
    /// settlement proofs, and identity proofs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let zk = client.zk();
    /// let circuits = zk.list_circuits().await?;
    /// for c in &circuits {
    ///     println!("{}: {} constraints", c.name, c.constraints);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_circuits(&self) -> SdkResult<Vec<CircuitInfo>> {
        self.rpc
            .call("tenzro_listCircuits", serde_json::json!([]))
            .await
    }
}

/// A zero-knowledge proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkProof {
    /// Hex-encoded proof data
    pub proof: String,
    /// Public inputs used in the proof
    #[serde(default)]
    pub public_inputs: Vec<String>,
    /// Proof system type ("groth16", "plonk", etc.)
    #[serde(default)]
    pub proof_type: String,
}

/// ZK proof verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkVerifyResult {
    /// Whether the proof is valid
    pub valid: bool,
    /// Verification details or error message
    #[serde(default)]
    pub message: String,
}

/// Proving key for a ZK circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvingKey {
    /// Key identifier
    #[serde(default)]
    pub key_id: String,
    /// Circuit type this key is for
    #[serde(default)]
    pub circuit_type: String,
}

/// Information about a ZK circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitInfo {
    /// Circuit name
    #[serde(default)]
    pub name: String,
    /// Circuit type ("inference", "settlement", "identity")
    #[serde(default)]
    pub circuit_type: String,
    /// Number of constraints
    #[serde(default)]
    pub constraints: u64,
}
