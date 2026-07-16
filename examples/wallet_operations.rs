//! Wallet operations example for Tenzro SDK
//!
//! This example demonstrates:
//! - Creating a new wallet
//! - Getting wallet balances
//! - Sending tokens (server-custodial path)
//! - Sending tokens self-custody (client-side hybrid signing)

use std::sync::Arc;

use async_trait::async_trait;
use tenzro_sdk::{Address, HybridSigner, TenzroClient, config::SdkConfig, error::SdkResult};

/// A stand-in for a real self-custody signer. In production this wraps a
/// TEE-sealed key (SEV-SNP / TDX / Nitro) or an offline hardware signer that
/// holds the Ed25519 + ML-DSA-65 keypair and never exposes the secret. The
/// runner's raw 32-byte Ed25519 public key IS its account address.
struct DemoSigner {
    ed25519_pk: Vec<u8>,
    ml_dsa_vk: Vec<u8>,
}

#[async_trait]
impl HybridSigner for DemoSigner {
    fn ed25519_public_key(&self) -> Vec<u8> {
        self.ed25519_pk.clone()
    }

    fn ml_dsa_verifying_key(&self) -> Vec<u8> {
        self.ml_dsa_vk.clone()
    }

    async fn sign_hybrid(&self, _message: &[u8]) -> SdkResult<(Vec<u8>, Vec<u8>)> {
        // A real signer computes an Ed25519 signature (64 bytes) and an
        // ML-DSA-65 signature (3309 bytes) over `message`
        // (= Transaction::hash()). Both legs are mandatory; the node rejects
        // a raw send that omits either.
        Ok((vec![0u8; 64], vec![0u8; 3309]))
    }
}

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

    // Self-custody send (TEE-equipped or key-holding runners).
    //
    // The signer holds the Ed25519 + ML-DSA-65 keypair locally. The SDK
    // fetches nonce + chain id, builds the canonical Transaction::hash()
    // preimage (including the PQ verifying key), asks the signer for both
    // legs over that hash, and submits via eth_sendRawTransaction — the node
    // never sees the secret. The signer's raw 32-byte Ed25519 public key is
    // the `from` account.
    println!("\nSending tokens self-custody (client-side hybrid signing)...");
    let signer: Arc<dyn HybridSigner> = Arc::new(DemoSigner {
        ed25519_pk: vec![0u8; 32],
        ml_dsa_vk: vec![0u8; 1952],
    });
    let recipient = Address::zero(); // In production, use a real address
    match wallet
        .send_self_custody(&signer, recipient, 1_000_000_000_000_000_000u128)
        .await
    {
        Ok(hash) => println!("Self-custody transaction hash: {}", hash),
        // A demo signer returns zeroed signatures, which the node rejects —
        // wire a real TEE/hardware signer to complete the submit.
        Err(e) => println!("Self-custody submit (demo signer): {}", e),
    }

    println!("\n=== Example completed successfully! ===");

    Ok(())
}
