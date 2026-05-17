//! Multi-VM settlement — the pointer-model invariant against live testnet.
//!
//! One TNZO balance, four VM views:
//!
//! ```text
//!   native (18-dec wei)            ─┐
//!   evm_wtnzo  (ERC-20, 18-dec)    │  all share the same underlying
//!   svm_wtnzo  (SPL, 9-dec)        │  native balance via the Sei V2
//!   daml_holding (CIP-56, 18-dec)  ─┘  pointer model — no bridge
//! ```
//!
//! Walks through:
//!   1. spawn two fresh agents (sender + receiver) via `participate`
//!   2. fund the sender from the faucet
//!   3. snapshot all four VM views of the sender's balance
//!   4. drive a `tenzro_crossVmTransfer` (native → evm)
//!   5. re-snapshot both addresses
//!   6. assert the pointer-model invariants:
//!        • native == evm_wtnzo       (EVM tracks native 1:1)
//!        • native == daml_holding    (Canton tracks native 1:1)
//!        • svm_wtnzo == native / 1e9 (SPL is 9-decimal truncation)
//!        • sender_before == sender_after + receiver_after  (conservation)
//!
//! Run with: `cargo run --example multivm_settlement`
//!
//! Honest caveat: the testnet handler accepts `tenzro_submitDamlCommand`
//! and returns a typed Canton envelope, but no live Canton participant
//! is wired up. What this example demonstrates is that `daml_holding`
//! is a real view of the underlying balance — the post-transfer delta
//! confirms it.

use tenzro_sdk::{config::SdkConfig, types::Address, TenzroClient};
use tokio::time::{sleep, Duration};

const TRANSFER_AMOUNT_WEI: u128 = 1_000_000_000_000_000_000; // 1 TNZO
const FAUCET_POLL_SECS: u64 = 180;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK Multi-VM Settlement ===\n");

    let client = TenzroClient::connect(SdkConfig::testnet()).await?;
    let provider_api = client.provider();
    let token = client.token();

    // ------------------------------------------------------------------
    // 1. Spawn sender + receiver
    // ------------------------------------------------------------------
    println!("1. Spawning sender and receiver identities...");
    let sender = provider_api.participate("").await?;
    let receiver = provider_api.participate("").await?;
    let sender_addr =
        Address::from_hex(&sender.address).ok_or("sender address parse failed")?;
    println!("   sender   {}", sender.address);
    println!("   receiver {}\n", receiver.address);

    // ------------------------------------------------------------------
    // 2. Fund sender from faucet
    // ------------------------------------------------------------------
    println!("2. Requesting faucet TNZO for sender...");
    let _ = client.request_faucet(sender_addr.clone()).await?;

    println!("   polling balance (up to {}s)...", FAUCET_POLL_SECS);
    let started = std::time::Instant::now();
    let mut funded = false;
    while started.elapsed().as_secs() < FAUCET_POLL_SECS {
        let b = client.get_balance(sender_addr.clone()).await.unwrap_or(0);
        if b >= TRANSFER_AMOUNT_WEI {
            println!("   funded after {}s\n", started.elapsed().as_secs());
            funded = true;
            break;
        }
        sleep(Duration::from_secs(3)).await;
    }
    if !funded {
        return Err(format!(
            "sender never funded within {}s — testnet faucet busy",
            FAUCET_POLL_SECS
        )
        .into());
    }

    // ------------------------------------------------------------------
    // 3. Snapshot pre-transfer four-view balance
    // ------------------------------------------------------------------
    println!("3. Pre-transfer balance (4 VM views):");
    let pre = token.get_token_balance(&sender.address, None).await?;
    print_balance(&pre);

    // Assert pre-transfer pointer-model invariants
    let native = parse_u128(&pre.native.balance_wei)?;
    let evm = parse_u128(&pre.evm_wtnzo.balance_wei)?;
    let spl = parse_u128(&pre.svm_wtnzo.balance_base_units)?;
    let daml = parse_u128(&pre.daml_holding.amount_wei)?;
    assert_eq!(native, evm, "EVM view diverges from native");
    assert_eq!(native, daml, "DAML view diverges from native");
    assert_eq!(spl, native / 1_000_000_000, "SVM truncation incorrect");
    println!("   invariants pre-transfer hold\n");

    // ------------------------------------------------------------------
    // 4. Cross-VM transfer (native → evm)
    // ------------------------------------------------------------------
    println!(
        "4. tenzro_crossVmTransfer 1 TNZO sender(native) → receiver(evm)..."
    );
    let result = token
        .cross_vm_transfer(
            "TNZO",
            &TRANSFER_AMOUNT_WEI.to_string(),
            "native",
            "evm",
            &sender.address,
            &receiver.address,
        )
        .await?;
    println!("   status: {}\n", result.status);

    // ------------------------------------------------------------------
    // 5. Snapshot post-transfer views for both addresses
    // ------------------------------------------------------------------
    println!("5. Post-transfer sender balance:");
    let sender_post = token.get_token_balance(&sender.address, None).await?;
    print_balance(&sender_post);

    println!("\n   Post-transfer receiver balance:");
    let receiver_post = token.get_token_balance(&receiver.address, None).await?;
    print_balance(&receiver_post);

    // ------------------------------------------------------------------
    // 6. Conservation + invariants post-transfer
    // ------------------------------------------------------------------
    let s_native = parse_u128(&sender_post.native.balance_wei)?;
    let r_native = parse_u128(&receiver_post.native.balance_wei)?;
    let s_evm = parse_u128(&sender_post.evm_wtnzo.balance_wei)?;
    let r_evm = parse_u128(&receiver_post.evm_wtnzo.balance_wei)?;
    let s_spl = parse_u128(&sender_post.svm_wtnzo.balance_base_units)?;
    let s_daml = parse_u128(&sender_post.daml_holding.amount_wei)?;

    assert_eq!(s_native, s_evm, "sender EVM view diverges from native");
    assert_eq!(r_native, r_evm, "receiver EVM view diverges from native");
    assert_eq!(s_native, s_daml, "sender DAML view diverges from native");
    assert_eq!(
        s_spl,
        s_native / 1_000_000_000,
        "sender SVM truncation incorrect"
    );
    assert_eq!(
        native,
        s_native + r_native,
        "conservation broken: {} != {} + {}",
        native,
        s_native,
        r_native
    );

    println!("\n6. Pointer-model invariants post-transfer: ALL PASS");
    println!("   • native == evm_wtnzo (sender & receiver)");
    println!("   • native == daml_holding (sender)");
    println!("   • svm_wtnzo == native / 1e9 (sender)");
    println!("   • conservation: sender_before == sender_after + receiver_after");

    println!("\n=== Multi-VM settlement complete ===");
    Ok(())
}

fn parse_u128(s: &str) -> Result<u128, Box<dyn std::error::Error>> {
    Ok(s.parse::<u128>()?)
}

fn print_balance(b: &tenzro_sdk::token::TokenBalance) {
    println!(
        "   native       {} wei",
        b.native.balance_wei
    );
    println!(
        "   evm_wtnzo    {} wei",
        b.evm_wtnzo.balance_wei
    );
    println!(
        "   svm_wtnzo    {} base units (9 dec)",
        b.svm_wtnzo.balance_base_units
    );
    println!(
        "   daml_holding {} wei",
        b.daml_holding.amount_wei
    );
}
