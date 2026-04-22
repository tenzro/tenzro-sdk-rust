//! Wallet operations example for Tenzro SDK
//!
//! This example demonstrates:
//! - Creating a new wallet
//! - Getting wallet balances
//! - Sending tokens

use tenzro_sdk::{Address, TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK Wallet Operations Example ===\n");

    // Connect to testnet
    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;
    let wallet = client.wallet();

    // Create a new wallet
    println!("Creating a new MPC wallet...");
    let info = wallet.create_wallet().await?;
    println!("Wallet created: {}\n", info.address);

    // Get all balances
    println!("Fetching wallet balances...");
    let address = Address::zero(); // Use a real address in production
    let balances = wallet.get_all_balances(address.clone()).await?;
    println!("Wallet address: {}", balances.address);
    println!("\nBalances:");
    for balance in &balances.balances {
        println!(
            "  {}: {} (raw: {})",
            balance.symbol,
            balance.as_decimal(),
            balance.balance
        );
    }

    // Get specific TNZO balance
    println!("\nFetching TNZO balance specifically...");
    let tnzo_balance = wallet.get_balance(address.clone()).await?;
    println!("TNZO balance: {} (raw units)", tnzo_balance);

    // Send tokens (demonstration)
    println!("\nSending tokens...");
    let from_address = address;
    let to_address = Address::zero(); // In production, use a real address
    let amount = 1000000000000000000u64; // 1.0 TNZO
    let tx_hash = wallet.send(from_address, to_address, amount).await?;
    println!("Transaction hash: {}", tx_hash);

    println!("\n=== Example completed successfully! ===");

    Ok(())
}
