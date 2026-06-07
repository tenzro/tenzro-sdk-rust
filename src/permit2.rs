//! Permit2 `SignatureTransfer` client.
//!
//! Wraps the node-side helpers for EIP-712 Permit2 flows on the Tenzro
//! EVM. Permit2 lets users sign a single EIP-712 message authorizing a
//! token transfer (with an optional witness — used by ERC-7683 origin
//! opens to bind the permit to a specific cross-chain order).
//!
//! - `tenzro_permit2DomainSeparator` — the per-chain EIP-712 domain
//!   separator over the canonical Tenzro Permit2 verifying contract
//!   (`0x0000…00001023`).
//! - `tenzro_permit2Digest` — compute the EIP-712 digest the user
//!   signs (with or without a witness).
//! - `tenzro_permit2VerifyAndConsume` — atomic verify + nonce-consume
//!   for a relayer presenting a signed permit.
//! - `tenzro_permit2NonceUsed` — read whether a `(owner, nonce)` slot
//!   has been consumed.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct Permit2Client {
    rpc: Arc<RpcClient>,
}

impl Permit2Client {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Fetch the EIP-712 domain separator for the canonical Tenzro
    /// Permit2 contract on `chain_id`.
    pub async fn domain_separator(&self, chain_id: u64) -> SdkResult<Permit2DomainSeparator> {
        self.rpc
            .call(
                "tenzro_permit2DomainSeparator",
                serde_json::json!([{ "chain_id": chain_id }]),
            )
            .await
    }

    /// Compute the EIP-712 digest a user signs for a Permit2
    /// `SignatureTransfer`. If `witness` is provided, the digest binds
    /// the permit to that opaque 32-byte witness (used by ERC-7683
    /// origin opens to lock the permit to a specific order).
    pub async fn digest(&self, req: Permit2DigestRequest) -> SdkResult<Permit2Digest> {
        self.rpc
            .call("tenzro_permit2Digest", serde_json::json!([req]))
            .await
    }

    /// Atomically verify a signed Permit2 message and consume its
    /// `(owner, nonce)` slot. Returns the consumed nonce position.
    pub async fn verify_and_consume(
        &self,
        req: Permit2VerifyAndConsumeRequest,
    ) -> SdkResult<Permit2VerifyAndConsumeResult> {
        self.rpc
            .call("tenzro_permit2VerifyAndConsume", serde_json::json!([req]))
            .await
    }

    /// Check whether a `(owner, nonce)` slot has been consumed.
    pub async fn nonce_used(&self, owner: &str, nonce: &str) -> SdkResult<Permit2NonceUsed> {
        self.rpc
            .call(
                "tenzro_permit2NonceUsed",
                serde_json::json!([{ "owner": owner, "nonce": nonce }]),
            )
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permit2DomainSeparator {
    #[serde(default)]
    pub domain_separator: String,
    #[serde(default)]
    pub verifying_contract: String,
    #[serde(default)]
    pub chain_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Permit2DigestRequest {
    pub chain_id: u64,
    pub owner: String,
    pub token: String,
    pub amount: String,
    pub spender: String,
    pub nonce: String,
    pub deadline: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness_type_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permit2Digest {
    #[serde(default)]
    pub digest: String,
    #[serde(default)]
    pub struct_hash: String,
    #[serde(default)]
    pub domain_separator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Permit2VerifyAndConsumeRequest {
    pub chain_id: u64,
    pub owner: String,
    pub token: String,
    pub amount: String,
    pub spender: String,
    pub nonce: String,
    pub deadline: u64,
    pub signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness_type_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permit2VerifyAndConsumeResult {
    #[serde(default)]
    pub consumed: bool,
    #[serde(default)]
    pub word_pos: String,
    #[serde(default)]
    pub bit_pos: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permit2NonceUsed {
    #[serde(default)]
    pub used: bool,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub nonce: String,
}
