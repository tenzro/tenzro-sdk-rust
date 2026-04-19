//! DeFi Example: Solana DEX Aggregation & Yield Discovery
//!
//! Demonstrates an autonomous agent on Solana that:
//!   1. Discovers SOL and SPL token balances
//!   2. Fetches real-time prices from Jupiter
//!   3. Finds best swap routes (SOL → USDC) via Jupiter aggregator
//!   4. Discovers yield opportunities across Solana DeFi protocols
//!   5. Stakes SOL with a validator for native yield
//!   6. Resolves .sol domains via Bonfida SNS
//!   7. Inspects NFT metadata via Metaplex DAS
//!   8. Settles profits back on Tenzro Ledger
//!
//! Uses the Solana MCP tools (port 3003).

use tenzro_sdk::{TenzroClient, SettlementRequest, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== DeFi: Solana DEX Aggregation & Yield ===\n");

    // ─── Connect ────────────────────────────────────────────────────────────
    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;

    // ─── 1. Identity & Wallet ───────────────────────────────────────────────
    println!("1. Provisioning identity and wallet...");
    let identity = client.identity().register_human("Solana DeFi Trader").await?;
    println!("   DID: {}", identity.did);

    let wallet = client.wallet().create_wallet().await?;
    println!("   Wallet: {}\n", wallet.address);

    // ─── 2. Register Solana DeFi agent ──────────────────────────────────────
    println!("2. Registering Solana DeFi agent...");
    let agent = client.agent().register(
        "solana-defi-agent",
        "Solana DEX Aggregator",
        &["defi", "swap", "solana", "jupiter"],
    ).await?;
    println!("   Agent ID: {}\n", agent.agent_id);

    // ─── 3. Check network health ────────────────────────────────────────────
    println!("3. Checking Solana network status...");

    let slot = client.agent().send_message(
        &agent.agent_id,
        "Use solana_get_slot tool",
    ).await?;
    println!("   Current slot: {}", slot.payload);

    let tps = client.agent().send_message(
        &agent.agent_id,
        "Use solana_get_tps tool",
    ).await?;
    println!("   Current TPS:  {}\n", tps.payload);

    // ─── 4. Query token balances ────────────────────────────────────────────
    println!("4. Querying token balances...");

    let sol_balance = client.agent().send_message(
        &agent.agent_id,
        &format!("Use solana_get_balance tool: address={}", wallet.address),
    ).await?;
    println!("   SOL balance: {}", sol_balance.payload);

    let token_accounts = client.agent().send_message(
        &agent.agent_id,
        &format!("Use solana_get_token_accounts tool: owner={}", wallet.address),
    ).await?;
    println!("   SPL tokens:  {}\n", token_accounts.payload);

    // ─── 5. Fetch prices via Jupiter ────────────────────────────────────────
    println!("5. Fetching real-time prices...");

    let sol_price = client.agent().send_message(
        &agent.agent_id,
        "Use solana_get_price tool: token_mint=So11111111111111111111111111111111111111112",
    ).await?;
    println!("   SOL/USD:  {}", sol_price.payload);

    // BONK price
    let bonk_price = client.agent().send_message(
        &agent.agent_id,
        "Use solana_get_price tool: token_mint=DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
    ).await?;
    println!("   BONK/USD: {}\n", bonk_price.payload);

    // ─── 6. Get swap quote (SOL → USDC via Jupiter) ─────────────────────────
    println!("6. Getting Jupiter swap quote (1 SOL → USDC)...");

    let swap_quote = client.agent().send_message(
        &agent.agent_id,
        "Use solana_swap tool: \
         input_mint=So11111111111111111111111111111111111111112, \
         output_mint=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v, \
         amount=1000000000, \
         slippage_bps=50",
    ).await?;
    println!("   Swap quote: {}\n", swap_quote.payload);

    // ─── 7. Discover yield opportunities ────────────────────────────────────
    println!("7. Discovering yield opportunities...");

    let yields = client.agent().send_message(
        &agent.agent_id,
        "Use solana_get_yield tool",
    ).await?;
    println!("   Available yields: {}\n", yields.payload);

    // ─── 8. Stake SOL for native yield ──────────────────────────────────────
    println!("8. Staking SOL with validator...");

    let stake_result = client.agent().send_message(
        &agent.agent_id,
        "Use solana_stake tool: \
         amount=500000000, \
         validator=Vote111111111111111111111111111111111111111",
    ).await?;
    println!("   Stake result: {}\n", stake_result.payload);

    // ─── 9. Resolve .sol domain ─────────────────────────────────────────────
    println!("9. Resolving SNS domain...");

    let domain = client.agent().send_message(
        &agent.agent_id,
        "Use solana_resolve_domain tool: domain=tenzro.sol",
    ).await?;
    println!("   tenzro.sol → {}\n", domain.payload);

    // ─── 10. Inspect NFT via Metaplex DAS ───────────────────────────────────
    println!("10. Looking up NFT metadata...");

    let nft = client.agent().send_message(
        &agent.agent_id,
        "Use solana_get_nft tool: \
         mint=11111111111111111111111111111111",
    ).await?;
    println!("   NFT metadata: {}\n", nft.payload);

    // ─── 11. Check a Solana transaction ─────────────────────────────────────
    println!("11. Inspecting recent transaction...");

    let tx = client.agent().send_message(
        &agent.agent_id,
        "Use solana_get_transaction tool: \
         signature=5VERv8NMhN1VfGkfF2WBkp2nFPYQochCUqvEhKZz8ygkqsM3cPrHabpKjbrDR4bVzJ",
    ).await?;
    println!("   Transaction: {}\n", tx.payload);

    // ─── 12. Get SPL token info ─────────────────────────────────────────────
    println!("12. Getting SPL token info...");
    let token_info = client.agent().send_message(
        &agent.agent_id,
        "Use solana_get_token_info tool: \
         mint=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    ).await?;
    println!("   USDC info: {}\n", token_info.payload);

    // ─── 13. Settle profits on Tenzro Ledger ────────────────────────────────
    println!("13. Settling swap profits on Tenzro Ledger...");

    let settlement = client.settlement().settle(SettlementRequest {
        request_id: format!("sol-swap-{}", uuid::Uuid::new_v4()),
        provider: wallet.address.clone(),
        customer: "0x0000000000000000000000000000000000000000".to_string(),
        amount: 500,
        asset: "TNZO".to_string(),
    }).await?;
    println!("   Settlement: {}", settlement.receipt_id);
    println!("   Status:     {}\n", settlement.status);

    println!("=== Solana DeFi Strategy Complete ===");
    println!("Chain: Solana");
    println!("Tools used: solana_get_slot, solana_get_tps, solana_get_balance,");
    println!("  solana_get_token_accounts, solana_get_price, solana_swap,");
    println!("  solana_get_yield, solana_stake, solana_resolve_domain,");
    println!("  solana_get_nft, solana_get_transaction, solana_get_token_info");

    Ok(())
}
