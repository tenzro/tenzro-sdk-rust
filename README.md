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
| `auth` | OAuth 2.1 + DPoP onboarding (human, delegated agent, autonomous agent), JWT/DID revocation, HITL approvals |
| `wallet` | Create wallets, check balances, send transactions |
| `identity` | TDIP DIDs, credentials, usernames, delegation |
| `agent` | Register agents, spawn, swarms, messaging |
| `inference` | Model discovery, chat, and multi-modal inference (forecast, vision embed/similarity, text embedding, segmentation, detection, audio ASR, video embed) — modality-aware routing via `tenzro_forecast`, `tenzro_visionEmbed`, `tenzro_textEmbed`, `tenzro_segment`, `tenzro_detect`, `tenzro_transcribe`, `tenzro_videoEmbed` |
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

## Auth (OAuth 2.1 + DPoP Onboarding)

Onboarding uses OAuth 2.1 (RFC 6749 successor) + DPoP-bound JWTs (RFC 9449)
+ Rich Authorization Requests (RFC 9396). Participants — humans, delegated
agents under a human controller, and fully autonomous agents — onboard via
three RPCs that each provision a TDIP identity + MPC wallet and return a
JWT bound to a holder-supplied DPoP `jkt` (RFC 7638 thumbprint of the
holder's Ed25519 public key).

```rust
use tenzro_sdk::TenzroClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TenzroClient::new("https://rpc.tenzro.network").await?;
    let auth = client.auth();

    // Onboard a new human — returns identity, MPC wallet, and access token
    let session = auth.onboard_human("Alice", None).await?;
    println!("DID:    {}", session.identity["did"]);
    println!("Wallet: {}", session.wallet["address"]);
    println!("Token:  {}…", &session.access_token[..32]);

    // Subsequent privileged calls authenticate ambiently — the SDK forwards
    // these env vars as `Authorization: DPoP <jwt>` and `DPoP: <proof>` on
    // every JSON-RPC request.
    std::env::set_var("TENZRO_BEARER_JWT", &session.access_token);
    std::env::set_var("TENZRO_DPOP_PROOF", "<freshly minted DPoP proof>");

    // Onboard a delegated agent under Alice's act-chain
    let did = session.identity["did"].as_str().unwrap_or_default().to_string();
    let _agent = auth.onboard_delegated_agent(
        &did,
        vec!["inference".into(), "settlement".into()],
        serde_json::json!({
            "max_transaction_value": "1000000000000000000",
            "allowed_chains": ["tenzro"],
        }),
        None,
    ).await?;

    // Revoke (cascades through act-chain by DID)
    auth.revoke_did(&did, Some("lost device")).await?;

    Ok(())
}
```

Holder-side DPoP proof generation is left to the caller — sign a per-request
JWT with your Ed25519 holder key (whose RFC 7638 thumbprint matches `dpop_jkt`)
and the JWS-compact form lands in `TENZRO_DPOP_PROOF`. See RFC 9449 §4.

## Transaction signing

The Tenzro node canonicalises the transaction hash over `Transaction::hash()`,
which includes the server-supplied `timestamp` field. Every transaction is
synchronously verified against its Ed25519 signature before acceptance; an
invalid or missing signature returns JSON-RPC error `-32003`.

All signing is **ambient and server-side**. `tenzro_signAndSendTransaction`
resolves the signer from the DPoP-bound bearer JWT (`TENZRO_BEARER_JWT`)
and signs against the holder's MPC wallet. Two supported flows:

1. **Atomic server-side sign + send (recommended).** With ambient auth
   configured, the SDK forwards the bearer + DPoP proof; the node assembles,
   hashes, signs against the MPC wallet bound to the JWT, verifies, and
   submits — all in one call.

   ```rust
   let tx_hash: String = client
       .rpc()
       .call("tenzro_signAndSendTransaction", serde_json::json!([{
           "from": "0x...",
           "to": "0x...",
           "value": "0x...",
           "nonce": "0x0",
           "chain_id": 1337,
       }]))
       .await?;
   ```

2. **Pre-signed submission.** For holders who want to control signing
   client-side, call `eth_sendRawTransaction` directly with `signature`,
   `public_key`, and explicit `timestamp` matching a client-computed
   `Transaction::hash()`. The signer must still match a holder identity
   visible to the node.

## Durable state

The node persists AI infrastructure to RocksDB and restores it on restart —
SDK consumers see consistent state across node upgrades and reboots:

- **Model catalog** — `ModelRegistry` writes `ModelInfo` records under
  `info:<model_id>` in `CF_MODELS`; models survive restart without
  re-registration.
- **Agent runtime** — `AgentRuntime` persists `RegisteredAgent`,
  `AgentLifecycleInfo`, and parent→children spawn trees under
  `agent:`/`lifecycle:`/`children:` prefixes in `CF_AGENTS`. Terminated
  agents are retained for audit of `state_history`, `registration_fee`,
  and `tenzro_did`.
- **Swarms** — `SwarmManager` persists `SwarmState` under `swarm:<swarm_id>`
  in `CF_AGENTS` with write-through on create, status transitions, and
  termination.

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
