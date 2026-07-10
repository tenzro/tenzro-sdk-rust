//! Intent routing and orchestration example for the Tenzro SDK
//!
//! This example demonstrates the three intent-driven entry points, each one
//! layer above the last:
//!
//! - `route_intent` — resolve an intent to the best model without naming one.
//!   Discovery only: no provider is dialed and no spend is recorded, but the
//!   per-DID budget gate and wallet-balance ceiling are still consulted.
//! - `chat_by_intent` — resolve an intent to a model and run a chat completion
//!   through the same path a named-model request takes.
//! - `orchestrate` — plan and run an ordered set of capabilities (models,
//!   skills, tools, agent/swarm delegation) to satisfy a natural-language goal.

use tenzro_sdk::{IntentParams, OrchestrateRequest, TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK Intent + Orchestration Example ===\n");

    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;
    let inference = client.inference();

    // 1. Route an intent to the best model. `budget` is a decimal string in the
    //    smallest TNZO unit; `optimize` is the cost-quality knob in [0.0, 1.0].
    println!("Routing a reasoning intent...");
    let route_params = IntentParams::new("reasoning")
        .with_budget("100000000000000000")
        .with_optimize(0.7)
        .with_quality_floor("strong")
        .with_tokens(400, 800);
    let decision = inference.route_intent(&route_params).await?;
    println!("Chosen model: {}", decision.model_id);
    println!("Tier: {}", decision.tier);
    println!("Estimated cost: {} TNZO (smallest unit)", decision.estimated_cost);
    println!("Fallback chain: {:?}", decision.fallback_chain);
    println!("Reason: {}\n", decision.reason);

    // 2. Route a research intent and run the chat completion in one call. The
    //    chosen route is attached to the response under `route`.
    println!("Running a research intent as a chat completion...");
    let chat_params = IntentParams::new("research").with_optimize(0.9);
    let messages = serde_json::json!([
        { "role": "user", "content": "Summarize the tradeoffs of BFT vs. Nakamoto consensus." }
    ]);
    let chat = inference.chat_by_intent(&chat_params, messages).await?;
    if let Some(route) = chat.get("route") {
        println!("Routed via: {}", route.get("model_id").and_then(|v| v.as_str()).unwrap_or("?"));
    }
    println!("Response: {}\n", serde_json::to_string_pretty(&chat)?);

    // 3. Orchestrate a multi-capability goal. When `payer_address` is set, the
    //    plan's aggregate estimated cost is checked against the payer's wallet
    //    balance before any step runs; an over-budget plan is rejected.
    println!("Orchestrating a multi-step goal...");
    let request = OrchestrateRequest::new(
        "Research recent decentralized-training results and draft a one-paragraph summary.",
    )
    .with_use_case("research")
    .with_budget("500000000000000000")
    .with_max_iterations(2);
    let outcome = inference.orchestrate(&request).await?;
    println!("Plan rationale: {}", outcome.plan.get("rationale").and_then(|v| v.as_str()).unwrap_or(""));
    println!("Iterations: {}", outcome.iterations);
    println!("Aggregate estimated cost: {} TNZO (smallest unit)", outcome.estimated_cost);
    for (i, step) in outcome.steps.iter().enumerate() {
        println!("  Step {} [{}]: {}", i + 1, step.kind, step.output);
    }

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
