//! Inference SDK for Tenzro Network
//!
//! This module provides AI model inference functionality.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use crate::types::ModelInfo;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Inference client for AI model operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let inference = client.inference();
///
/// // List available models
/// let models = inference.list_models().await?;
/// println!("Found {} models", models.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct InferenceClient {
    rpc: Arc<RpcClient>,
}

impl InferenceClient {
    /// Creates a new inference client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Lists all available models on the network
    pub async fn list_models(&self) -> SdkResult<Vec<ModelInfo>> {
        self.rpc
            .call("tenzro_listModels", serde_json::json!([]))
            .await
    }

    /// Submits an inference request to a model
    pub async fn request(
        &self,
        model_id: &str,
        input: &str,
        max_tokens: Option<u32>,
    ) -> SdkResult<InferenceResult> {
        let mut params = serde_json::json!({
            "model_id": model_id,
            "input": input,
        });

        if let Some(mt) = max_tokens {
            params["max_tokens"] = serde_json::json!(mt);
        }

        self.rpc
            .call("tenzro_inferenceRequest", serde_json::json!([params]))
            .await
    }

    /// Estimates the cost of an inference request
    pub async fn estimate_cost(&self, _model_id: &str, input_tokens: u32) -> SdkResult<u64> {
        let cost_per_input_token = 10u64;
        let cost_per_output_token = 20u64;
        let estimated_output_tokens = input_tokens * 2;

        let total_cost = (input_tokens as u64 * cost_per_input_token)
            + (estimated_output_tokens as u64 * cost_per_output_token);

        Ok(total_cost)
    }

    /// Resolves an intent to the best model without naming one. Discovery
    /// only: no provider is dialed and no spend is recorded, but the per-DID
    /// budget gate and the wallet-balance ceiling are still consulted, so an
    /// unaffordable request is rejected at discovery time. Feed the returned
    /// `model_id` into [`InferenceClient::request`] or a chat surface to run
    /// it.
    pub async fn route_intent(&self, params: &IntentParams) -> SdkResult<RouteDecision> {
        self.rpc
            .call("tenzro_routeIntent", serde_json::json!([params.to_json()]))
            .await
    }

    /// Resolves an intent to a model and runs a chat completion through the
    /// same path a named-model request takes. `messages` is the rich chat
    /// shape (role/content turns). The chosen route is attached to the
    /// response under `route`. Returns the raw chat response object.
    pub async fn chat_by_intent(
        &self,
        params: &IntentParams,
        messages: serde_json::Value,
    ) -> SdkResult<serde_json::Value> {
        let mut body = params.to_json();
        body["messages"] = messages;
        self.rpc
            .call("tenzro_chatByIntent", serde_json::json!([body]))
            .await
    }

    /// Satisfies a natural-language intent by planning and running an ordered
    /// set of capabilities — models, registered skills, registered tools, and
    /// agent/swarm delegation. One layer above [`InferenceClient::chat_by_intent`]:
    /// that resolves a single model; this composes models with the skill/tool
    /// registries and the swarm runtime. When `payer_address` is set, the
    /// plan's aggregate estimated cost is checked against the payer's wallet
    /// balance before any step runs; an over-budget plan is rejected.
    pub async fn orchestrate(
        &self,
        request: &OrchestrateRequest,
    ) -> SdkResult<OrchestrationOutcome> {
        self.rpc
            .call("tenzro_orchestrate", serde_json::json!([request.to_json()]))
            .await
    }

    /// Reads the inference router's live metrics snapshot: total requests
    /// routed, hedges dispatched, hedges won, and requests abandoned on the
    /// whole-request deadline. Returns the raw metrics object.
    pub async fn router_metrics(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_getRouterMetrics", serde_json::json!({}))
            .await
    }

    /// Look up the cached provenance manifest for generated content by its
    /// 32-byte hex `content_hash` (with or without `0x` prefix). This is the
    /// machine-readable synthetic-content marker per EU AI Act Art. 50(2).
    /// The node returns JSON-RPC `-32004` when no manifest is cached for the
    /// hash — surfaces as `SdkError::RpcError`.
    pub async fn get_provenance(
        &self,
        content_hash: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getProvenance",
                serde_json::json!({ "content_hash": content_hash }),
            )
            .await
    }
}

/// The shared intent fields consumed by `route_intent` and `chat_by_intent`.
///
/// Build with [`IntentParams::new`] and refine with the `with_*` setters.
/// `budget` is a decimal string in the smallest TNZO unit (a u128 exceeds
/// JSON's safe integer range).
#[derive(Debug, Clone, Default)]
pub struct IntentParams {
    /// Use case: `chat`, `code`, `reasoning`, `research`, `summarize`,
    /// `extract`, or `embed`.
    pub use_case: String,
    /// Per-request cost cap, smallest TNZO unit, decimal string.
    pub budget: Option<String>,
    /// Cost-quality knob in `[0.0, 1.0]`.
    pub optimize: Option<f64>,
    /// Reject any model below this tier (`cheap` | `strong`).
    pub quality_floor: Option<String>,
    /// Estimated input tokens for cost estimation.
    pub est_input_tokens: Option<u64>,
    /// Estimated output tokens for cost estimation.
    pub est_output_tokens: Option<u64>,
    /// Payer DID — enables the per-DID budget gate.
    pub payer_did: Option<String>,
    /// Payer wallet address (hex) — enables the wallet-balance ceiling.
    pub payer_address: Option<String>,
}

impl IntentParams {
    /// Builds intent params for a use case with no other constraints.
    pub fn new(use_case: impl Into<String>) -> Self {
        Self {
            use_case: use_case.into(),
            ..Default::default()
        }
    }

    /// Sets the per-request budget cap (decimal string, smallest TNZO unit).
    #[must_use]
    pub fn with_budget(mut self, budget: impl Into<String>) -> Self {
        self.budget = Some(budget.into());
        self
    }

    /// Sets the cost-quality knob in `[0.0, 1.0]`.
    #[must_use]
    pub fn with_optimize(mut self, optimize: f64) -> Self {
        self.optimize = Some(optimize);
        self
    }

    /// Sets the minimum quality tier (`cheap` | `strong`).
    #[must_use]
    pub fn with_quality_floor(mut self, floor: impl Into<String>) -> Self {
        self.quality_floor = Some(floor.into());
        self
    }

    /// Sets the token estimates used for cost estimation.
    #[must_use]
    pub fn with_tokens(mut self, input: u64, output: u64) -> Self {
        self.est_input_tokens = Some(input);
        self.est_output_tokens = Some(output);
        self
    }

    /// Sets the payer DID (enables the per-DID budget gate).
    #[must_use]
    pub fn with_payer_did(mut self, did: impl Into<String>) -> Self {
        self.payer_did = Some(did.into());
        self
    }

    /// Sets the payer wallet address (enables the wallet-balance ceiling).
    #[must_use]
    pub fn with_payer_address(mut self, addr: impl Into<String>) -> Self {
        self.payer_address = Some(addr.into());
        self
    }

    /// Serializes into the node's snake_case RPC param object.
    fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::json!({ "use_case": self.use_case });
        if let Some(ref b) = self.budget {
            obj["budget"] = serde_json::json!(b);
        }
        if let Some(o) = self.optimize {
            obj["optimize"] = serde_json::json!(o);
        }
        if let Some(ref f) = self.quality_floor {
            obj["quality_floor"] = serde_json::json!(f);
        }
        if let Some(i) = self.est_input_tokens {
            obj["est_input_tokens"] = serde_json::json!(i);
        }
        if let Some(o) = self.est_output_tokens {
            obj["est_output_tokens"] = serde_json::json!(o);
        }
        if let Some(ref d) = self.payer_did {
            obj["payer_did"] = serde_json::json!(d);
        }
        if let Some(ref a) = self.payer_address {
            obj["payer_address"] = serde_json::json!(a);
        }
        obj
    }
}

/// The model-selection decision returned by `route_intent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    /// Chosen model id.
    #[serde(default)]
    pub model_id: String,
    /// Selected tier label.
    #[serde(default)]
    pub tier: String,
    /// Estimated cost, smallest TNZO unit, decimal string.
    #[serde(default)]
    pub estimated_cost: String,
    /// Ordered fallback model ids.
    #[serde(default)]
    pub fallback_chain: Vec<String>,
    /// Human-readable selection reason.
    #[serde(default)]
    pub reason: String,
}

/// The intent→capabilities request for `orchestrate`.
///
/// `intent` is the natural-language goal; the remaining fields narrow model
/// selection and the budget ceiling. Build with [`OrchestrateRequest::new`].
#[derive(Debug, Clone, Default)]
pub struct OrchestrateRequest {
    /// Natural-language goal to satisfy.
    pub intent: String,
    /// Primary use-case hint. Defaults to `chat` on the node when absent.
    pub use_case: Option<String>,
    /// Per-request cost cap, smallest TNZO unit, decimal string.
    pub budget: Option<String>,
    /// Payer DID — enables the per-DID budget gate on model steps.
    pub payer_did: Option<String>,
    /// Payer wallet address (hex) — enables the plan-level wallet ceiling.
    pub payer_address: Option<String>,
    /// Max re-plan iterations, clamped to `[1, 6]` on the node.
    pub max_iterations: Option<u32>,
}

impl OrchestrateRequest {
    /// Builds an orchestration request for `intent` with no other constraints.
    pub fn new(intent: impl Into<String>) -> Self {
        Self {
            intent: intent.into(),
            ..Default::default()
        }
    }

    /// Sets the primary use-case hint.
    #[must_use]
    pub fn with_use_case(mut self, use_case: impl Into<String>) -> Self {
        self.use_case = Some(use_case.into());
        self
    }

    /// Sets the per-request budget cap (decimal string, smallest TNZO unit).
    #[must_use]
    pub fn with_budget(mut self, budget: impl Into<String>) -> Self {
        self.budget = Some(budget.into());
        self
    }

    /// Sets the payer DID.
    #[must_use]
    pub fn with_payer_did(mut self, did: impl Into<String>) -> Self {
        self.payer_did = Some(did.into());
        self
    }

    /// Sets the payer wallet address (enables the plan-level wallet ceiling).
    #[must_use]
    pub fn with_payer_address(mut self, addr: impl Into<String>) -> Self {
        self.payer_address = Some(addr.into());
        self
    }

    /// Sets the re-plan iteration bound.
    #[must_use]
    pub fn with_max_iterations(mut self, n: u32) -> Self {
        self.max_iterations = Some(n);
        self
    }

    /// Serializes into the node's snake_case RPC param object.
    fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::json!({ "intent": self.intent });
        if let Some(ref u) = self.use_case {
            obj["use_case"] = serde_json::json!(u);
        }
        if let Some(ref b) = self.budget {
            obj["budget"] = serde_json::json!(b);
        }
        if let Some(ref d) = self.payer_did {
            obj["payer_did"] = serde_json::json!(d);
        }
        if let Some(ref a) = self.payer_address {
            obj["payer_address"] = serde_json::json!(a);
        }
        if let Some(n) = self.max_iterations {
            obj["max_iterations"] = serde_json::json!(n);
        }
        obj
    }
}

/// One executed capability step in an orchestration outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationStep {
    /// Capability kind (`model` | `skill` | `tool` | `agent` | `swarm`).
    #[serde(default)]
    pub kind: String,
    /// Free-form output text for the step.
    #[serde(default)]
    pub output: String,
    /// Structured detail (model id chosen, member replies, etc.).
    #[serde(default)]
    pub detail: serde_json::Value,
}

/// The outcome of an orchestration: the final plan, per-step results, and
/// aggregate accounting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationOutcome {
    /// The plan that ran (post any re-planning).
    #[serde(default)]
    pub plan: serde_json::Value,
    /// One result per executed step, in order.
    #[serde(default)]
    pub steps: Vec<OrchestrationStep>,
    /// Aggregate estimated cost, smallest TNZO unit, decimal string.
    #[serde(default)]
    pub estimated_cost: String,
    /// Number of re-plan iterations consumed (1 = single-shot).
    #[serde(default)]
    pub iterations: u32,
}

/// Inference result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    /// Model output text
    #[serde(default)]
    pub output: String,
    /// Request ID
    #[serde(default)]
    pub request_id: String,
    /// Model ID that processed the request
    #[serde(default)]
    pub model_id: String,
    /// Tokens used
    #[serde(default)]
    pub tokens_used: u64,
}
