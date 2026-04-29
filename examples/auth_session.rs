//! OAuth 2.1 + DPoP auth session example.
//!
//! Walks through the full auth lifecycle a Tenzro client cares about:
//!
//! 1. **Onboard a human** — provisions a TDIP `did:tenzro:human:*` identity,
//!    a fresh MPC wallet, and an OAuth 2.1 access + refresh token pair
//!    (RFC 6749 successor). The access token is an HS256 JWT (1h TTL); the
//!    refresh token is an opaque UUID (30-day TTL, no rotation in V1).
//!
//! 2. **Refresh the access token** — exchange the refresh token for a
//!    fresh access token without re-onboarding.
//!
//! 3. **Link an existing wallet** — mint a fresh access + refresh token
//!    pair against an existing MPC wallet (e.g. one provisioned earlier
//!    via `tenzro_createWallet`).
//!
//! Pass the holder's RFC 7638 SHA-256 thumbprint (`dpop_jkt`) on every
//! call to bind the issued token to a specific Ed25519 holder key. Every
//! subsequent privileged RPC must then accompany the bearer with a fresh
//! DPoP proof in the `DPoP` header signed by the same key (RFC 9449).
//!
//! Run with: `cargo run --example auth_session -p tenzro-sdk`

use tenzro_sdk::{config::SdkConfig, TenzroClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK — Auth Session Example ===\n");

    let client = TenzroClient::connect(SdkConfig::testnet()).await?;
    let auth = client.auth();

    // 1. Onboard a human. `dpop_jkt` is optional but strongly recommended:
    // pass the RFC 7638 thumbprint of the holder's Ed25519 key so the JWT
    // is DPoP-bound to that key.
    println!("Onboarding a new human participant...");
    let session = auth.onboard_human("Alice", None).await?;

    println!("  did:           {}", session.identity["did"]);
    println!("  wallet:        {}", session.wallet["address"]);
    println!(
        "  access_token:  {}…",
        &session.access_token[..session.access_token.len().min(32)]
    );
    println!("  expires_in:    {}s", session.expires_in);
    println!("  refresh_token: {}…", &session.refresh_token[..session.refresh_token.len().min(8)]);
    println!("  dpop_bound:    {}\n", session.dpop_bound);

    // 2. Refresh the access token. The refresh token is NOT rotated in
    // V1 — only the access token changes. Run this whenever the access
    // token nears expiry (default 1h TTL).
    println!("Refreshing the access token...");
    let refreshed = auth.refresh_token(&session.refresh_token, None).await?;
    println!(
        "  new access_token: {}…",
        &refreshed.access_token[..refreshed.access_token.len().min(32)]
    );
    println!("  expires_in:       {}s", refreshed.expires_in);
    println!("  dpop_bound:       {}\n", refreshed.dpop_bound);

    // 3. Link an existing wallet to a new auth session. This is useful
    // when the holder already provisioned a wallet via `tenzro_createWallet`
    // (or imported one) and now wants OAuth-style auth credentials without
    // re-running onboarding.
    let wallet_id = session
        .wallet
        .get("wallet_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    if !wallet_id.is_empty() {
        println!("Linking the existing wallet to a fresh auth session...");
        let linked = auth
            .link_wallet_for_auth(wallet_id, None, Some("Alice (linked)"), None)
            .await?;
        println!(
            "  access_token:  {}…",
            &linked.access_token[..linked.access_token.len().min(32)]
        );
        println!(
            "  refresh_token: {}…\n",
            &linked.refresh_token[..linked.refresh_token.len().min(8)]
        );
    }

    println!("Auth lifecycle exercised end-to-end.");
    Ok(())
}
