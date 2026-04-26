//! Settlement and payment example for Tenzro SDK
//!
//! This example demonstrates:
//! - Creating escrows
//! - Releasing escrow payments
//! - Opening payment channels
//! - Submitting settlement requests

use tenzro_sdk::{TenzroClient, SettlementRequest, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK Settlement Example ===\n");

    // Connect to testnet
    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;
    let settlement = client.settlement();

    // Create an on-chain escrow via signed CreateEscrow transaction.
    //
    // Replace the payer address with a wallet you control. The escrow_id is
    // derived deterministically by the VM as
    // SHA-256("tenzro/escrow/id/v1" || payer || nonce_le); this example shows
    // only the resulting transaction hash, since the actual escrow_id is
    // observable via the receipt logs once the tx finalizes.
    //
    // Authentication is ambient: this call carries the OAuth 2.1 bearer JWT
    // and per-request DPoP proof from `TENZRO_BEARER_JWT` /
    // `TENZRO_DPOP_PROOF` (set after onboarding via `client.auth()`).
    // Signing happens server-side against the holder's MPC wallet — no raw
    // private key crosses the SDK surface.
    println!("Creating an on-chain escrow (signed CreateEscrow)...");
    let payer_addr = std::env::var("TENZRO_PAYER")
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000000000000000000000000001".to_string());

    let create_tx_hash = settlement.create_escrow(
        &payer_addr,
        "0x0000000000000000000000000000000000000000000000000000000000000002",
        1_000_000_000_000_000_000u128,         // 1 TNZO in wei
        "TNZO",
        u64::MAX,                              // expires_at (no expiry)
        "both_signatures",
    ).await?;

    println!("CreateEscrow tx submitted: {}", create_tx_hash);
    println!("  Amount: 1 TNZO");
    println!("  Conditions: BothSignatures\n");

    // For the rest of this example, replace `escrow_id` with the value emitted
    // by the receipt log of the CreateEscrow tx. Until then, this example uses
    // a placeholder zero id.
    let escrow_id: [u8; 32] = [0u8; 32];

    // Open a payment channel
    println!("Opening a micropayment channel...");
    let channel_id = settlement.open_payment_channel(
        "0x0000000000000000000000000000000000000001",
        10000000,
    ).await?;

    println!("Payment channel opened: {}", channel_id);
    println!("  Initial deposit: 10000000 TNZO\n");

    // Submit a settlement request
    println!("Submitting a settlement request...");
    let request = SettlementRequest {
        request_id: "req-demo-001".to_string(),
        provider: "0x0000000000000000000000000000000000000002".to_string(),
        customer: "0x0000000000000000000000000000000000000003".to_string(),
        amount: 5000,
        asset: "TNZO".to_string(),
    };

    let receipt = settlement.settle(request).await?;

    println!("Settlement completed!");
    println!("  Receipt ID: {}", receipt.receipt_id);
    println!("  Transaction hash: {}", receipt.tx_hash);
    println!("  Status: {}", receipt.status);

    // Release the escrow (only the original payer can do this).
    println!("\nReleasing escrow...");
    let release_proof = vec![1, 2, 3, 4];
    let tx_hash = settlement
        .release_escrow(&payer_addr, escrow_id, release_proof)
        .await?;
    println!("ReleaseEscrow tx submitted!");
    println!("  Transaction hash: {}", tx_hash);

    println!("\n=== Example completed successfully! ===");

    Ok(())
}
