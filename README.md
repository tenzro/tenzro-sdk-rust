# Tenzro SDK for Rust

[![Crates.io](https://img.shields.io/crates/v/tenzro-sdk)](https://crates.io/crates/tenzro-sdk)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-tenzro.com-blue)](https://tenzro.com/docs/rust-sdk)

The official Rust SDK for [Tenzro Network](https://tenzro.com) -- build AI-native applications with wallets, identity, agents, inference, cross-chain bridge, crypto, TEE, ZK proofs, and settlement.

## Installation

```toml
[dependencies]
tenzro-sdk = "0.3"
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

## Catch-up sync

A node lagging behind the network can pull batches of historical blocks via
`get_block_range`. The call returns up to 256 blocks per request with a
`next_height` + `more_available` cursor for paginating past pruning gaps:

```rust
let mut cur = 0u64;
loop {
    let r = client.get_block_range(cur, cur + 255, Some(256)).await?;
    for b in &r.blocks { /* import block */ }
    if !r.more_available { break; }
    cur = r.next_height;
}
```

`client.syncing()` reports the live gap by comparing the local tip against
peer-reported network tips (gossiped on `tenzro/status`); pair it with
`get_block_range` to drive a catch-up loop only when needed.

## Modules (68)

| Module | Description |
|--------|-------------|
| `auth` | OAuth 2.1 + DPoP onboarding (human, delegated agent, autonomous agent), JWT/DID revocation, HITL approvals |
| `wallet` | Create wallets, check balances, send transactions |
| `identity` | TDIP DIDs, credentials, usernames, delegation |
| `agent` | Register agents, spawn, swarms, messaging |
| `inference` | Model discovery, chat, and multi-modal inference (forecast, vision embed/similarity, text embedding, segmentation, detection, audio ASR, video embed) — modality-aware routing via `tenzro_forecast`, `tenzro_visionEmbed`, `tenzro_textEmbed`, `tenzro_segment`, `tenzro_detect`, `tenzro_transcribe`, `tenzro_videoEmbed`. Streaming: `chat_stream` (token stream), `chat_stream_channel` (per-token billing on a micropayment channel) |
| `token` | Create tokens, cross-VM transfers, registry |
| `nft` | Collections, minting, transfers, cross-VM pointers |
| `bridge` | LayerZero, CCIP, deBridge, LI.FI bridging |
| `lifi` | LI.FI direct: chains, tokens, quotes, route execution |
| `wormhole` | Wormhole: 19-guardian VAAs, 30+ chains incl. Solana |
| `cct` | Chainlink CCT v1.6+ pool registry (LockRelease + BurnMint) |
| `erc8004` | Trustless Agents Registry: register, feedback, validation |
| `settlement` | Escrow, micropayments, batch settlement |
| `payment` | MPP, x402, AP2 payment protocols. `list_x402_schemes()` discovers pluggable scheme adapters (`exact`, `permit2`) |
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
| `memory` | Per-agent memory tier — `grant()`, `recall()`, `archive()`, `list_records()`. Lance vector kNN + Tantivy BM25 hybrid (RRF k=60). **Requires DPoP+JWT auth** (`TENZRO_BEARER_JWT` + `TENZRO_DPOP_PROOF`) — bearer's DID must match `agent_did` or its `controller_did`. |
| `skill` | Skills registry |
| `tool` | Tools registry |
| `canton` | Canton 3.5+ JSON Ledger API surface. Reads: `list_domains()`, `list_contracts()`, `list_parties()`, `list_packages()`, `health()`, `version()`, `get_my_user()`, `canton_coin_balance()` (CIP-56), `fee_schedule()`, `connected_synchronizers()`, `get_transaction(update_id)`, `list_user_rights(user_id?)`, `get_my_analytics()`, `list_api_key_analytics(key_id?)`. Writes: `submit_create_command()`, `submit_exercise_command()`, `allocate_party(hint, display_name?)`, `grant_user_rights(user_id?, party, can_act_as, can_read_as, identity_provider_id?)`, `upload_dar(bytes)`. Wire shape verified against Canton 3.5.1: the `submit-and-wait-for-transaction` request body nests `JsCommands` under a top-level `commands` key, each command is externally-tagged (`{CreateCommand: {...}}` / `{ExerciseCommand: {...}}`). Routes through the Tenzro node's canton-scoped RPC. When the presenting API key has a bound `canton_user_id`, the node auto-forwards `actAs` / `requestingParties` as the tenant's `primaryParty`. Stage 2.b (`identity_providers.enabled` on the node) auto-mints a per-tenant upstream OAuth client at issuance; the credentials stay on the node, which mints + forwards the tenant's Canton JWT internally — the `tnz_...` API key is the tenant's only credential. `client.api_keys().create(...)` returns non-secret `tenant_oauth_client` metadata (client_id + issuer_url + audience). |
| `provider` | Hardware, model serving, scheduling |
| `ap2` | Agentic Payment Protocol |
| `agent_payments` | Agent spending policies |
| `capital` | Regulated capital-allocation intents — `open`, `quote`, `assign`, `execute`, `verify`, `compensate`, `settle`, `submit_reserve_attestation`, `get_reserve`, `attested_mint` (1:1 reserve-attested issuance) |
| `workflow` | Multi-party saga workflows — `open`, `step_execute`, `step_verify`, `step_compensate`, `finalize`, `mirror_to_canton`, `verify_did_envelope`, lifecycle + receipt readers, AP2 / x402 / MPP / Stripe SPT / Visa TAP / Mastercard Agent Pay mandate binding |
| `eip7702` | Pectra Type-4 delegation registry — `install_delegation`, `get_delegation`, `revoke_delegation` |
| `erc7683` | Cross-chain intents origin opener + destination fill registry |
| `permit2` | Permit2 `SignatureTransfer` (`domain_separator`, `digest`, `verify_and_consume`, `nonce_used`) with optional witness binding for ERC-7683 origin opens |
| `secure_mint` | Per-token 1:1 reserve-attestation invariant for tokenized RWAs (`set_policy`, `get_policy`, `check`, `apply`, `record_burn`) |
| `hyperlane` | Hyperlane V3 messaging with sovereign Tenzro-validator-set ISM (`list_chains`, `quote_dispatch`, `dispatch`, `get_message`) |
| `axelar` | Axelar GMP — Cosmos / Move / Stellar / XRPL reach (`call_contract`, `pay_gas`, `get_message`, `list_chains`) |
| `babylon` | Babylon Bitcoin staking finality-providers + EOTS delegations (`register_finality_provider`, `submit_finality_signature`, `total_stake_for_provider`, `list_delegations`) |
| `caip` | Chain-agnostic discovery (`caip2`, `caip10`, `caip19`) per submitted `tenzro` CASA namespace (`ChainAgnostic/namespaces#184`) |
| `bridge_fee` | Cross-chain bridge fees in TNZO — `quote()` for destination-native fees, `list_sponsorship_pools()` for per-adapter vault state, `sponsor()` against a previously-quoted envelope, `get_analytics()` for self-read CU consumption, `list_analytics()` for operator cross-tenant read. Admin-only mutations: `set_rate()`, `set_refill_threshold()`. Requires `chainlink` API key scope on the node. |
| `urwa` | ERC-7943 (uRWA) compliance surface: token freeze, kill-switch, forced-transfer mutations for tokenized RWA pool admins (admin-token-gated) |
| `ivms101` | FATF Travel Rule IVMS101 v1.1.0 canonical envelope helpers for KYC payloads on cross-border transfers |
| `attested_clock` | TEE-attested-timestamp envelope for saga step deadlines + obligation expiry — 30s drift tolerance |
| `signed_agent_card` | A2A v1.0 SignedAgentCard JWS envelope helpers + canonical-hash computation for verifier rebinding |
| `wormhole_ntt` | Wormhole NTT (Native Token Transfers) — NttManager registry + multi-transceiver chain catalog (Wormhole / Axelar / LayerZero / custom) |
| `chainlink_feed` | (Internal node-side) Chainlink AggregatorV3 reader with 30s in-memory cache, per-feed staleness threshold, cross-feed rate derivation. Used by the bridge fee oracle when operator enables `chainlink_feeds` in node config. |
| `circuit_breaker` | Provider health management |
| `nanopayment` | Micropayment channels |
| `erc7802` | Cross-chain token mint/burn |
| `svm_cross_vm` | Tenzro Cross-VM SVM-native program: program ID + 4 instruction encoders (`bridge_to_evm`, `bridge_from_evm`, `register_token_pointer`, `transfer_cross_vm`) |
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
   configured, the SDK forwards the bearer + DPoP proof; the node looks up
   the live nonce and gas price, assembles, hashes, signs against the MPC
   wallet bound to the JWT, verifies, and submits — all in one call. `nonce`,
   `chain_id`, and `gas_price` are optional; `value` accepts the alias
   `amount` for parity with the desktop and CLI clients. Self-sends
   (`from == to`) return a `cannot transfer to self` validation error.

   ```rust
   let tx_hash: String = client
       .rpc()
       .call("tenzro_signAndSendTransaction", serde_json::json!([{
           "from": "0x...",
           "to": "0x...",
           "value": "0x..."
           // nonce, chain_id, gas_price all optional — looked up live
       }]))
       .await?;
   ```

2. **Pre-signed submission.** For holders who want to control signing
   client-side, call `eth_sendRawTransaction` directly with `signature`,
   `public_key`, and explicit `timestamp` matching a client-computed
   `Transaction::hash()`. The signer must still match a holder identity
   visible to the node.

## Wallet model

`client.wallet().create_wallet()` provisions a chain-agnostic 2-of-3 Ed25519
MPC wallet. Tenzro wallets are not per-chain — a single wallet projects into
EVM, SVM, and Canton via the pointer-token model, so there is no `chain`
parameter. VM-specific operations are exposed through `client.token()`
(`cross_vm_transfer`, `wrap_tnzo`); transfers to external chains use
`client.bridge()` (LayerZero V2, Chainlink CCIP), `client.debridge()`,
`client.wormhole()`, or `client.lifi()`.

`client.get_transaction(hash)` resolves from finalized storage first, then
falls back to the consensus mempool — `status` is `"pending"` while the
transaction is in-mempool and `"finalized"` once block-included, so callers
polling immediately after broadcast can distinguish "not yet finalized" from
"unknown hash" (the call returns `null` only when the hash is unknown to
both storage and mempool).

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
cargo run --example auth_session
cargo run --example cortex_reasoning
cargo run --example agent_kit_inference
cargo run --example agent_kit_mpp
cargo run --example agent_kit_yield
cargo run --example agent_kit_bridge
cargo run --example agent_kit_canton
cargo run --example defi_solana_swap
cargo run --example defi_base_yield
cargo run --example defi_canton_dvp
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
