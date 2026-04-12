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

    // Register an agent
    println!("Registering agent...");
    let response = agent
        .register("data-analyzer-001", "Data Analyzer Agent", &["inference", "analysis"])
        .await?;
    println!("Agent registered successfully!");
    println!("  Agent ID: {}", response.agent_id);
    println!("  Status: {}\n", response.status);

    // List all agents
    println!("Listing all registered agents...");
    let agents = agent.list_agents().await?;
    println!("Found {} agents", agents.len());
    for agent_identity in &agents {
        println!("  - {} ({})", agent_identity.name, agent_identity.agent_id);
    }
    println!();

    // Send a message to the agent
    println!("Sending message to agent...");
    let response = agent
        .send_message("data-analyzer-001", "Analyze user metrics for the last 30 days")
        .await?;
    println!("Received response from agent!");
    println!("  Message ID: {}", response.message_id);
    println!("  Payload: {}\n", response.payload);

    // Delegate a task to the agent via A2A
    println!("Delegating task to agent...");
    let task_response = agent
        .delegate_task("data-analyzer-001", "Process and aggregate user interaction data")
        .await?;
    println!("Task delegated successfully!");
    println!("  Task ID: {}", task_response.id);
    println!("  Status: {}", task_response.status);

    println!("\n=== Example completed successfully! ===");

    Ok(())
}
