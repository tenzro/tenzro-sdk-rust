//! IVMS101 Travel Rule envelope client.
//!
//! Computes the canonical SHA-256 binding hash for an IVMS101 envelope
//! so a settlement receipt can be anchored to a specific
//! originator + beneficiary + VASP + transfer data record.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct Ivms101Client {
    rpc: Arc<RpcClient>,
}

impl Ivms101Client {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Compute the canonical hash for a raw IVMS101 envelope JSON
    /// payload. Returns the 32-byte hex hash + a summary of the
    /// originating + beneficiary VASP DIDs + asset + amount.
    pub async fn canonical_hash(&self, envelope: serde_json::Value) -> SdkResult<Ivms101HashResult> {
        self.rpc.call("tenzro_ivms101Hash", envelope).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ivms101HashResult {
    pub envelope_hash_hex: String,
    pub spec_version: String,
    pub originating_vasp_did: Option<String>,
    pub beneficiary_vasp_did: Option<String>,
    pub asset_caip19: String,
    pub amount_smallest_unit: String,
}
