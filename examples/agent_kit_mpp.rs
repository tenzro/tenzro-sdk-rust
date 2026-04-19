//! AgentKit: MPP Payment Agent — thin shell example
//!
//! Spawns the `ref-mpp-payment-agent-v1` template and runs a single
//! MPP (Machine Payments Protocol) payment in dry-run mode.
//!
//! The template handles the full HTTP 402 challenge / credential / receipt
//! flow autonomously. The user only provides amount + recipient context.

use tenzro_sdk::{TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== AgentKit: MPP Payment Agent ===\n");

    let client = TenzroClient::connect(SdkConfig::testnet()).await?;

    // Spawn the MPP payment template
    println!("1. Spawning MPP payment agent...");
    let spawn = client.agent().spawn_agent_template(
        "ref-mpp-payment-agent-v1",
        Some("MPP Example"),
        None,
    ).await?;

    let agent_id = &spawn.agent_id;
    println!("   Agent: {agent_id}\n");

    // Dry-run the payment step
    println!("2. Executing MPP pay step (dry-run)...");
    let result = client.agent().run_agent_template(agent_id, Some(1), true).await?;

    println!("   Iterations: {}", result.iterations_completed);
    println!("   Result:     {}\n", result.result);
    println!("Done.");
    Ok(())
}
