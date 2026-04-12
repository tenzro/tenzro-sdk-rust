//! App Developer example for Tenzro SDK
//!
//! This example demonstrates the developer-funded app pattern:
//! - Creating an AppClient with a master wallet
//! - Spawning and funding user sub-wallets
//! - Sponsoring transactions, inference, agents, bridges, and tasks
//! - Setting spending policies and session keys
//! - Tracking usage statistics

use tenzro_sdk::AppClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK App Developer Example ===\n");

    // ========================================================================
    // 1. Initialize the AppClient with a master wallet
    // ========================================================================
    println!("1. Initializing AppClient with master wallet...");

    // In production, load the private key from a secure vault / env var.
    let master_key = std::env::var("TENZRO_MASTER_KEY")
        .unwrap_or_else(|_| "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef".into());

    let app = AppClient::new("https://rpc.tenzro.network", &master_key).await?;
    println!("   Master wallet: {}", app.master_wallet().address);

    let balance = app.get_master_balance().await?;
    println!("   Master balance: {} wei\n", balance);

    // ========================================================================
    // 2. Create user wallets
    // ========================================================================
    println!("2. Creating user wallets...");

    // Fund Alice with 0.5 TNZO
    let alice = app
        .create_user_wallet("alice", 500_000_000_000_000_000)
        .await?;
    println!("   Alice: {} (label: {})", alice.address, alice.label);

    // Fund Bob with 0.1 TNZO
    let bob = app
        .create_user_wallet("bob", 100_000_000_000_000_000)
        .await?;
    println!("   Bob:   {} (label: {})\n", bob.address, bob.label);

    // ========================================================================
    // 3. Set spending policies
    // ========================================================================
    println!("3. Setting spending policies...");

    // Alice: 1 TNZO daily, 0.2 TNZO per tx
    let alice_policy = app
        .set_user_limits(
            &alice.address,
            1_000_000_000_000_000_000,  // 1 TNZO daily
            200_000_000_000_000_000,    // 0.2 TNZO per tx
        )
        .await?;
    println!(
        "   Alice policy: daily={} per_tx={}",
        alice_policy.daily_limit, alice_policy.per_tx_limit
    );

    // Bob: 0.5 TNZO daily, 0.1 TNZO per tx
    let bob_policy = app
        .set_user_limits(
            &bob.address,
            500_000_000_000_000_000,    // 0.5 TNZO daily
            100_000_000_000_000_000,    // 0.1 TNZO per tx
        )
        .await?;
    println!(
        "   Bob policy:   daily={} per_tx={}\n",
        bob_policy.daily_limit, bob_policy.per_tx_limit
    );

    // ========================================================================
    // 4. Create session keys
    // ========================================================================
    println!("4. Creating session keys...");

    let alice_session = app
        .create_session_key(
            &alice.address,
            3600, // 1 hour
            vec!["transfer".into(), "inference".into()],
        )
        .await?;
    println!(
        "   Alice session: {} (expires: {}, ops: {:?})\n",
        alice_session.session_id, alice_session.expires_at, alice_session.operations
    );

    // ========================================================================
    // 5. Sponsor transactions (master pays gas)
    // ========================================================================
    println!("5. Sponsoring a transfer for Alice...");

    let tx = app
        .sponsor_transaction(
            &alice.address,
            &bob.address,
            50_000_000_000_000_000, // 0.05 TNZO
        )
        .await?;
    println!("   Tx hash: {}\n", tx.tx_hash);

    // ========================================================================
    // 6. Sponsor inference (master pays)
    // ========================================================================
    println!("6. Sponsoring inference for Bob...");

    let inference = app
        .sponsor_inference(&bob.address, "gemma3-270m", "Explain Tenzro Network in one sentence.")
        .await?;
    println!("   Model: {}", inference.model_id);
    println!("   Output: {}", inference.output);
    println!("   Tokens: {}, Cost: {} TNZO\n", inference.tokens, inference.cost);

    // ========================================================================
    // 7. Sponsor agent registration (master pays)
    // ========================================================================
    println!("7. Sponsoring agent registration for Alice...");

    let agent = app
        .sponsor_agent(
            &alice.address,
            "alice-trading-bot",
            vec!["inference".into(), "settlement".into()],
        )
        .await?;
    println!("   Agent ID: {}", agent.agent_id);
    println!("   Agent wallet: {}\n", agent.wallet_address);

    // ========================================================================
    // 8. Sponsor bridge (master pays fees)
    // ========================================================================
    println!("8. Sponsoring bridge transfer for Bob...");

    let bridge = app
        .sponsor_bridge(
            &bob.address,
            "TNZO",
            "tenzro",
            "ethereum",
            "100000000000000000", // 0.1 TNZO
            "0xRecipientOnEthereum",
        )
        .await?;
    println!("   Bridge tx: {}", bridge.tx_hash);
    println!("   Status: {}\n", bridge.status);

    // ========================================================================
    // 9. Sponsor task posting (master pays budget)
    // ========================================================================
    println!("9. Sponsoring task posting for Alice...");

    let task = app
        .sponsor_task(
            &alice.address,
            "Summarize whitepaper",
            "Read and summarize the Tenzro Network whitepaper in 500 words.",
            "text_generation",
            100_000_000_000_000_000, // 0.1 TNZO budget
        )
        .await?;
    println!("   Task ID: {}\n", task.task_id);

    // ========================================================================
    // 10. Fund an existing wallet
    // ========================================================================
    println!("10. Topping up Bob's wallet...");

    let fund = app
        .fund_user_wallet(&bob.address, 200_000_000_000_000_000) // 0.2 TNZO
        .await?;
    println!("   Fund tx: {} (amount: {} wei)\n", fund.tx_hash, fund.amount);

    // ========================================================================
    // 11. List user wallets
    // ========================================================================
    println!("11. Listing all user wallets...");

    let users = app.list_user_wallets().await?;
    for u in &users {
        println!("   - {} (label: {}, created: {})", u.address, u.label, u.created_at);
    }
    println!();

    // ========================================================================
    // 12. Usage statistics
    // ========================================================================
    println!("12. Usage statistics...");

    let stats = app.get_usage_stats().await?;
    println!("   Users created:       {}", stats.user_count);
    println!("   Transactions:        {}", stats.transaction_count);
    println!("   Total gas spent:     {} wei", stats.total_gas_spent);
    println!("   Total inference cost: {} TNZO", stats.total_inference_cost);
    println!("   Total bridge fees:   {} wei", stats.total_bridge_fees);
    println!();

    // ========================================================================
    // 13. Access the full TenzroClient for advanced ops
    // ========================================================================
    println!("13. Using the underlying TenzroClient...");

    let block = app.client().block_number().await?;
    println!("   Current block: {}", block);

    println!("\n=== Done ===");
    Ok(())
}
