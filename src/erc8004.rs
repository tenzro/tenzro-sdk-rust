//! ERC-8004 Trustless Agents Registry SDK for Tenzro Network.
//!
//! Client-side helpers for the three ERC-8004 registries —
//! IdentityRegistry, ReputationRegistry, and ValidationRegistry —
//! covering the canonical v0.6+ surface (mutators + reads) so the SDK
//! is at parity with the node's RPC layer and the EVM precompile at
//! `0x101a` / `0x101b` / `0x101c`.
//!
//! All `encode_*` methods return hex calldata that the caller can sign
//! and broadcast via `eth_sendRawTransaction`. `decode_*` helpers
//! round-trip return data from `eth_call` into typed structs.
//!
//! Tenzro derives `agentId = keccak256(utf8(did_string))` so the same
//! calldata works against either the native registry or the Ethereum
//! mirror.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// ERC-8004 (Trustless Agents Registry) client.
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::TenzroClient;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = TenzroClient::new("https://rpc.tenzro.xyz").await?;
/// let erc8004 = client.erc8004();
///
/// // Derive a deterministic agent id from a Tenzro DID.
/// let id = erc8004.derive_agent_id("did:tenzro:machine:abc123").await?;
/// println!("agentId = {}", id.agent_id);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Erc8004Client {
    rpc: Arc<RpcClient>,
}

impl Erc8004Client {
    /// Creates a new ERC-8004 client.
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    // ---------------------------------------------------------------
    // Identity registry — base surface
    // ---------------------------------------------------------------

    /// Derive a canonical ERC-8004 `agentId = keccak256(utf8(did))`.
    pub async fn derive_agent_id(&self, did: &str) -> SdkResult<Erc8004AgentId> {
        self.rpc
            .call(
                "tenzro_erc8004DeriveAgentId",
                serde_json::json!({ "did": did }),
            )
            .await
    }

    /// ABI-encode `IdentityRegistry.registerAgent(bytes32 agentId, address agentAddress, string metadataURI)`.
    pub async fn encode_register(
        &self,
        did: &str,
        agent_address: &str,
        metadata_uri: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeRegister",
                serde_json::json!({
                    "did": did,
                    "agent_address": agent_address,
                    "metadata_uri": metadata_uri,
                }),
            )
            .await
    }

    /// ABI-encode `IdentityRegistry.getAgent(bytes32 agentId)`.
    pub async fn encode_get_agent(&self, agent_id: &str) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeGetAgent",
                serde_json::json!({ "agent_id": agent_id }),
            )
            .await
    }

    /// Decode `(address, string)` returndata from `getAgent()` into a typed record.
    pub async fn decode_get_agent(&self, return_data: &str) -> SdkResult<Erc8004Agent> {
        self.rpc
            .call(
                "tenzro_erc8004DecodeGetAgent",
                serde_json::json!({ "return_data": return_data }),
            )
            .await
    }

    // ---------------------------------------------------------------
    // Identity registry — v0.6+ mutators
    // ---------------------------------------------------------------

    /// ABI-encode `IdentityRegistry.setAgentURI(uint256 agentId, string metadataURI)`.
    pub async fn encode_set_agent_uri(
        &self,
        agent_id: &str,
        metadata_uri: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeSetAgentURI",
                serde_json::json!({
                    "agent_id": agent_id,
                    "metadata_uri": metadata_uri,
                }),
            )
            .await
    }

    /// ABI-encode `IdentityRegistry.setAgentWallet(uint256 agentId, address newWallet, uint256 deadline, bytes signature)`.
    pub async fn encode_set_agent_wallet(
        &self,
        agent_id: &str,
        new_wallet: &str,
        deadline: u64,
        signature: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeSetAgentWallet",
                serde_json::json!({
                    "agent_id": agent_id,
                    "new_wallet": new_wallet,
                    "deadline": deadline,
                    "signature": signature,
                }),
            )
            .await
    }

    /// ABI-encode `IdentityRegistry.setMetadata(uint256 agentId, string metadataKey, bytes metadataValue)`.
    pub async fn encode_set_metadata(
        &self,
        agent_id: &str,
        metadata_key: &str,
        metadata_value: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeSetMetadata",
                serde_json::json!({
                    "agent_id": agent_id,
                    "metadata_key": metadata_key,
                    "metadata_value": metadata_value,
                }),
            )
            .await
    }

    // ---------------------------------------------------------------
    // Identity registry — v0.6+ reads
    // ---------------------------------------------------------------

    /// ABI-encode `IdentityRegistry.getMetadata(uint256 agentId, string metadataKey)`.
    pub async fn encode_get_metadata(
        &self,
        agent_id: &str,
        metadata_key: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeGetMetadata",
                serde_json::json!({
                    "agent_id": agent_id,
                    "metadata_key": metadata_key,
                }),
            )
            .await
    }

    /// Decode `bytes` returndata from `getMetadata()` into hex bytes.
    pub async fn decode_get_metadata(&self, return_data: &str) -> SdkResult<Erc8004Metadata> {
        self.rpc
            .call(
                "tenzro_erc8004DecodeGetMetadata",
                serde_json::json!({ "return_data": return_data }),
            )
            .await
    }

    /// ABI-encode `IdentityRegistry.getAgentURI(uint256 agentId)`.
    pub async fn encode_get_agent_uri(&self, agent_id: &str) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeGetAgentURI",
                serde_json::json!({ "agent_id": agent_id }),
            )
            .await
    }

    /// ABI-encode `IdentityRegistry.getAgentWallet(uint256 agentId)`.
    pub async fn encode_get_agent_wallet(&self, agent_id: &str) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeGetAgentWallet",
                serde_json::json!({ "agent_id": agent_id }),
            )
            .await
    }

    // ---------------------------------------------------------------
    // Reputation registry
    // ---------------------------------------------------------------

    /// ABI-encode `ReputationRegistry.submitFeedback(bytes32 subjectAgentId, int8 rating, string contextURI)`.
    pub async fn encode_feedback(
        &self,
        subject_agent_id: &str,
        rating: i8,
        context_uri: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeFeedback",
                serde_json::json!({
                    "subject_agent_id": subject_agent_id,
                    "rating": rating,
                    "context_uri": context_uri,
                }),
            )
            .await
    }

    /// ABI-encode `ReputationRegistry.getFeedback(bytes32 subject, uint256 index)`.
    pub async fn encode_get_feedback(
        &self,
        subject_agent_id: &str,
        index: u64,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeGetFeedback",
                serde_json::json!({
                    "subject_agent_id": subject_agent_id,
                    "index": index,
                }),
            )
            .await
    }

    /// ABI-encode `ReputationRegistry.getFeedbackCount(bytes32 subject)`.
    pub async fn encode_get_feedback_count(
        &self,
        subject_agent_id: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeGetFeedbackCount",
                serde_json::json!({ "subject_agent_id": subject_agent_id }),
            )
            .await
    }

    /// ABI-encode `ReputationRegistry.revokeFeedback(uint256 agentId, bytes32 feedbackId)` (v0.6+).
    pub async fn encode_revoke_feedback(
        &self,
        agent_id: &str,
        feedback_id: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeRevokeFeedback",
                serde_json::json!({
                    "agent_id": agent_id,
                    "feedback_id": feedback_id,
                }),
            )
            .await
    }

    /// ABI-encode `ReputationRegistry.appendResponse(uint256 agentId, bytes32 feedbackId, string responseURI)` (v0.6+).
    pub async fn encode_append_response(
        &self,
        agent_id: &str,
        feedback_id: &str,
        response_uri: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeAppendResponse",
                serde_json::json!({
                    "agent_id": agent_id,
                    "feedback_id": feedback_id,
                    "response_uri": response_uri,
                }),
            )
            .await
    }

    /// ABI-encode `ReputationRegistry.isFeedbackRevoked(uint256 agentId, bytes32 feedbackId)` (v0.6+).
    pub async fn encode_is_feedback_revoked(
        &self,
        agent_id: &str,
        feedback_id: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeIsFeedbackRevoked",
                serde_json::json!({
                    "agent_id": agent_id,
                    "feedback_id": feedback_id,
                }),
            )
            .await
    }

    /// ABI-encode `ReputationRegistry.getFeedbackResponses(uint256 agentId, bytes32 feedbackId)` (v0.6+).
    pub async fn encode_get_feedback_responses(
        &self,
        agent_id: &str,
        feedback_id: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeGetFeedbackResponses",
                serde_json::json!({
                    "agent_id": agent_id,
                    "feedback_id": feedback_id,
                }),
            )
            .await
    }

    // ---------------------------------------------------------------
    // Validation registry
    // ---------------------------------------------------------------

    /// ABI-encode `ValidationRegistry.validationRequest(address validatorAddress, uint256 agentId, string requestURI, bytes32 requestHash)`.
    pub async fn encode_validation_request(
        &self,
        validator_address: &str,
        agent_id: &str,
        request_uri: &str,
        request_hash: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeValidationRequest",
                serde_json::json!({
                    "validator_address": validator_address,
                    "agent_id": agent_id,
                    "request_uri": request_uri,
                    "request_hash": request_hash,
                }),
            )
            .await
    }

    /// ABI-encode `ValidationRegistry.validationResponse(bytes32 requestHash, uint8 response, string responseURI, bytes32 responseHash, string tag)`.
    /// `response` is a 0-100 quality score per ERC-8004.
    pub async fn encode_validation_response(
        &self,
        request_hash: &str,
        response: u8,
        response_uri: &str,
        response_hash: &str,
        tag: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeValidationResponse",
                serde_json::json!({
                    "request_hash": request_hash,
                    "response": response,
                    "response_uri": response_uri,
                    "response_hash": response_hash,
                    "tag": tag,
                }),
            )
            .await
    }

    /// ABI-encode `ValidationRegistry.getValidation(bytes32 requestHash)`.
    pub async fn encode_get_validation(
        &self,
        request_hash: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeGetValidation",
                serde_json::json!({ "request_hash": request_hash }),
            )
            .await
    }
}

/// Deterministic agent id derived from a Tenzro DID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc8004AgentId {
    /// DID echoed back.
    #[serde(default)]
    pub did: String,
    /// 32-byte agent id as 0x-prefixed hex.
    #[serde(default)]
    pub agent_id: String,
}

/// Hex-encoded ABI calldata ready for `eth_sendRawTransaction` / `eth_call`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc8004Calldata {
    /// Full hex-encoded calldata (selector + abi-encoded args).
    #[serde(default)]
    pub calldata: String,
    /// Optional echoed agent id for `encode_register`.
    #[serde(default)]
    pub agent_id: String,
}

/// Decoded agent record returned by `IdentityRegistry.getAgent()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc8004Agent {
    /// Agent owner / controller address.
    #[serde(default)]
    pub agent_address: String,
    /// Off-chain metadata URI (e.g. IPFS, HTTPS).
    #[serde(default)]
    pub metadata_uri: String,
}

/// Decoded `bytes` value returned by `IdentityRegistry.getMetadata()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc8004Metadata {
    /// Hex-encoded value (`0x` prefix), or empty hex when the entry is unset.
    #[serde(default)]
    pub metadata_value: String,
}
