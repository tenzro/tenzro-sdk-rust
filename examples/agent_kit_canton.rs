//! AgentKit: Canton Trade Settler — thin shell example
//!
//! Spawns the `ref-canton-trade-settler-v1` template and runs the DAML
//! trade settlement workflow in dry-run mode.
//!
//! The template auto-discovers trade opportunities via the `trade-opportunity`
//! tool tag and submits DAML Create commands for each match.

use tenzro_sdk::{TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== AgentKit: Canton Trade Settler ===\n");

    let client = TenzroClient::connect(SdkConfig::testnet()).await?;

    println!("1. Spawning Canton trade settler...");
    let spawn = client.agent().spawn_agent_template(
        "ref-canton-trade-settler-v1",
        Some("Canton Example"),
        None,
    ).await?;

    let agent_id = &spawn.agent_id;
    println!("   Agent: {agent_id}");
    println!("   Template: {}\n", spawn.template_id);

    println!("2. Executing DAML settlement (dry-run)...");
    let result = client.agent().run_agent_template(agent_id, Some(1), true).await?;

    println!("   Iterations: {}", result.iterations_completed);
    println!("   Status:     {}", result.status);
    println!("\nDone.");
    Ok(())
}
