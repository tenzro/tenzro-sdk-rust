//! Agent SDK for Tenzro Network
//!
//! This module provides AI agent registration and interaction functionality.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use crate::types::AgentIdentity;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Agent client for AI agent operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let agent_client = client.agent();
///
/// // List all agents
/// let agents = agent_client.list_agents().await?;
/// for agent in agents {
///     println!("Agent: {} - {}", agent.agent_id, agent.name);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct AgentClient {
    rpc: Arc<RpcClient>,
}

impl AgentClient {
    /// Creates a new agent client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Registers a new AI agent on the network. The node provisions a
    /// server-side hybrid (FROST Ed25519 + ML-DSA-65) wallet for the agent
    /// and binds it to a fresh `did:tenzro:machine:` identity. Returns the
    /// `agent_id`, `wallet_address`, `tenzro_did`, and the agent's
    /// `classical_public_key` (32-byte Ed25519 hex) plus the
    /// `pq_verifying_key_len`. The classical key is what the
    /// `MessageRouter` will use to verify Ed25519 signatures on
    /// outbound `tenzro_sendAgentMessage` calls.
    ///
    /// `creator` is the hex-encoded address that owns this agent. Each
    /// element of `capabilities` may be a short tag (`"nlp"`, `"vision"`,
    /// `"code"`, `"data"`, `"blockchain"`, `"smart_contract"`,
    /// `"api_integration"`, `"coordination"`) — anything else becomes a
    /// `Custom` capability with that name.
    ///
    /// For self-custodial registration (no server-held keys), see
    /// [`Self::register_with_keys`].
    pub async fn register(
        &self,
        name: &str,
        creator: &str,
        capabilities: &[&str],
    ) -> SdkResult<RegisterAgentResponse> {
        self.rpc
            .call(
                "tenzro_registerAgent",
                serde_json::json!([{
                    "name": name,
                    "creator": creator,
                    "capabilities": capabilities,
                }]),
            )
            .await
    }

    /// BYOK registration: register an agent self-custodially with a
    /// caller-supplied hybrid keypair. Both `public_key` (32 bytes,
    /// Ed25519) and `pq_public_key` (1952 bytes, ML-DSA-65) must be
    /// supplied as hex (with or without `0x` prefix). The node performs
    /// no wallet provisioning — the caller retains full control of the
    /// signing keys, and is responsible for producing all
    /// `tenzro_sendAgentMessage` signatures off-node. The returned
    /// envelope sets `byok: true` and omits the
    /// `classical_public_key`/`pq_verifying_key_len` fields (the
    /// caller already has them).
    pub async fn register_with_keys(
        &self,
        name: &str,
        creator: &str,
        capabilities: &[&str],
        public_key: &str,
        pq_public_key: &str,
    ) -> SdkResult<RegisterAgentResponse> {
        self.rpc
            .call(
                "tenzro_registerAgent",
                serde_json::json!([{
                    "name": name,
                    "creator": creator,
                    "capabilities": capabilities,
                    "public_key": public_key,
                    "pq_public_key": pq_public_key,
                }]),
            )
            .await
    }

    /// Sends an unsigned `AgentMessage` from one registered agent to
    /// another. Only valid when the local node's `MessageRouter` has
    /// `enable_signing == false` (test/dev configs). On the production
    /// router this call is rejected with a signature-required error —
    /// use [`Self::send_message_signed`] instead.
    pub async fn send_message(
        &self,
        from: &str,
        to: &str,
        message: &str,
    ) -> SdkResult<AgentMessageResponse> {
        self.rpc
            .call(
                "tenzro_sendAgentMessage",
                serde_json::json!([{
                    "from": from,
                    "to": to,
                    "message": message,
                }]),
            )
            .await
    }

    /// Sends a hybrid-signed `AgentMessage`. Both signature legs are
    /// required when the router enforces signing (production default);
    /// half-signed messages are rejected to prevent downgrade attacks.
    ///
    /// The signing preimage is `SHA-256(AgentMessage::signing_data())`,
    /// which depends on `from`, `to`, the resolved sender/recipient
    /// wallet addresses, the message body, the message type, and the
    /// optional `reply_to`. Callers must construct the same preimage
    /// off-node and produce both an Ed25519 (64-byte) and an
    /// ML-DSA-65 (3309-byte) signature. Both signatures are passed as
    /// hex (with or without `0x` prefix).
    ///
    /// `reply_to` (if used) MUST be set on the message before signing,
    /// because it is part of `signing_data` — the SDK forwards it
    /// verbatim so the server reconstructs the same preimage the
    /// caller signed.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_message_signed(
        &self,
        from: &str,
        to: &str,
        message: &str,
        signature: &str,
        pq_signature: &str,
        message_type: Option<&str>,
        reply_to: Option<&str>,
    ) -> SdkResult<AgentMessageResponse> {
        self.rpc
            .call(
                "tenzro_sendAgentMessage",
                serde_json::json!([{
                    "from": from,
                    "to": to,
                    "message": message,
                    "signature": signature,
                    "pq_signature": pq_signature,
                    "message_type": message_type,
                    "reply_to": reply_to,
                }]),
            )
            .await
    }

    /// Lists all registered agents
    pub async fn list_agents(&self) -> SdkResult<Vec<AgentIdentity>> {
        self.rpc
            .call("tenzro_listAgents", serde_json::json!([]))
            .await
    }

    /// Delegates a task to an agent via A2A protocol
    pub async fn delegate_task(
        &self,
        _agent_id: &str,
        task_description: &str,
    ) -> SdkResult<DelegateTaskResponse> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tasks/send",
            "params": {
                "id": uuid::Uuid::new_v4().to_string(),
                "message": {
                    "role": "user",
                    "parts": [{ "type": "text", "text": task_description }],
                },
            },
            "id": 1,
        });

        self.rpc.post("/a2a", &body).await
    }

    /// Spawns a child agent under a parent agent
    pub async fn spawn_agent(
        &self,
        parent_id: &str,
        name: &str,
        capabilities: &[&str],
    ) -> SdkResult<SpawnAgentResponse> {
        self.rpc
            .call(
                "tenzro_spawnAgent",
                serde_json::json!([{
                    "parent_id": parent_id,
                    "name": name,
                    "capabilities": capabilities,
                }]),
            )
            .await
    }

    /// Runs an agentic task loop for an agent
    pub async fn run_agent_task(
        &self,
        agent_id: &str,
        task: &str,
        inference_url: Option<&str>,
    ) -> SdkResult<RunAgentTaskResponse> {
        self.rpc
            .call(
                "tenzro_runAgentTask",
                serde_json::json!([{
                    "agent_id": agent_id,
                    "task": task,
                    "inference_url": inference_url,
                }]),
            )
            .await
    }

    /// Creates a swarm of member agents under an orchestrator
    pub async fn create_swarm(
        &self,
        orchestrator_id: &str,
        members: Vec<SwarmMemberSpec>,
        max_members: Option<usize>,
        task_timeout_secs: Option<u64>,
        parallel: Option<bool>,
    ) -> SdkResult<CreateSwarmResponse> {
        self.rpc
            .call(
                "tenzro_createSwarm",
                serde_json::json!([{
                    "orchestrator_id": orchestrator_id,
                    "members": members,
                    "max_members": max_members,
                    "task_timeout_secs": task_timeout_secs,
                    "parallel": parallel,
                }]),
            )
            .await
    }

    /// Gets the current status of a swarm
    pub async fn get_swarm_status(&self, swarm_id: &str) -> SdkResult<SwarmStatus> {
        self.rpc
            .call(
                "tenzro_getSwarmStatus",
                serde_json::json!([{ "swarm_id": swarm_id }]),
            )
            .await
    }

    /// Terminates a swarm and all its member agents
    pub async fn terminate_swarm(&self, swarm_id: &str) -> SdkResult<TerminateSwarmResponse> {
        self.rpc
            .call(
                "tenzro_terminateSwarm",
                serde_json::json!([{ "swarm_id": swarm_id }]),
            )
            .await
    }

    /// Spawns an agent from a marketplace template.
    ///
    /// When `parent_machine_did` is `Some`, the spawned agent's effective
    /// delegation scope is the strict intersection of the parent's scope
    /// and the template's spec — the child can never be broader than its
    /// parent on any axis (numeric ceilings, allow-lists, time bound).
    pub async fn spawn_agent_template(
        &self,
        template_id: &str,
        display_name: Option<&str>,
        context: Option<&str>,
        parent_machine_did: Option<&str>,
    ) -> SdkResult<SpawnAgentTemplateResponse> {
        self.rpc
            .call(
                "tenzro_spawnAgentTemplate",
                serde_json::json!([{
                    "template_id": template_id,
                    "display_name": display_name,
                    "context": context,
                    "parent_machine_did": parent_machine_did,
                }]),
            )
            .await
    }

    /// Runs an agent template through an iterative execution loop
    pub async fn run_agent_template(
        &self,
        agent_id: &str,
        max_iterations: Option<u32>,
        dry_run: bool,
    ) -> SdkResult<RunAgentTemplateReport> {
        self.rpc
            .call(
                "tenzro_runAgentTemplate",
                serde_json::json!([{
                    "agent_id": agent_id,
                    "max_iterations": max_iterations,
                    "dry_run": dry_run,
                }]),
            )
            .await
    }

    /// Downloads an agent template definition from the marketplace
    pub async fn download_agent_template(
        &self,
        template_id: &str,
    ) -> SdkResult<AgentTemplateDefinition> {
        self.rpc
            .call(
                "tenzro_downloadAgentTemplate",
                serde_json::json!([{ "template_id": template_id }]),
            )
            .await
    }

    /// Discovers available models with optional filters
    pub async fn discover_models(&self, modality: Option<&str>, serving_only: Option<bool>, query: Option<&str>) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_discoverModels",
                serde_json::json!([{"modality": modality, "serving_only": serving_only, "query": query}]),
            )
            .await
    }

    /// Discovers available agents with optional capability filter
    pub async fn discover_agents(&self, capability: Option<&str>) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_discoverAgents",
                serde_json::json!([{"capability": capability}]),
            )
            .await
    }

    /// Spawns an agent with a specific skill attached
    pub async fn spawn_agent_with_skill(&self, parent_id: &str, name: &str, skill_id: &str, capabilities: Option<Vec<String>>) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_spawnAgentWithSkill",
                serde_json::json!([{"parent_id": parent_id, "name": name, "skill_id": skill_id, "capabilities": capabilities}]),
            )
            .await
    }

    /// Funds an agent's wallet from a source address
    pub async fn fund_agent(&self, agent_id: &str, from_address: &str, amount_tnzo: f64) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_fundAgent",
                serde_json::json!([{"agent_id": agent_id, "from_address": from_address, "amount_tnzo": amount_tnzo}]),
            )
            .await
    }

    /// Swaps tokens for an agent
    pub async fn swap_token(&self, agent_id: &str, from_token: &str, to_token: &str, amount: &str, chain: Option<&str>) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_swapToken",
                serde_json::json!([{"agent_id": agent_id, "from_token": from_token, "to_token": to_token, "amount": amount, "chain": chain}]),
            )
            .await
    }

    /// Runs the full agent payment pipeline for inference
    pub async fn agent_pay_for_inference(&self, agent_id: &str, model_id: &str, prompt: &str, max_tokens: Option<u32>) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_agentPayForInference",
                serde_json::json!([{"agent_id": agent_id, "model_id": model_id, "prompt": prompt, "max_tokens": max_tokens}]),
            )
            .await
    }

    /// Sets the gas policy for an agent
    pub async fn set_gas_policy(
        &self,
        agent_id: &str,
        policy: GasPolicy,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_setAgentGasPolicy",
                serde_json::json!([{
                    "agent_id": agent_id,
                    "policy_type": policy.policy_type,
                    "max_gas": policy.max_gas,
                }]),
            )
            .await
    }

    /// Gets the current gas policy for an agent
    pub async fn get_gas_policy(&self, agent_id: &str) -> SdkResult<GasPolicy> {
        self.rpc
            .call(
                "tenzro_getAgentGasPolicy",
                serde_json::json!([{
                    "agent_id": agent_id,
                }]),
            )
            .await
    }

    /// Updates an existing agent template on the marketplace
    pub async fn update_agent_template(
        &self,
        template_id: &str,
        params: UpdateAgentTemplateParams,
    ) -> SdkResult<AgentTemplateDefinition> {
        self.rpc
            .call(
                "tenzro_updateAgentTemplate",
                serde_json::json!([{
                    "template_id": template_id,
                    "name": params.name,
                    "description": params.description,
                    "system_prompt": params.system_prompt,
                    "tags": params.tags,
                    "pricing": params.pricing,
                    "template_type": params.template_type,
                }]),
            )
            .await
    }

    /// Operational suspend (Active → Suspended). The reversible counterpart
    /// of [`resume_agent`]. Distinct from the kill-switch `pause`/`quarantine`
    /// axes which require signed `PauseAgent` / `QuarantineAgent` transactions.
    pub async fn suspend_agent(
        &self,
        agent_id: &str,
        reason: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_suspendAgent",
                serde_json::json!({
                    "agent_id": agent_id,
                    "reason": reason,
                }),
            )
            .await
    }

    /// Recover a Suspended agent back to Active. Used to recover from
    /// auto-suspend (heartbeat monitor) or manual [`suspend_agent`].
    pub async fn resume_agent(&self, agent_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_resumeAgent",
                serde_json::json!({ "agent_id": agent_id }),
            )
            .await
    }

    /// Send a liveness heartbeat for an agent. The heartbeat monitor uses
    /// `last_heartbeat` to decide whether to auto-suspend on the next sweep.
    pub async fn agent_heartbeat(&self, agent_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_agentHeartbeat",
                serde_json::json!({ "agent_id": agent_id }),
            )
            .await
    }
}

/// Response from agent registration. Mirrors the JSON shape returned by
/// `tenzro_registerAgent`. The `byok` flag distinguishes the two
/// registration modes — when `true`, `classical_public_key` and
/// `pq_verifying_key_len` are absent because the keys are caller-held.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterAgentResponse {
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub creator: String,
    #[serde(default)]
    pub wallet_address: String,
    #[serde(default)]
    pub capabilities: usize,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub tenzro_did: String,
    #[serde(default)]
    pub registration_fee: String,
    /// Hex-encoded 32-byte Ed25519 verifying key. Provisioner mode only.
    #[serde(default)]
    pub classical_public_key: String,
    /// Length of the ML-DSA-65 verifying key (1952 for production).
    /// Provisioner mode only.
    #[serde(default)]
    pub pq_verifying_key_len: usize,
    /// `true` for self-custodial registrations where the caller supplied
    /// both the classical and PQ public keys; `false` for the
    /// node-provisioned hybrid wallet path.
    #[serde(default)]
    pub byok: bool,
}

/// Response from sending an agent message. Mirrors the JSON shape
/// returned by `tenzro_sendAgentMessage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessageResponse {
    #[serde(default)]
    pub message_id: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub status: String,
    /// Unix-millis timestamp the server stamped on the constructed
    /// `AgentMessage` before validating the signature.
    #[serde(default)]
    pub timestamp: u64,
    /// `true` when the server attached both signature legs to the
    /// message; `false` when the message was accepted unsigned (only
    /// possible on `enable_signing == false` routers).
    #[serde(default)]
    pub signed: bool,
}

/// Response from delegating a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateTaskResponse {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub status: String,
}

/// Response from spawning a child agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnAgentResponse {
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub parent_id: String,
    #[serde(default)]
    pub name: String,
}

/// Response from running an agentic task loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunAgentTaskResponse {
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub result: String,
}

/// Spec for a swarm member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmMemberSpec {
    pub name: String,
    pub capabilities: Vec<String>,
}

/// Response from creating a swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSwarmResponse {
    #[serde(default)]
    pub swarm_id: String,
    #[serde(default)]
    pub orchestrator_id: String,
}

/// Status of a single swarm member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmMemberInfo {
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub status: String,
    pub result: Option<String>,
}

/// Full swarm status snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmStatus {
    #[serde(default)]
    pub swarm_id: String,
    #[serde(default)]
    pub orchestrator_id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub member_count: usize,
    #[serde(default)]
    pub members: Vec<SwarmMemberInfo>,
}

/// Response from terminating a swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminateSwarmResponse {
    #[serde(default)]
    pub swarm_id: String,
    #[serde(default)]
    pub status: String,
}

/// Response from spawning an agent from a template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnAgentTemplateResponse {
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub template_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub status: String,
}

/// Report from running an agent template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunAgentTemplateReport {
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub iterations_completed: u32,
    #[serde(default)]
    pub result: String,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub status: String,
}

/// Full agent template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplateDefinition {
    #[serde(default)]
    pub template_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub template_type: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub creator: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub pricing: serde_json::Value,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub version: String,
}

/// Gas policy for agent blockchain operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasPolicy {
    #[serde(default)]
    pub policy_type: String,
    #[serde(default)]
    pub max_gas: Option<u128>,
}

/// Parameters for updating an agent template
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateAgentTemplateParams {
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub tags: Option<Vec<String>>,
    pub pricing: Option<serde_json::Value>,
    pub template_type: Option<String>,
}
