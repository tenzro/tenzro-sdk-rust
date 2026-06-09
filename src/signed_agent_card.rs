//! A2A v1.0 SignedAgentCard canonical-hash client.
//!
//! Producers of agent cards hash + JWS-sign the canonical hash; relying
//! parties recompute the hash and verify the signature against the
//! domain owner's published key (typically `did:web:tenzro.network`).

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct SignedAgentCardClient {
    rpc: Arc<RpcClient>,
}

impl SignedAgentCardClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Compute the canonical SHA-256 hash for the given agent card
    /// JSON. Domain owners sign this hash via JWS to produce the
    /// `SignedAgentCard` envelope.
    pub async fn canonical_hash(
        &self,
        agent_card: serde_json::Value,
    ) -> SdkResult<SignedAgentCardHash> {
        self.rpc
            .call("tenzro_signedAgentCardCanonicalHash", agent_card)
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAgentCardHash {
    pub canonical_hash_hex: String,
    pub agent_card_name: String,
    pub agent_card_url: String,
    pub protocol_version: String,
    pub skills_count: usize,
}
