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

    // Create an escrow
    println!("Creating an escrow...");
    let escrow_id = settlement.create_escrow(
        "0x0000000000000000000000000000000000000001",
        1000000,
        "TNZO",
        "both_signatures",
    ).await?;

    println!("Escrow created: {}", escrow_id);
    println!("  Amount: 1000000 TNZO (smallest unit)");
    println!("  Conditions: BothSignatures\n");

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

    // Release the escrow
    println!("\nReleasing escrow...");
    let release_proof = vec![1, 2, 3, 4];
    let tx_hash = settlement.release_escrow(&escrow_id, release_proof).await?;
    println!("Escrow released!");
    println!("  Transaction hash: {}", tx_hash);

    println!("\n=== Example completed successfully! ===");

    Ok(())
}
