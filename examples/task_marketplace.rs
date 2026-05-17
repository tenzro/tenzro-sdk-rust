//! Task marketplace settlement cycle — end-to-end against the live testnet.
//!
//! Walks through the full money-moving path:
//!
//! ```text
//!   participate (poster)  → faucet → post_task
//!   participate (provider)         → quote_task
//!                                  → assign_task   (locks price)
//!                                  → complete_task (TNZO transfer)
//!   get_token_balance              → reconcile poster/provider deltas
//! ```
//!
//! `complete_task` is the moneyed step: the RPC handler transfers the
//! locked price (`quoted_price`, falling back to `max_price`) from the
//! poster's wallet to the provider's wallet through the unified token
//! registry. The settlement block returned by the handler contains the
//! post-transfer balances; the example confirms them via
//! `tenzro_getTokenBalance`.
//!
//! Run with: `cargo run --example task_marketplace`
//! Note: the testnet faucet uses a single MPC wallet — funding may take
//! up to ~120s under contention.
//!
//! Honest caveat: `participate` currently takes a password string that
//! the live RPC handler silently ignores. We pass empty strings to make
//! that explicit.

use tenzro_sdk::{
    config::SdkConfig,
    task::QuoteOpts,
    types::Address,
    TenzroClient,
};
use tokio::time::{sleep, Duration};

const POST_PRICE_WEI: u128 = 1_000_000_000_000_000_000;   // 1 TNZO
const QUOTE_PRICE_WEI: u128 = 900_000_000_000_000_000;    // 0.9 TNZO
const FAUCET_POLL_SECS: u64 = 180;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK Task Marketplace — Settlement Cycle ===\n");

    let client = TenzroClient::connect(SdkConfig::testnet()).await?;
    let provider_api = client.provider();
    let task = client.task();
    let token = client.token();

    // ------------------------------------------------------------------
    // 1. Spawn poster + provider identities (each gets an MPC wallet)
    // ------------------------------------------------------------------
    println!("1. Spawning poster and provider identities...");
    let poster_resp = provider_api.participate("").await?;
    let provider_resp = provider_api.participate("").await?;

    let poster_addr = Address::from_hex(&poster_resp.address)
        .ok_or("poster address parse failed")?;
    let provider_addr = Address::from_hex(&provider_resp.address)
        .ok_or("provider address parse failed")?;

    println!("   poster   {}", poster_resp.address);
    println!("   provider {}\n", provider_resp.address);

    // ------------------------------------------------------------------
    // 2. Fund the poster from the faucet, poll until visible
    // ------------------------------------------------------------------
    println!("2. Requesting faucet TNZO for poster...");
    let faucet = client.request_faucet(poster_addr.clone()).await?;
    println!("   faucet response: {:?}", faucet);

    println!("   polling balance (up to {}s)...", FAUCET_POLL_SECS);
    let mut funded = false;
    let elapsed = std::time::Instant::now();
    while elapsed.elapsed().as_secs() < FAUCET_POLL_SECS {
        let balance = client.get_balance(poster_addr.clone()).await.unwrap_or(0);
        if balance >= POST_PRICE_WEI {
            println!(
                "   funded after {}s — balance {} wei\n",
                elapsed.elapsed().as_secs(),
                balance
            );
            funded = true;
            break;
        }
        sleep(Duration::from_secs(3)).await;
    }
    if !funded {
        return Err(format!(
            "poster never funded within {}s — testnet faucet busy",
            FAUCET_POLL_SECS
        )
        .into());
    }

    // ------------------------------------------------------------------
    // 3. Snapshot pre-settlement balances
    // ------------------------------------------------------------------
    let poster_before = client.get_balance(poster_addr.clone()).await?;
    let provider_before = client.get_balance(provider_addr.clone()).await?;
    println!("3. Pre-settlement balances:");
    println!("   poster    {} wei", poster_before);
    println!("   provider  {} wei\n", provider_before);

    // ------------------------------------------------------------------
    // 4. Poster opens a task
    // ------------------------------------------------------------------
    println!("4. Posting task (max_price 1 TNZO, type=inference)...");
    let posted = task
        .post_task(
            "Sentiment analysis: 2 reviews",
            "Score sentiment 1-5 for each input review.",
            "inference",
            POST_PRICE_WEI,
            r#"["Great product!", "Needs improvement."]"#,
            &poster_addr,
        )
        .await?;
    println!("   task_id  {}", posted.task_id);
    println!("   status   {:?}\n", posted.status);

    // ------------------------------------------------------------------
    // 5. Provider submits a quote
    // ------------------------------------------------------------------
    println!("5. Provider quotes at 0.9 TNZO...");
    let opts = QuoteOpts {
        model_id: Some("gemma3-270m".to_string()),
        confidence: Some(90),
        estimated_duration_secs: Some(45),
        notes: Some("Quick sentiment scoring with Gemma 3 270M.".to_string()),
    };
    let quote = task
        .quote_task(&posted.task_id, &provider_addr, QUOTE_PRICE_WEI, opts)
        .await?;
    println!("   price    {} wei", quote.price);
    println!("   model    {}\n", quote.model_id);

    // ------------------------------------------------------------------
    // 6. Poster assigns to provider, locking the quoted price
    // ------------------------------------------------------------------
    println!("6. Poster assigns task to provider (locks 0.9 TNZO)...");
    let assignment = task
        .assign_task(&posted.task_id, &provider_addr, Some(QUOTE_PRICE_WEI))
        .await?;
    println!("   assignment: {}\n", assignment);

    // ------------------------------------------------------------------
    // 7. Provider completes the task — settlement fires on-chain
    // ------------------------------------------------------------------
    println!("7. Completing task (triggers on-chain TNZO transfer)...");
    let receipt = task
        .complete_task(
            &posted.task_id,
            "[{\"review\":\"Great product!\",\"score\":5},{\"review\":\"Needs improvement.\",\"score\":2}]",
        )
        .await?;
    println!("   status      {}", receipt.status);
    println!("   settlement  {}\n", receipt.settlement);

    // ------------------------------------------------------------------
    // 8. Reconcile balances via tenzro_getTokenBalance
    // ------------------------------------------------------------------
    println!("8. Post-settlement balances:");
    let poster_after = client.get_balance(poster_addr.clone()).await?;
    let provider_after = client.get_balance(provider_addr.clone()).await?;
    println!("   poster    {} wei  (Δ {} wei)", poster_after, poster_before as i128 - poster_after as i128);
    println!("   provider  {} wei  (Δ +{} wei)", provider_after, provider_after - provider_before);

    // Multi-VM view: same native balance is visible from EVM/SVM/DAML
    println!("\n9. Provider balance via cross-VM views (pointer model):");
    let mv = token.get_token_balance(&provider_resp.address, None).await?;
    println!("   {:#?}", mv);

    println!("\n=== Settlement cycle complete ===");
    Ok(())
}
