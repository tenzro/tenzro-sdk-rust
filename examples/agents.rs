//! AI Agent example for Tenzro SDK
//!
//! This example demonstrates:
//! - Registering AI agents
//! - Sending messages to agents
//! - Delegating tasks to agents
//! - Listing agents

use tenzro_sdk::{TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK AI Agent Example ===\n");

    // Connect to testnet
    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;
    let agent = client.agent();

    // Register an agent. The node provisions a server-side hybrid wallet
    // (FROST Ed25519 + ML-DSA-65) and binds it to a fresh did:tenzro:machine:
    // identity. The `creator` is the human/machine address that will own the
    // agent — replace with your wallet address.
    let creator = "0x0000000000000000000000000000000000000000000000000000000000000001";
    println!("Registering agent...");
    let response = agent
        .register("Data Analyzer Agent", creator, &["nlp", "data"])
        .await?;
    println!("Agent registered successfully!");
    println!("  Agent ID: {}", response.agent_id);
    println!("  DID: {}", response.tenzro_did);
    println!("  Wallet: {}", response.wallet_address);
    println!("  Classical pubkey: {}\n", response.classical_public_key);
    let agent_id = response.agent_id.clone();

    // List all agents
    println!("Listing all registered agents...");
    let agents = agent.list_agents().await?;
    println!("Found {} agents", agents.len());
    for agent_identity in &agents {
        println!("  - {} ({})", agent_identity.name, agent_identity.agent_id);
    }
    println!();

    // Send an unsigned message — only succeeds against a router with
    // enable_signing == false. For the production router, see
    // send_message_signed below.
    println!("Sending message to agent (unsigned, dev mode)...");
    let response = agent
        .send_message(&agent_id, &agent_id, "Analyze user metrics for the last 30 days")
        .await?;
    println!("Message accepted!");
    println!("  Message ID: {}", response.message_id);
    println!("  Signed: {}\n", response.signed);

    // Delegate a task to the agent via A2A
    println!("Delegating task to agent...");
    let task_response = agent
        .delegate_task(&agent_id, "Process and aggregate user interaction data")
        .await?;
    println!("Task delegated successfully!");
    println!("  Task ID: {}", task_response.id);
    println!("  Status: {}", task_response.status);

    println!("\n=== Example completed successfully! ===");

    Ok(())
}
