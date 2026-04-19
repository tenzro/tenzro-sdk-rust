//! AgentKit: Yield Rebalancer — thin shell example
//!
//! Spawns the `ref-yield-rebalancer-v1` template from the registry, then
//! runs its declarative execution spec in dry-run mode.
//!
//! The template's `ExecutionSpec` handles everything:
//!   1. Auto-discovers yield sources via `required_tool_tags: ["yield-source"]`
//!   2. Evaluates `opportunity.apy > 5.0` with a JMESPath predicate
//!   3. Bridges + deposits into the winning vault
//!
//! All the user does is pick the template, set context vars, and press Run.

use tenzro_sdk::{TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== AgentKit: Yield Rebalancer ===\n");

    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;
    let marketplace = client.marketplace();

    // 1. List executable templates tagged "yield"
    println!("1. Discovering yield templates...");
    let templates = marketplace.list_agent_templates(Some(true), Some(20), Some(0)).await?;
    let yield_templates: Vec<_> = templates.iter()
        .filter(|t| t.tags.iter().any(|tag| tag == "yield"))
        .collect();
    println!("   Found {} yield template(s)\n", yield_templates.len());

    // 2. Spawn the reference yield rebalancer
    println!("2. Spawning ref-yield-rebalancer-v1...");
    let spawn_result = client.agent().spawn_agent_template(
        "ref-yield-rebalancer-v1",
        Some("SDK Example User"),
        None,
    ).await?;

    let agent_id = &spawn_result.agent_id;
    println!("   Agent ID:    {agent_id}");
    println!("   Template:    {}\n", spawn_result.template_id);

    // 3. Dry-run the execution spec
    println!("3. Running execution spec (dry-run)...");
    let run_result = client.agent().run_agent_template(agent_id, Some(1), true).await?;

    println!("   Iterations:  {}", run_result.iterations_completed);
    println!("   Result:      {}", run_result.result);
    println!("   Status:      {}", run_result.status);

    println!("\nDone. Remove dry_run to execute for real.");
    Ok(())
}
