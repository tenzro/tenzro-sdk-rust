//! ERC-8004 Trustless Agents Registry SDK for Tenzro Network
//!
//! This module provides client-side helpers for the three ERC-8004 registries
//! — IdentityRegistry, ReputationRegistry, and ValidationRegistry — enabling
//! on-chain agent discovery, feedback, and validation on any EVM chain.
//!
//! All encode methods return hex calldata that the caller can sign and
//! broadcast through any EVM wallet. Decoding helpers round-trip return
//! data from `eth_call` into typed structs.

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
/// let client = TenzroClient::new("https://rpc.tenzro.network").await?;
/// let erc8004 = client.erc8004();
///
/// // Derive a deterministic agent id
/// let id = erc8004.derive_agent_id("0xowner...", "0x01").await?;
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

    /// Derive a deterministic ERC-8004 agent id from owner + salt.
    ///
    /// Matches the on-chain `keccak256(abi.encode("TENZRO_ERC8004_AGENT", owner, salt))`
    /// computation used by IdentityRegistry.
    pub async fn derive_agent_id(
        &self,
        owner: &str,
        salt: &str,
    ) -> SdkResult<Erc8004AgentId> {
        self.rpc
            .call(
                "tenzro_erc8004DeriveAgentId",
                serde_json::json!({
                    "owner": owner,
                    "salt": salt,
                }),
            )
            .await
    }

    /// ABI-encode `IdentityRegistry.register(agentId, registrationDataURI, owner)`.
    pub async fn encode_register(
        &self,
        agent_id: &str,
        registration_data_uri: &str,
        owner: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeRegister",
                serde_json::json!({
                    "agent_id": agent_id,
                    "registration_data_uri": registration_data_uri,
                    "owner": owner,
                }),
            )
            .await
    }

    /// ABI-encode `IdentityRegistry.getAgent(agentId)`.
    pub async fn encode_get_agent(&self, agent_id: &str) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeGetAgent",
                serde_json::json!({
                    "agent_id": agent_id,
                }),
            )
            .await
    }

    /// Decode return data from a `getAgent()` eth_call into typed agent info.
    pub async fn decode_get_agent(&self, returndata: &str) -> SdkResult<Erc8004Agent> {
        self.rpc
            .call(
                "tenzro_erc8004DecodeGetAgent",
                serde_json::json!({
                    "returndata": returndata,
                }),
            )
            .await
    }

    /// ABI-encode `ReputationRegistry.submitFeedback(agentId, score, feedbackAuthId, feedbackURI)`.
    /// `score` must be 0-100.
    pub async fn encode_feedback(
        &self,
        agent_id: &str,
        score: u8,
        feedback_auth_id: &str,
        feedback_uri: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeFeedback",
                serde_json::json!({
                    "agent_id": agent_id,
                    "score": score,
                    "feedback_auth_id": feedback_auth_id,
                    "feedback_uri": feedback_uri,
                }),
            )
            .await
    }

    /// ABI-encode `ValidationRegistry.requestValidation(agentId, validatorId, requestURI, dataHash)`.
    pub async fn encode_request_validation(
        &self,
        agent_id: &str,
        validator_id: &str,
        request_uri: &str,
        data_hash: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeRequestValidation",
                serde_json::json!({
                    "agent_id": agent_id,
                    "validator_id": validator_id,
                    "request_uri": request_uri,
                    "data_hash": data_hash,
                }),
            )
            .await
    }

    /// ABI-encode `ValidationRegistry.submitValidation(dataHash, response, responseURI, tag)`.
    /// `response` is a 0-100 quality score.
    pub async fn encode_submit_validation(
        &self,
        data_hash: &str,
        response: u8,
        response_uri: &str,
        tag: &str,
    ) -> SdkResult<Erc8004Calldata> {
        self.rpc
            .call(
                "tenzro_erc8004EncodeSubmitValidation",
                serde_json::json!({
                    "data_hash": data_hash,
                    "response": response,
                    "response_uri": response_uri,
                    "tag": tag,
                }),
            )
            .await
    }
}

/// Deterministic agent id derived from owner + salt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc8004AgentId {
    /// 32-byte agent id as 0x-prefixed hex.
    #[serde(default)]
    pub agent_id: String,
    /// Owner address echoed back.
    #[serde(default)]
    pub owner: String,
    /// Salt used to derive the id.
    #[serde(default)]
    pub salt: String,
}

/// Hex-encoded ABI calldata ready for eth_sendTransaction or eth_call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc8004Calldata {
    /// 4-byte function selector as 0x-prefixed hex.
    #[serde(default)]
    pub selector: String,
    /// Full hex-encoded calldata (selector + abi-encoded args).
    #[serde(default)]
    pub calldata: String,
}

/// Decoded agent record returned by `IdentityRegistry.getAgent()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc8004Agent {
    /// Off-chain registration data URI (e.g. IPFS, HTTPS).
    #[serde(default)]
    pub registration_data_uri: String,
    /// Agent owner / controller address.
    #[serde(default)]
    pub owner: String,
}
