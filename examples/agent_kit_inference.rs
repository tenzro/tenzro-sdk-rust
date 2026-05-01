//! AgentKit: Model Inference Proxy — thin shell example
//!
//! Spawns the `ref-model-inference-proxy-v1` template and routes an
//! inference request through the cheapest available provider.
//!
//! The template discovers models via the `model-discovery` tool tag,
//! pays the provider via x402, and invokes the `chat-completion` skill.

use tenzro_sdk::{TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== AgentKit: Model Inference Proxy ===\n");

    let client = TenzroClient::connect(SdkConfig::testnet()).await?;

    println!("1. Spawning inference proxy agent...");
    let spawn = client.agent().spawn_agent_template(
        "ref-model-inference-proxy-v1",
        Some("Inference Example"),
        None,
        None,
    ).await?;

    let agent_id = &spawn.agent_id;
    println!("   Agent: {agent_id}\n");

    println!("2. Routing inference request (dry-run)...");
    let result = client.agent().run_agent_template(agent_id, Some(1), true).await?;

    println!("   Iterations: {}", result.iterations_completed);
    println!("   Result:     {}\n", result.result);
    println!("Done.");
    Ok(())
}
