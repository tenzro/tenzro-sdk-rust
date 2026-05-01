//! AgentKit: Bridge Arbitrage Scanner — thin shell example
//!
//! Spawns the `ref-bridge-arbitrage-scanner-v1` template and runs a scan
//! for cross-chain arbitrage in dry-run mode.
//!
//! The template discovers bridge fee differentials via the `bridge-fee-oracle`
//! tool tag and executes transfers when the spread exceeds 50bps.

use tenzro_sdk::{TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== AgentKit: Bridge Arbitrage Scanner ===\n");

    let client = TenzroClient::connect(SdkConfig::testnet()).await?;

    println!("1. Spawning bridge arbitrage scanner...");
    let spawn = client.agent().spawn_agent_template(
        "ref-bridge-arbitrage-scanner-v1",
        Some("Bridge Arb Example"),
        None,
        None,
    ).await?;

    let agent_id = &spawn.agent_id;
    println!("   Agent: {agent_id}\n");

    println!("2. Scanning for arbitrage (dry-run)...");
    let result = client.agent().run_agent_template(agent_id, Some(1), true).await?;

    println!("   Iterations: {}", result.iterations_completed);
    println!("   Result:     {}", result.result);
    println!("   Status:     {}\n", result.status);
    println!("Done. Set dry_run=false to execute real bridge transfers.");
    Ok(())
}
