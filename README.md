# Tenzro SDK for Rust

[![Crates.io](https://img.shields.io/crates/v/tenzro-sdk)](https://crates.io/crates/tenzro-sdk)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-tenzro.com-blue)](https://tenzro.com/docs/rust-sdk)

The official Rust SDK for [Tenzro Network](https://tenzro.com) -- build AI-native applications with wallets, identity, agents, inference, cross-chain bridge, crypto, TEE, ZK proofs, and settlement.

## Installation

```toml
[dependencies]
tenzro-sdk = "0.1"
```

Or:
```bash
cargo add tenzro-sdk
```

## Quick Start

```rust
use tenzro_sdk::TenzroClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TenzroClient::new("https://rpc.tenzro.network").await?;

    // Create wallet
    let wallet = client.wallet().create_wallet().await?;
    println!("Address: {}", wallet.address);

    // Register identity
    let identity = client.identity().register_human("Alice").await?;
    println!("DID: {}", identity.did);

    // List AI models
    let models = client.inference().list_models().await?;
    println!("{} models available", models.len());

    Ok(())
}
```

## Modules (37)

| Module | Description |
|--------|-------------|
| `auth` | Issue, list, revoke, and validate onboarding keys |
| `wallet` | Create wallets, check balances, send transactions |
| `identity` | TDIP DIDs, credentials, usernames, delegation |
| `agent` | Register agents, spawn, swarms, messaging |
| `inference` | Model discovery, inference requests, streaming |
| `token` | Create tokens, cross-VM transfers, registry |
| `nft` | Collections, minting, transfers, cross-VM pointers |
| `bridge` | LayerZero, CCIP, deBridge, LI.FI bridging |
| `settlement` | Escrow, micropayments, batch settlement |
| `payment` | MPP, x402, AP2 payment protocols |
| `governance` | Proposals, voting, delegation |
| `staking` | Stake, unstake, rewards |
| `compliance` | ERC-3643, KYC enforcement, freeze/unfreeze |
| `crypto` | Sign, verify, encrypt, decrypt, hash, key exchange |
| `tee` | TEE attestation, seal/unseal, confidential compute |
| `zk` | ZK proof generation, verification, circuits |
| `custody` | MPC wallets, keystores, sessions, spending limits |
| `streaming` | Real-time inference, event streams |
| `app` | AppClient, master wallets, paymaster, user management |
| `contract` | Deploy contracts, eth_call, ABI encoding |
| `debridge` | deBridge DLN search, swap, cross-chain |
| `events` | Event queries, subscriptions, webhooks |
| `task` | Task marketplace, quotes, assignment |
| `marketplace` | Agent templates, discovery, spawning |
| `skill` | Skills registry |
| `tool` | Tools registry |
| `canton` | Canton/DAML contracts |
| `provider` | Hardware, model serving, scheduling |
| `ap2` | Agentic Payment Protocol |
| `agent_payments` | Agent spending policies |
| `circuit_breaker` | Provider health management |
| `nanopayment` | Micropayment channels |
| `erc7802` | Cross-chain token mint/burn |
| `types` | Core types (Address, ModelInfo, etc.) |
| `config` | SDK configuration |
| `rpc` | JSON-RPC client |
| `error` | Error types |

## Auth (Onboarding Keys)

```rust
use tenzro_sdk::{TenzroClient, identity::IdentityType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TenzroClient::new("https://rpc.tenzro.network").await?;
    let auth = client.auth();

    // Issue an onboarding key
    let key = auth.issue_onboarding_key(
        "Alice",
        "did:tenzro:human:abc123",
        "0x1234abcd",
        IdentityType::Human,
    ).await?;
    println!("Key: {}", key.key);
    println!("Expires: {}", key.expires_at);

    // List all active keys
    let keys = auth.list_onboarding_keys().await?;
    for k in &keys {
        println!("{} — {} ({})", k.name, k.did, k.status);
    }

    // Validate a key
    let result = auth.validate_onboarding_key(&key.key).await?;
    if result.valid {
        println!("Valid for DID: {}", result.did.unwrap_or_default());
    }

    // Revoke a key
    let revoke = auth.revoke_onboarding_key("did:tenzro:human:abc123").await?;
    println!("Revoked: {}", revoke.revoked);

    Ok(())
}
```

## AppClient (Developer Pattern)

```rust
use tenzro_sdk::AppClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = AppClient::new(
        "https://rpc.tenzro.network",
        "master-private-key",
    ).await?;

    // Create user wallet (funded from master)
    let user = app.create_user_wallet("alice", 1_000_000_000_000_000_000).await?;

    // Sponsor inference (master pays)
    let result = app.sponsor_inference(&user.address, "gemma3-270m", "Hello").await?;
    println!("{}", result.output);

    Ok(())
}
```

## Examples

```bash
cargo run --example basic_usage
cargo run --example wallet_operations
cargo run --example ai_inference
cargo run --example agents
cargo run --example app_developer
cargo run --example task_marketplace
cargo run --example agent_marketplace
cargo run --example governance
cargo run --example settlement
cargo run --example complete_example
```

See the [examples/](examples/) directory and [Tenzro Cookbook](https://github.com/tenzro/tenzro-cookbook).

## Live Testnet

| Endpoint | URL |
|----------|-----|
| JSON-RPC | `https://rpc.tenzro.network` |
| Web API | `https://api.tenzro.network` |
| MCP Server | `https://mcp.tenzro.network/mcp` |
| A2A Server | `https://a2a.tenzro.network` |

## Documentation

- [Rust SDK Reference](https://tenzro.com/docs/rust-sdk)
- [Tutorials](https://tenzro.com/tutorials)
- [Cookbook](https://github.com/tenzro/tenzro-cookbook)
- [API Reference](https://tenzro.com/docs/api-reference)

## Contact

- Website: [tenzro.com](https://tenzro.com)
- Engineering: [eng@tenzro.com](mailto:eng@tenzro.com)
- GitHub: [github.com/tenzro](https://github.com/tenzro)

## License

Apache 2.0. See [LICENSE](LICENSE).
