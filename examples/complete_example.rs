//! Complete example demonstrating all Tenzro SDK features
//!
//! This comprehensive example shows:
//! - Connecting to the network
//! - Wallet operations
//! - AI model inference
//! - Settlement and payments
//! - AI agent interactions
//! - Governance participation

use tenzro_sdk::{Address, TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK Complete Example ===\n");

    // ========================================
    // 1. CONNECT TO NETWORK
    // ========================================
    println!("STEP 1: Connecting to Tenzro Network");
    println!("----------------------------------------");

    let config = SdkConfig::builder()
        .endpoint("https://rpc.tenzro.network")
        .timeout(30000)
        .max_retries(3)
        .build()?;

    let client = TenzroClient::connect(config).await?;
    println!("Connected to Tenzro Network");

    let info = client.node_info().await?;
    println!("  Version: {}", info.version);
    println!("  Chain ID: {}", info.chain_id);
    println!("  Block height: {}", info.block_height);
    println!();

    // ========================================
    // 2. WALLET OPERATIONS
    // ========================================
    println!("STEP 2: Wallet Operations");
    println!("----------------------------------------");

    let wallet = client.wallet();
    let wallet_info = wallet.create_wallet().await?;
    println!("Created wallet: {}", wallet_info.address);

    let address = Address::zero();
    let balances = wallet.get_all_balances(address).await?;
    println!("  Balances:");
    for balance in &balances.balances {
        println!("    {}: {}", balance.symbol, balance.as_decimal());
    }
    println!();

    // ========================================
    // 3. AI MODEL INFERENCE
    // ========================================
    println!("STEP 3: AI Model Inference");
    println!("----------------------------------------");

    let inference = client.inference();

    // Estimate cost
    let cost = inference.estimate_cost("gemma4-9b", 100).await?;
    println!("Estimated cost for inference: {} TNZO", cost);

    // Submit inference
    let response = inference.request(
        "gemma4-9b",
        "Explain Tenzro Network in one sentence.",
        Some(100),
    ).await?;
    println!("Inference completed");
    println!("  Output: {}", response.output);
    println!();

    // ========================================
    // 4. SETTLEMENT & PAYMENTS
    // ========================================
    println!("STEP 4: Settlement & Payments");
    println!("----------------------------------------");

    let settlement = client.settlement();

    // Create on-chain escrow (signed CreateEscrow tx). Authentication is
    // ambient — set TENZRO_BEARER_JWT + TENZRO_DPOP_PROOF after onboarding
    // via `client.auth()`. Signing happens server-side against the holder's
    // MPC wallet, so no raw private key crosses the SDK surface.
    let payer_addr = std::env::var("TENZRO_PAYER")
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000000000000000000000000001".to_string());
    let escrow_tx_hash = settlement.create_escrow(
        &payer_addr,
        "0x0000000000000000000000000000000000000000000000000000000000000002",
        1_000_000_000_000_000_000u128,
        "TNZO",
        u64::MAX,
        "both_signatures",
    ).await?;
    println!("CreateEscrow tx: {}", escrow_tx_hash);

    // Open payment channel
    let channel_id = settlement.open_payment_channel(
        "0x0000000000000000000000000000000000000001",
        10000000,
    ).await?;
    println!("Opened payment channel: {}", channel_id);
    println!();

    // ========================================
    // 5. AI AGENTS
    // ========================================
    println!("STEP 5: AI Agent Operations");
    println!("----------------------------------------");

    let agent = client.agent();

    // Register agent using SDK API
    let reg_response = agent.register(
        "analytics-agent",
        "Analytics Agent",
        &["inference", "analysis"],
    ).await?;
    println!("Registered agent: {}", reg_response.agent_id);

    // Send message to agent
    let msg_response = agent.send_message(&reg_response.agent_id, "Analyze latest transactions").await?;
    println!("Agent responded: {}", msg_response.payload);
    println!();

    // ========================================
    // 6. GOVERNANCE
    // ========================================
    println!("STEP 6: Governance Participation");
    println!("----------------------------------------");

    let governance = client.governance();

    // Check voting power
    let voting_power = governance.get_voting_power("0x0000000000000000000000000000000000000000").await?;
    println!("  Voting power: {} TNZO", voting_power.total_power);

    // Create proposal
    let proposal = governance.create_proposal(
        "Network Upgrade Proposal",
        "Upgrade network to support advanced AI features",
        "parameter_change",
    ).await?;
    println!("Created proposal: {}", proposal.proposal_id);

    // Vote on proposal
    let vote = governance.vote(&proposal.proposal_id, "for").await?;
    println!("Voted on proposal: {}", vote.vote_id);
    println!();

    // ========================================
    // SUMMARY
    // ========================================
    println!("========================================");
    println!("All operations completed successfully!");
    println!("========================================");
    println!("\nSummary:");
    println!("  - Connected to Tenzro Network");
    println!("  - Created and managed wallet");
    println!("  - Executed AI model inference");
    println!("  - Performed settlement operations");
    println!("  - Registered and interacted with AI agent");
    println!("  - Participated in governance");

    Ok(())
}
