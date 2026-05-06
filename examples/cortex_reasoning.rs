//! Cortex recurrent-depth reasoning example for the Tenzro SDK.
//!
//! Demonstrates:
//! - Listing locally-registered Cortex workers
//! - Listing remote Cortex workers (gossip-discovered)
//! - Submitting a Standard-tier reasoning request
//! - Submitting an Institutional-tier request with TEE attestation
//! - Inspecting the signed CortexReceipt
//! - Computing cost via CortexPricing

use tenzro_sdk::cortex::{
    AttestationRequirement, CortexPricing, CortexRequest, ReasoningTier,
};
use tenzro_sdk::{TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK Cortex Reasoning Example ===\n");

    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;
    let cortex = client.cortex();

    // 1. Local worker discovery
    println!("Listing locally-registered Cortex workers...");
    match cortex.list_workers().await {
        Ok(list) => {
            println!("  Local workers: {}", list.count);
            for w in list.workers.iter().take(5) {
                println!("    {}", w);
            }
        }
        Err(e) => println!("  (failed: {e})"),
    }
    println!();

    // 2. Remote worker discovery (gossip-learned)
    println!("Listing remote Cortex workers learned via gossip...");
    match cortex.list_remote_workers().await {
        Ok(list) => {
            println!("  Remote workers: {}", list.count);
            for w in list.workers.iter().take(5) {
                println!("    {}", w);
            }
        }
        Err(e) => println!("  (failed: {e})"),
    }
    println!();

    // 3. Standard-tier reasoning request (8 loops, no attestation)
    let model_id = "openmythos-3b";
    println!("Submitting Standard-tier reasoning request to {model_id}...");
    match cortex
        .reason(model_id, "What is the capital of France?", ReasoningTier::Standard)
        .await
    {
        Ok(resp) => {
            println!("  request_id: {}", resp.request_id);
            println!("  output: {}", String::from_utf8_lossy(&resp.output));
            println!(
                "  tokens_in={} tokens_out={} loops_used={} latency_ms={}",
                resp.metadata.input_tokens,
                resp.metadata.output_tokens,
                resp.metadata.loops_used,
                resp.metadata.latency_ms
            );
            println!("  price_tnzo={} settled={}", resp.price_tnzo, resp.settled);
            println!("  receipt.worker_did={}", resp.receipt.worker_did);
            println!("  receipt.weights_hash={}", resp.receipt.weights_hash);
            println!(
                "  receipt.loops_requested={} loops_used={}",
                resp.receipt.loops_requested, resp.receipt.loops_used
            );
        }
        Err(e) => println!("  (failed: {e})"),
    }
    println!();

    // 4. Institutional-tier reasoning request with TEE attestation + custom budget
    println!("Submitting Institutional-tier reasoning request with TEE attestation...");
    let req = CortexRequest {
        request_id: None,
        model_id: model_id.to_string(),
        input: "Plan a multi-step trade strategy".to_string(),
        tier: Some(ReasoningTier::Institutional),
        min_loops: Some(16),
        max_loops: Some(32),
        max_cost_tnzo: Some(1_000_000_000_000_000_000), // 1 TNZO ceiling
        deadline_ms: Some(30_000),
        attestation: Some(AttestationRequirement::Tee),
        requester: None,
        params: Default::default(),
    };
    match cortex.reason_with_request(&req).await {
        Ok(resp) => {
            println!("  receipt.tee_quote present: {}", resp.receipt.tee_quote.is_some());
            println!("  receipt.zk_proof present: {}", resp.receipt.zk_proof.is_some());
            println!("  price_tnzo={}", resp.price_tnzo);
        }
        Err(e) => println!("  (failed: {e})"),
    }
    println!();

    // 5. Local cost estimation via CortexPricing
    let pricing = CortexPricing::default();
    let estimated = pricing.compute(50, 100, 8, AttestationRequirement::None);
    let estimated_tee = pricing.compute(50, 100, 16, AttestationRequirement::Tee);
    let estimated_full = pricing.compute(50, 100, 32, AttestationRequirement::TeeAndZk);
    println!("Cost estimates (smallest TNZO unit):");
    println!("  Standard (8 loops, no attestation):     {estimated}");
    println!("  Deep     (16 loops, TEE attestation):    {estimated_tee}");
    println!("  Institutional (32 loops, TEE+ZK):        {estimated_full}");

    println!("\n=== Cortex example completed ===");
    Ok(())
}
