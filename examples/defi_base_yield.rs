//! DeFi Example: Base L2 Yield Strategy
//!
//! Demonstrates an autonomous yield-farming agent on Base (Ethereum L2) that:
//!   1. Registers an identity and provisions a wallet
//!   2. Checks ETH/USDC prices via Chainlink price feeds
//!   3. Monitors gas prices for optimal entry timing
//!   4. Looks up ERC-20 token balances
//!   5. Encodes and executes a smart contract deposit
//!   6. Registers the agent on-chain via ERC-8004
//!   7. Settles the yield profit back to Tenzro Ledger
//!
//! Uses the Ethereum MCP tools (port 3004) which support Base as an L2.

use tenzro_sdk::{TenzroClient, SettlementRequest, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== DeFi: Base L2 Yield Strategy ===\n");

    // ─── Connect ────────────────────────────────────────────────────────────
    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;

    // ─── 1. Identity & Wallet ───────────────────────────────────────────────
    println!("1. Provisioning identity and wallet...");
    let identity = client.identity().register_human("Base DeFi Strategist").await?;
    println!("   DID:    {}", identity.did);

    let wallet = client.wallet().create_wallet().await?;
    println!("   Wallet: {}\n", wallet.address);

    // ─── 2. Register agent with Base DeFi skill ─────────────────────────────
    println!("2. Spawning yield strategy agent...");
    let agent = client.agent().register(
        "base-yield-agent",
        "Base Yield Strategist",
        &["defi", "yield", "base"],
    ).await?;
    println!("   Agent ID: {}\n", agent.agent_id);

    // ─── 3. Check Chainlink price feeds ─────────────────────────────────────
    println!("3. Querying Chainlink price feeds on Base...");

    // ETH/USD price via Chainlink AggregatorV3
    let eth_price = client.agent().send_message(
        &agent.agent_id,
        "Use eth_get_price tool: pair=ETH/USD, chain_id=8453",
    ).await?;
    println!("   ETH/USD: {}", eth_price.payload);

    // USDC/USD to verify peg
    let usdc_price = client.agent().send_message(
        &agent.agent_id,
        "Use eth_get_price tool: pair=USDC/USD, chain_id=8453",
    ).await?;
    println!("   USDC/USD: {}\n", usdc_price.payload);

    // ─── 4. Monitor gas for optimal entry ───────────────────────────────────
    println!("4. Checking Base gas prices...");
    let gas = client.agent().send_message(
        &agent.agent_id,
        "Use eth_get_gas_price tool: chain_id=8453",
    ).await?;
    println!("   Gas price: {}", gas.payload);

    let fee_history = client.agent().send_message(
        &agent.agent_id,
        "Use eth_get_fee_history tool: block_count=5, chain_id=8453",
    ).await?;
    println!("   Fee trend: {}\n", fee_history.payload);

    // ─── 5. Check token balances ────────────────────────────────────────────
    println!("5. Checking ERC-20 balances...");

    // USDC on Base (well-known address)
    let usdc_balance = client.agent().send_message(
        &agent.agent_id,
        &format!(
            "Use eth_get_token_balance tool: address={}, \
             token_address=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913, \
             chain_id=8453",
            wallet.address,
        ),
    ).await?;
    println!("   USDC balance: {}", usdc_balance.payload);

    // ETH balance
    let eth_bal = client.agent().send_message(
        &agent.agent_id,
        &format!("Use eth_get_balance tool: address={}, chain_id=8453", wallet.address),
    ).await?;
    println!("   ETH balance:  {}\n", eth_bal.payload);

    // ─── 6. Encode vault deposit call ───────────────────────────────────────
    println!("6. Encoding vault deposit transaction...");

    // Encode an ERC-4626 vault deposit(uint256 assets, address receiver)
    let deposit_calldata = client.agent().send_message(
        &agent.agent_id,
        &format!(
            "Use eth_encode_function tool: \
             function_signature=deposit(uint256,address), \
             args=[\"1000000\", \"{}\"]",
            wallet.address,
        ),
    ).await?;
    println!("   Calldata: {}\n", deposit_calldata.payload);

    // ─── 7. Simulate the deposit via contract call ──────────────────────────
    println!("7. Simulating vault deposit (eth_call)...");
    let sim_result = client.agent().send_message(
        &agent.agent_id,
        &format!(
            "Use eth_call_contract tool: \
             to=0x0000000000000000000000000000000000000001, \
             data={}, from={}, chain_id=8453",
            deposit_calldata.payload, wallet.address,
        ),
    ).await?;
    println!("   Simulation result: {}\n", sim_result.payload);

    // ─── 8. Register agent on-chain via ERC-8004 ────────────────────────────
    println!("8. Registering agent on-chain (ERC-8004)...");
    let erc8004 = client.agent().send_message(
        &agent.agent_id,
        &format!(
            "Use eth_register_agent_8004 tool: \
             agent_address={}, metadata_uri=ipfs://QmYieldStrategyBase",
            wallet.address,
        ),
    ).await?;
    println!("   ERC-8004 registration: {}\n", erc8004.payload);

    // ─── 9. Settle yield profits on Tenzro Ledger ───────────────────────────
    println!("9. Settling yield profits...");
    let settlement = client.settlement().settle(SettlementRequest {
        request_id: format!("base-yield-{}", uuid::Uuid::new_v4()),
        provider: wallet.address.clone(),
        customer: "0x0000000000000000000000000000000000000000".to_string(),
        amount: 1000,
        asset: "TNZO".to_string(),
    }).await?;
    println!("   Settlement: {}", settlement.receipt_id);
    println!("   Status:     {}\n", settlement.status);

    // ─── 10. Create on-chain attestation ────────────────────────────────────
    println!("10. Querying EAS attestation...");
    let attestation = client.agent().send_message(
        &agent.agent_id,
        "Use eth_get_attestation tool: \
         uid=0x0000000000000000000000000000000000000000000000000000000000000001, \
         chain_id=8453",
    ).await?;
    println!("   Attestation: {}\n", attestation.payload);

    println!("=== Base Yield Strategy Complete ===");
    println!("Chain: Base (chain_id 8453)");
    println!("Tools used: eth_get_price, eth_get_gas_price, eth_get_fee_history,");
    println!("  eth_get_token_balance, eth_get_balance, eth_encode_function,");
    println!("  eth_call_contract, eth_register_agent_8004, eth_get_attestation");

    Ok(())
}
