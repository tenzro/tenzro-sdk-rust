//! Capital Intent end-to-end example.
//!
//! Opens a CapitalIntent, auto-assigns it to a solver (auto-ranked by
//! ERC-8004 reputation + price + ETA), executes a leg, verifies, and
//! settles. Mirrors the lifecycle documented in
//! `docs/capital-intent.md`.
//!
//! ```bash
//! cargo run -p tenzro-sdk --example capital_intent
//! ```

use serde_json::json;
use tenzro_sdk::TenzroClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TenzroClient::new("https://rpc.tenzro.network").await?;
    let capital = client.capital();

    // 1. Build a signed CapitalIntent payload. Real callers sign over the
    //    canonical bytes with the principal's wallet key.
    let intent = json!({
        "objective": {
            "kind": "acquire",
            "basket": [
                { "asset_id": "0xTBILL_3M", "weight_bps": 6000 },
                { "asset_id": "0xTBILL_6M", "weight_bps": 4000 }
            ]
        },
        "constraints": {
            "max_price": 1_000_000_000_000_u64,
            "max_eta_secs": 3_600,
            "max_slippage_bps": 25
        },
        "compliance": {
            "reg_regime": "us-reg-d-506c",
            "required_kya": ["accredited", "kya-tier-2"],
            "jurisdictions": ["US"]
        },
        "authorization": {
            "principal_did": "did:tenzro:human:0123...",
            "signature": "0xPRINCIPAL_SIG",
            "expires_at": 1_900_000_000_u64
        },
        "settlement_req": {
            "payer":   "0xPAYER",
            "asset_id": "0xUSDC",
            "amount":   "1000000000"
        }
    });

    let opened = capital.open(intent).await?;
    println!("opened: {opened}");
    let intent_id = opened
        .get("intent_id")
        .and_then(|v| v.as_str())
        .unwrap_or("intent-id-here");

    // 2. (Solvers submit bids out-of-band.) Auto-rank by ERC-8004 reputation,
    //    then price, then eta.
    let assigned = capital
        .assign(intent_id, None, true, Some("0xPAYER"), Some("0xCUSTODY_VAULT"))
        .await?;
    println!("assigned: {assigned}");

    // 3. Execute a leg.
    let leg = json!({
        "venue": "venue:dex:0xUSDC-TBILL3M",
        "asset_id": "0xTBILL_3M",
        "side": "acquire",
        "quantity": "600",
        "unit_price": "999500000",
        "settlement_ref": "0xVENUE_RECEIPT_1"
    });
    capital.execute(intent_id, leg).await?;

    // 4. Verify, then settle.
    capital.verify(intent_id).await?;
    capital.settle(intent_id, None).await?;

    // 5. Read final state for audit.
    let state = capital.get(intent_id).await?;
    println!("final state: {state}");
    Ok(())
}
