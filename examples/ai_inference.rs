//! AI Inference example for Tenzro SDK
//!
//! This example demonstrates:
//! - Listing available AI models
//! - Submitting inference requests
//! - Estimating inference costs

use tenzro_sdk::{TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK AI Inference Example ===\n");

    // Connect to testnet
    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;
    let inference = client.inference();

    // List available models
    println!("Listing available AI models...");
    let models = inference.list_models().await?;
    println!("Found {} models\n", models.len());

    for model in &models {
        println!("Model: {}", model.model_id);
        println!("  Name: {}", model.name);
        println!("  Version: {}", model.version);
        println!("  Description: {}", model.description);
        println!();
    }

    // Estimate cost for inference
    let model_id = "gemma4-9b";
    let input_tokens = 100u32;
    println!("Estimating cost for {} with {} input tokens...", model_id, input_tokens);
    let estimated_cost = inference.estimate_cost(model_id, input_tokens).await?;
    println!("Estimated cost: {} TNZO (smallest unit)\n", estimated_cost);

    // Submit an inference request
    println!("Submitting inference request...");
    let response = inference.request(
        model_id,
        "What is the capital of France?",
        Some(100),
    ).await?;

    println!("Inference completed!");
    println!("Request ID: {}", response.request_id);
    println!("Model: {}", response.model_id);
    println!("Output: {}", response.output);
    println!("Tokens used: {}", response.tokens_used);

    println!("\n=== Example completed successfully! ===");

    Ok(())
}
