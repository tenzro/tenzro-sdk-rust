//! DeFi Example: Canton Enterprise DvP Settlement
//!
//! Demonstrates institutional DeFi on Canton (DAML) including:
//!   1. Party allocation and domain discovery
//!   2. Real-world asset (RWA) tokenization via CIP-56
//!   3. Delivery-vs-Payment (DvP) atomic settlement
//!   4. Canton Coin transfers between participants
//!   5. DAML contract lifecycle (create, exercise, query)
//!   6. DAR package upload for custom workflows
//!   7. Fee schedule queries
//!   8. Settlement finality on Tenzro Ledger
//!
//! Uses the Canton MCP tools (port 3005).

use tenzro_sdk::{TenzroClient, SettlementRequest, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== DeFi: Canton Enterprise DvP Settlement ===\n");

    // ─── Connect ────────────────────────────────────────────────────────────
    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;

    // ─── 1. Identity & Wallet ───────────────────────────────────────────────
    println!("1. Provisioning identity and wallet...");
    let identity = client.identity().register_human("Canton Fund Manager").await?;
    println!("   DID: {}", identity.did);

    let wallet = client.wallet().create_wallet().await?;
    println!("   Wallet: {}\n", wallet.address);

    // ─── 2. Register Canton agent ───────────────────────────────────────────
    println!("2. Registering Canton enterprise agent...");
    let agent = client.agent().register(
        "canton-dvp-agent",
        "Canton DvP Settler",
        &["enterprise", "canton", "daml", "dvp"],
    ).await?;
    println!("   Agent ID: {}\n", agent.agent_id);

    // ─── 3. Check Canton health & domains ───────────────────────────────────
    println!("3. Checking Canton network health...");

    let health = client.agent().send_message(
        &agent.agent_id,
        "Use canton_get_health tool",
    ).await?;
    println!("   Canton health: {}", health.payload);

    let domains = client.agent().send_message(
        &agent.agent_id,
        "Use canton_list_domains tool",
    ).await?;
    println!("   Domains: {}\n", domains.payload);

    // ─── 4. Allocate parties ────────────────────────────────────────────────
    println!("4. Allocating DAML parties...");

    let buyer = client.agent().send_message(
        &agent.agent_id,
        "Use canton_allocate_party tool: \
         party_id_hint=buyer-fund-a, \
         display_name=Fund A (Buyer)",
    ).await?;
    println!("   Buyer party:  {}", buyer.payload);

    let seller = client.agent().send_message(
        &agent.agent_id,
        "Use canton_allocate_party tool: \
         party_id_hint=seller-bank-b, \
         display_name=Bank B (Seller)",
    ).await?;
    println!("   Seller party: {}\n", seller.payload);

    // List all parties
    let parties = client.agent().send_message(
        &agent.agent_id,
        "Use canton_list_parties tool",
    ).await?;
    println!("   All parties: {}\n", parties.payload);

    // ─── 5. Tokenize a real-world asset (CIP-56) ────────────────────────────
    println!("5. Tokenizing bond asset (CIP-56)...");

    let asset = client.agent().send_message(
        &agent.agent_id,
        "Use canton_create_asset tool: \
         owner=seller-bank-b, \
         asset_type=bond, \
         quantity=1000000, \
         metadata={\"isin\": \"US912828ZT58\", \"coupon\": \"2.5%\", \"maturity\": \"2030-11-15\"}",
    ).await?;
    println!("   Asset created: {}\n", asset.payload);

    // ─── 6. Check Canton Coin balances ──────────────────────────────────────
    println!("6. Checking Canton Coin balances...");

    let buyer_balance = client.agent().send_message(
        &agent.agent_id,
        "Use canton_get_balance tool: party=buyer-fund-a",
    ).await?;
    println!("   Buyer balance:  {}", buyer_balance.payload);

    let seller_balance = client.agent().send_message(
        &agent.agent_id,
        "Use canton_get_balance tool: party=seller-bank-b",
    ).await?;
    println!("   Seller balance: {}\n", seller_balance.payload);

    // ─── 7. Fund the buyer (Canton Coin transfer) ───────────────────────────
    println!("7. Funding buyer with Canton Coin...");

    let transfer = client.agent().send_message(
        &agent.agent_id,
        "Use canton_transfer tool: \
         sender=seller-bank-b, \
         receiver=buyer-fund-a, \
         amount=5000000",
    ).await?;
    println!("   Transfer: {}\n", transfer.payload);

    // ─── 8. Execute DvP settlement ──────────────────────────────────────────
    println!("8. Executing atomic Delivery-vs-Payment...");

    let dvp = client.agent().send_message(
        &agent.agent_id,
        "Use canton_dvp_settle tool: \
         buyer=buyer-fund-a, \
         seller=seller-bank-b, \
         asset_id=bond-us912828zt58, \
         quantity=100, \
         price=5000000",
    ).await?;
    println!("   DvP settlement: {}\n", dvp.payload);

    // ─── 9. Submit a DAML command ───────────────────────────────────────────
    println!("9. Creating a DAML escrow contract...");

    let escrow = client.agent().send_message(
        &agent.agent_id,
        "Use canton_submit_command tool: \
         command_type=create, \
         template_id=Tenzro.Escrow:EscrowContract, \
         payload={\"buyer\": \"buyer-fund-a\", \"seller\": \"seller-bank-b\", \
         \"amount\": 1000000, \"asset_ref\": \"bond-us912828zt58\"}, \
         act_as=buyer-fund-a",
    ).await?;
    println!("   Escrow contract: {}\n", escrow.payload);

    // ─── 10. Query active contracts ─────────────────────────────────────────
    println!("10. Querying active DAML contracts...");

    let contracts = client.agent().send_message(
        &agent.agent_id,
        "Use canton_list_contracts tool: \
         template_id=Tenzro.Escrow:EscrowContract, \
         party=buyer-fund-a",
    ).await?;
    println!("   Active contracts: {}\n", contracts.payload);

    // ─── 11. Get contract events ────────────────────────────────────────────
    println!("11. Fetching contract events...");

    let events = client.agent().send_message(
        &agent.agent_id,
        "Use canton_get_events tool: \
         contract_id=escrow-001, \
         party=buyer-fund-a",
    ).await?;
    println!("   Events: {}\n", events.payload);

    // ─── 12. Query fee schedule ─────────────────────────────────────────────
    println!("12. Checking synchronizer fee schedule...");

    let fees = client.agent().send_message(
        &agent.agent_id,
        "Use canton_get_fee_schedule tool",
    ).await?;
    println!("   Fee schedule: {}\n", fees.payload);

    // ─── 13. Upload DAR package ─────────────────────────────────────────────
    println!("13. Uploading custom DAR package...");

    let dar = client.agent().send_message(
        &agent.agent_id,
        "Use canton_upload_dar tool: \
         dar_path=/packages/tenzro-settlement-1.0.dar",
    ).await?;
    println!("   DAR upload: {}\n", dar.payload);

    // ─── 14. Get a Canton transaction ───────────────────────────────────────
    println!("14. Looking up settlement transaction...");

    let tx = client.agent().send_message(
        &agent.agent_id,
        "Use canton_get_transaction tool: \
         transaction_id=dvp-settle-001, \
         party=buyer-fund-a",
    ).await?;
    println!("   Transaction: {}\n", tx.payload);

    // ─── 15. Settle on Tenzro Ledger ────────────────────────────────────────
    println!("15. Settling DvP on Tenzro Ledger...");

    let settlement = client.settlement().settle(SettlementRequest {
        request_id: format!("canton-dvp-{}", uuid::Uuid::new_v4()),
        provider: wallet.address.clone(),
        customer: "0x0000000000000000000000000000000000000000".to_string(),
        amount: 5000000,
        asset: "TNZO".to_string(),
    }).await?;
    println!("   Settlement: {}", settlement.receipt_id);
    println!("   Status:     {}\n", settlement.status);

    // ─── 16. Add settlement credential to identity ──────────────────────────
    println!("16. Creating settlement attestation credential...");

    let cred = client.identity().add_credential(
        &identity.did,
        "DvPSettlementAttestation",
        None,
        Some(serde_json::json!({
            "settlement_id": settlement.receipt_id,
            "asset": "US912828ZT58",
            "quantity": 100,
            "counterparty": "seller-bank-b",
            "chain": "canton",
        })),
    ).await?;
    println!("   Credential: {}\n", cred);

    println!("=== Canton DvP Settlement Complete ===");
    println!("Chain: Canton (DAML 3.x)");
    println!("Tools used: canton_get_health, canton_list_domains, canton_allocate_party,");
    println!("  canton_list_parties, canton_create_asset, canton_get_balance,");
    println!("  canton_transfer, canton_dvp_settle, canton_submit_command,");
    println!("  canton_list_contracts, canton_get_events, canton_get_fee_schedule,");
    println!("  canton_upload_dar, canton_get_transaction");

    Ok(())
}
