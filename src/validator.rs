//! Validator registry client for Tenzro Network
//!
//! Wraps the `tenzro_getValidatorState` / `tenzro_listValidators` /
//! `tenzro_listActiveValidators` (read) and `tenzro_rotateValidatorKey`
//! (write) RPCs. Used by operator dashboards, SREs, validator-fleet
//! tooling, and any consumer that needs to enumerate the active set,
//! inspect a single validator's stake / activation epoch /
//! TEE-attestation status, or rotate a validator's consensus keys.
//!
//! Validator self-registration still happens through the staking
//! transaction path, not via RPC. Only the consensus / PQ / BLS
//! key-rotation is exposed here.
//!
//! # Example
//!
//! ```no_run
//! # use tenzro_sdk::TenzroClient;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = TenzroClient::new("https://rpc.tenzro.xyz").await?;
//! let actives = client.validators().list_active().await?;
//! println!("{} active validators", actives.count);
//! for v in actives.validators {
//!     println!("{}: stake={}", v.address, v.self_stake);
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Validator registry status, mirrored to its Rust `Debug`-format
/// string on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorStatus {
    Active,
    Candidate,
    PendingActive,
    PendingExit,
    Exited,
    Jailed,
}

/// A single entry in the on-chain validator registry.
///
/// `address` and `withdrawal_address` are base58-encoded (Tenzro's
/// `Address::Display` impl). Pubkeys are plain lowercase hex without
/// `0x` prefix. `self_stake` is a u128 encoded as a decimal string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorRegistryEntry {
    /// Base58-encoded 32-byte address.
    #[serde(default)]
    pub address: String,
    /// Ed25519 consensus pubkey, hex-encoded.
    #[serde(default)]
    pub consensus_pubkey: String,
    /// Byte length of the post-quantum (ML-DSA-65) pubkey blob (1952).
    #[serde(default)]
    pub pq_pubkey_len: u32,
    /// BLS12-381 G1 (min-pk) pubkey, hex-encoded (96 hex chars).
    #[serde(default)]
    pub bls_pubkey: String,
    /// Base58-encoded 32-byte withdrawal address.
    #[serde(default)]
    pub withdrawal_address: String,
    /// u128 decimal string.
    #[serde(default)]
    pub self_stake: String,
    pub status: ValidatorStatus,
    #[serde(default)]
    pub registered_at_epoch: u64,
    #[serde(default)]
    pub activated_at_epoch: Option<u64>,
    #[serde(default)]
    pub exited_at_epoch: Option<u64>,
    #[serde(default)]
    pub jailed_until_epoch: Option<u64>,
    #[serde(default)]
    pub tee_attestation_hash: Option<String>,
    #[serde(default)]
    pub metadata_uri: String,
    /// Unix ms timestamp of last registry mutation.
    #[serde(default)]
    pub updated_at: u64,
}

/// Result of `tenzro_listValidators` / `tenzro_listActiveValidators`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListValidatorsResult {
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub validators: Vec<ValidatorRegistryEntry>,
}

/// Request shape for `tenzro_rotateValidatorKey`.
///
/// All hex fields are 0x-prefixed. The `signature` is produced offline
/// by the operator: sign
/// `SHA-256("tenzro/rotate-validator-key" || address(32) ||
/// new_consensus(32) || new_pq(1952) || new_bls(48) || nonce_le(8))`
/// with the *current* Ed25519 consensus key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotateValidatorKeyRequest {
    /// Validator operator address (0x-prefixed 32-byte hex).
    pub address: String,
    /// New Ed25519 consensus pubkey (0x-prefixed 32-byte hex).
    pub new_consensus_pubkey: String,
    /// New ML-DSA-65 verifying key (0x-prefixed 1952-byte hex).
    pub new_pq_pubkey: String,
    /// New BLS12-381 G1 (min_pk) verifying key (0x-prefixed 48-byte hex).
    pub new_bls_pubkey: String,
    /// Monotonic rotation nonce.
    pub nonce: u64,
    /// Ed25519 signature (0x-prefixed 64-byte hex) by the *current*
    /// consensus key over the canonical preimage.
    pub signature: String,
}

/// Result of `tenzro_rotateValidatorKey`.
///
/// The rotation is recorded on the receiving node only — until the
/// consensus-mediated `RotateValidatorKey` typed transaction ships,
/// operators must fan out the same request to every active validator
/// to avoid a fork at the next epoch boundary (see
/// `tools/deploy/rotate-validator-key.sh`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotateValidatorKeyResult {
    pub address: String,
    pub status: String,
    pub new_consensus_pubkey: String,
    pub new_pq_pubkey: String,
    pub new_bls_pubkey: String,
    pub nonce: u64,
    pub message: String,
}

/// Read-only validator registry client.
#[derive(Clone)]
pub struct ValidatorClient {
    rpc: Arc<RpcClient>,
}

impl ValidatorClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Fetch a single validator's entry by hex-encoded 32-byte address
    /// (with or without `0x` prefix). Returns `None` if not registered.
    pub async fn get_state(
        &self,
        address: &str,
    ) -> SdkResult<Option<ValidatorRegistryEntry>> {
        self.rpc
            .call(
                "tenzro_getValidatorState",
                serde_json::json!({ "address": address }),
            )
            .await
    }

    /// List validators, optionally filtered by status.
    pub async fn list(
        &self,
        status: Option<ValidatorStatus>,
    ) -> SdkResult<ListValidatorsResult> {
        let params = match status {
            Some(s) => serde_json::json!({ "status": s }),
            None => serde_json::json!({}),
        };
        self.rpc.call("tenzro_listValidators", params).await
    }

    /// List only currently-Active validators. Convenience over
    /// `list(Some(ValidatorStatus::Active))`.
    pub async fn list_active(&self) -> SdkResult<ListValidatorsResult> {
        self.rpc
            .call("tenzro_listActiveValidators", serde_json::json!({}))
            .await
    }

    /// Rotate a validator's consensus + PQ + BLS keys.
    ///
    /// Calls `tenzro_rotateValidatorKey`. The signature in `req` must
    /// be produced offline with the *current* consensus key over the
    /// canonical preimage — see [`RotateValidatorKeyRequest`].
    ///
    /// The rotation lands on the receiving node only. Operators must
    /// fan out the same call to every active validator before the
    /// next epoch boundary to avoid a fork — see
    /// `tools/deploy/rotate-validator-key.sh`.
    pub async fn rotate_keys(
        &self,
        req: &RotateValidatorKeyRequest,
    ) -> SdkResult<RotateValidatorKeyResult> {
        let params = serde_json::to_value(req)
            .map_err(|_| crate::error::SdkError::SerializationError)?;
        self.rpc.call("tenzro_rotateValidatorKey", params).await
    }
}
