//! Canton with API key — end-to-end operator → subject lifecycle.
//!
//! Walks the full per-operator sovereignty flow for a node's
//! Canton-scoped key:
//!
//!   1. The operator (holder of this node's `X-Tenzro-Admin-Token`)
//!      mints a `subject`-class key bound to a Tenzro DID, scoped to
//!      `canton`. The plaintext `tnz_...` token is shown exactly once.
//!   2. The subject (holder of the `tnz_...` key, exported as
//!      `TENZRO_API_KEY`) uses it to call a scope-gated Canton RPC.
//!   3. The subject lists its own keys via `list_mine` (subject-gated
//!      surface — no admin token required).
//!   4. The subject self-revokes the key via `revoke_mine`. Operator
//!      involvement is not needed for the revocation, mirroring the
//!      "key-holder controls their own revocation" property of the
//!      `subject` class.
//!
//! The same key class spectrum is available without changing the SDK
//! call shape — `operator_internal` keys behave the same way over the
//! wire except the subject path is closed, and `operator_protected`
//! keys cannot be revoked via RPC at all (rotate by updating the
//! operator secret and restarting the node).
//!
//! Required environment for a full run against a live node:
//!
//!   TENZRO_RPC_ENDPOINT    — e.g. https://rpc.tenzro.network
//!   TENZRO_ADMIN_TOKEN     — operator's admin token for the target node
//!   TENZRO_SUBJECT_DID     — subject identifier to bind the key to
//!                            (e.g. did:tenzro:human:<uuid>)
//!
//! If `TENZRO_ADMIN_TOKEN` is unset the example prints the plan and
//! exits cleanly — useful in CI where no operator credential is
//! configured.

use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tenzro_sdk::{
    api_key::{CreateApiKeyParams, KeyClass},
    config::SdkConfig,
    TenzroClient,
};

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Canton with API key — operator → subject lifecycle ===\n");

    let admin_token = env::var("TENZRO_ADMIN_TOKEN").ok();
    let subject_did = env::var("TENZRO_SUBJECT_DID")
        .unwrap_or_else(|_| "did:tenzro:human:example-subject".to_string());

    if admin_token.is_none() {
        println!("TENZRO_ADMIN_TOKEN is not set — dry run only.\n");
        println!("To run end-to-end against a live node, export:");
        println!("  TENZRO_ADMIN_TOKEN=<operator token for the target node>");
        println!("  TENZRO_SUBJECT_DID=did:tenzro:human:<uuid>");
        println!("  TENZRO_RPC_ENDPOINT=https://rpc.tenzro.network");
        println!("\nThen re-run this example.");
        return Ok(());
    }

    // ── Step 1: operator mints a subject-class key with `canton` scope ──
    //
    // The operator client picks up `TENZRO_ADMIN_TOKEN` from the env
    // and forwards it as `X-Tenzro-Admin-Token` on every request.
    let operator = TenzroClient::connect(SdkConfig::testnet()).await?;
    println!("1. Operator mints a subject-class Canton key.");
    let created = operator
        .api_key()
        .create(CreateApiKeyParams {
            label: format!("canton-example-{}", unix_secs()),
            subject: Some(subject_did.clone()),
            scopes: vec!["canton".to_string()],
            class: KeyClass::Subject,
            ..Default::default()
        })
        .await?;
    println!("   key_id:    {}", created.key_id);
    println!("   subject:   {:?}", created.subject);
    println!("   scopes:    {:?}", created.scopes);
    println!("   class:     {:?}", created.class);
    println!(
        "   plaintext: {} (shown EXACTLY once — persist immediately)\n",
        created.key
    );

    // ── Step 2: subject uses the key for a scope-gated Canton call ──
    //
    // In a real deployment the subject would export the plaintext
    // `tnz_...` key as `TENZRO_API_KEY` in their own environment, on
    // a machine that has no access to the operator's admin token.
    // Here we build a fresh client with the env override applied for
    // the lifetime of this process so the example runs in one binary —
    // and also clear `TENZRO_ADMIN_TOKEN` so the subject client cannot
    // accidentally piggyback on operator privileges.
    // SAFETY: single-threaded example; no other threads are reading
    // these env vars concurrently. In Rust 2024, `set_var`/`remove_var`
    // are `unsafe` to guard against TOCTOU races in multi-threaded
    // programs (see https://doc.rust-lang.org/std/env/fn.set_var.html).
    unsafe {
        env::set_var("TENZRO_API_KEY", &created.key);
        env::remove_var("TENZRO_ADMIN_TOKEN");
    }
    let subject = TenzroClient::connect(SdkConfig::testnet()).await?;

    println!("2. Subject uses the key to query Canton domains.");
    match subject.canton().list_domains().await {
        Ok(domains) => println!("   Domains returned: {}\n", domains.domains.len()),
        Err(e) => println!("   list_domains failed: {e}\n"),
    }

    // ── Step 3: subject lists its own keys (no admin token needed) ──
    println!("3. Subject lists its own keys via list_mine.");
    let mine = subject.api_key().list_mine().await?;
    println!("   Subject:   {}", mine.subject);
    println!("   Key count: {}\n", mine.keys.len());

    // ── Step 4: subject self-revokes the key ──
    println!("4. Subject self-revokes the key via revoke_mine.");
    let revoked = subject.api_key().revoke_mine(&created.key_id).await?;
    println!("   key_id:    {}", revoked.key_id);
    println!("   revoked:   {}\n", revoked.revoked);

    println!("Done. The key is now inactive — further Canton calls with");
    println!("this key will fail with -32004 (unknown or revoked).");
    Ok(())
}
