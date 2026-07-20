//! Application registry + non-custodial settlement authorization.
//!
//! A developer building a fiat-priced product on Tenzro registers an *app*
//! on-chain (permissionless — any DID may register by signing with a key it
//! controls), funds the app's own TNZO wallet, and settles usage against it.
//! Tenzro never holds custody of the developer's payment-provider secrets or
//! their funds: the developer charges fiat on their own PSP, signs a
//! [`SettlementAuthorization`] with a key registered in the on-chain
//! [`AppRecord`], and any node executes the TNZO movement.
//!
//! # Flow
//!
//! ```text
//! 1. register_app        — developer registers app_id + signing keys + margin, on-chain
//! 2. (fund app_wallet)   — developer moves their own TNZO into the app wallet
//! 3. developer backend   — charges the end user fiat on the developer's own PSP
//! 4. settle_authorized   — a signed authorization moves TNZO app_wallet -> payer,
//!                          commission -> treasury; idempotent on (app_id, external_ref)
//! ```
//!
//! # Signing
//!
//! Two paths are offered for every mutating call:
//!
//! - **Pre-signed forwarding** (default, most non-custodial): the developer's
//!   backend produces the signature bytes and the DID-envelope header value
//!   itself, and passes them in. The SDK never touches a secret. Use
//!   [`AppClient::register_app_presigned`], [`AppClient::set_app_status_presigned`],
//!   and [`AppClient::settle_authorized_presigned`].
//! - **Local-signer convenience**: for the registry writes the developer
//!   supplies an [`Arc<dyn EnvelopeSigner>`](EnvelopeSigner) (an Ed25519 key)
//!   plus its `did:key` identifier; the SDK builds the canonical preimage and
//!   asks the signer to sign it directly (the node verifies Ed25519 over the
//!   raw preimage). For settlement it supplies an
//!   [`Arc<dyn Signer>`](crate::signer::Signer) — the input there is the
//!   32-byte [`SettlementAuthorization::signing_hash`]. Use
//!   [`AppClient::register_app`], [`AppClient::set_app_status`], and
//!   [`AppClient::settle_authorized`].
//!
//! The canonical byte encodings below are reproduced from the node's
//! `app_registry`, `tenzro_types::settlement`, and `tenzro_identity::envelope`
//! modules. The SDK is workspace-isolated (empty `[workspace]`, standalone git
//! mirror) so it cannot depend on those crates; the encodings are kept
//! byte-identical here and covered by round-trip tests.
//!
//! # Example
//!
//! ```no_run
//! # use tenzro_sdk::app::{AppClient, AppSigningKeySpec};
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let app = AppClient::new("https://rpc.tenzro.xyz").await?;
//!
//! // Register the app with a pre-built envelope header (developer signed it
//! // in their own backend).
//! let record = app
//!     .register_app_presigned(
//!         "my-app",
//!         "did:tenzro:human:...",
//!         "0x00..00",                       // app wallet address
//!         vec![AppSigningKeySpec {
//!             key_id: "backend-1".into(),
//!             public_key: vec![0u8; 32],    // Ed25519 verifying key
//!             daily_limit_tnzo: None,
//!         }],
//!         500,                              // margin_bps (5%)
//!         0,                                // min_balance
//!         true,                             // active
//!         "<signed-envelope-header>",
//!     )
//!     .await?;
//! println!("registered {}", record.app_id);
//! # Ok(())
//! # }
//! ```

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use crate::signer::{SignContext, Signer, SignerError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Canonical domain tags (byte-identical to the node + types crates)
// ---------------------------------------------------------------------------

const APP_REGISTRATION_DOMAIN: &[u8] = b"tenzro/app/registration";
const APP_STATUS_DOMAIN: &[u8] = b"tenzro/app/status";
const SETTLEMENT_AUTHORIZATION_DOMAIN: &[u8] = b"tenzro/settlement/authorization";
const ENVELOPE_DOMAIN_V1: &[u8] = b"tenzro-did-envelope:v1";

const METHOD_REGISTER_APP: &str = "tenzro_registerApp";
const METHOD_SET_APP_STATUS: &str = "tenzro_setAppStatus";

/// Protocol ceiling on the developer's per-settlement margin. A margin above
/// this is rejected on-chain; the SDK validates it early so callers get an
/// error before spending a round-trip.
pub const MAX_DEVELOPER_MARGIN_BPS: u32 = 2000;

// ---------------------------------------------------------------------------
// Canonical byte helpers
// ---------------------------------------------------------------------------

/// Length-prefixed byte push: `u32` big-endian length, then the bytes. Matches
/// the node's `push_bytes` used throughout `canonical_params`.
pub(crate) fn push_bytes(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    buf.extend_from_slice(bytes);
}

/// SHA-256 of `bytes` as a 32-byte array. Matches `tenzro_identity::envelope::params_hash`.
pub fn params_hash(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}

/// Parse a 0x-optional hex string into bytes.
fn parse_hex(s: &str) -> SdkResult<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(s).map_err(|e| SdkError::InvalidParameter(format!("invalid hex: {e}")))
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// One signing key attached to an app record. The developer's backend holds the
/// secret; only the 32-byte Ed25519 verifying key is on-chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSigningKeySpec {
    /// Stable identifier for this key inside the app (1..=64 bytes).
    pub key_id: String,
    /// Ed25519 verifying key (exactly 32 bytes).
    pub public_key: Vec<u8>,
    /// Optional per-key daily settlement ceiling, in TNZO base units.
    pub daily_limit_tnzo: Option<u128>,
}

/// An on-chain app record as returned by the node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRecord {
    /// Developer-chosen app identifier (first-writer-wins).
    pub app_id: String,
    /// DID of the registering developer; only this DID may update the record.
    pub developer_did: String,
    /// Hex-encoded address of the app's own TNZO wallet.
    pub app_wallet: String,
    /// Signing keys authorized to settle against this app.
    #[serde(default)]
    pub signing_pubkeys: Vec<AppSigningKeyView>,
    /// Developer's per-settlement margin in basis points (<= [`MAX_DEVELOPER_MARGIN_BPS`]).
    pub margin_bps: u32,
    /// Minimum app-wallet balance to keep in reserve, in TNZO base units.
    #[serde(default)]
    pub min_balance: u128,
    /// Server-set unix-ms creation timestamp, preserved across updates.
    #[serde(default)]
    pub created_at: u64,
    /// Whether the app is active (settlements refused when false).
    pub active: bool,
}

/// A signing key as echoed back by the node (public key hex-encoded).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSigningKeyView {
    /// Key identifier.
    pub key_id: String,
    /// Hex-encoded Ed25519 verifying key.
    pub public_key: String,
    /// Optional per-key daily settlement ceiling, in TNZO base units.
    #[serde(default)]
    pub daily_limit_tnzo: Option<u128>,
}

/// Outcome of a `tenzro_settleAuthorized` call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettleOutcome {
    /// App the settlement was billed to.
    pub app_id: String,
    /// Developer-supplied external reference (idempotency key).
    pub external_ref: String,
    /// Payer DID credited.
    pub payer_did: String,
    /// Hex-encoded payer wallet that received the net TNZO.
    #[serde(default)]
    pub payer_wallet: String,
    /// Gross TNZO moved from the app wallet, in base units.
    #[serde(default)]
    pub amount_tnzo: u128,
    /// Net TNZO credited to the payer after commission.
    #[serde(default)]
    pub payer_net_tnzo: u128,
    /// Commission routed to the treasury.
    #[serde(default)]
    pub commission_tnzo: u128,
    /// Signing key that authorized this settlement.
    #[serde(default)]
    pub key_id: String,
    /// Server-set unix-ms settlement timestamp.
    #[serde(default)]
    pub settled_at: u64,
    /// Whether the settlement succeeded.
    pub success: bool,
    /// Failure reason when `success` is false.
    #[serde(default)]
    pub failure_reason: Option<String>,
    /// Whether the app wallet had sufficient funds.
    #[serde(default)]
    pub app_wallet_funded: bool,
    /// True when this call replayed an already-recorded (app_id, external_ref).
    #[serde(default)]
    pub duplicate: bool,
}

/// A settlement authorization the developer's backend signs to move TNZO from
/// the app wallet to a payer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementAuthorization {
    /// App the settlement bills against.
    pub app_id: String,
    /// Chain the app wallet lives on.
    pub chain_id: u64,
    /// DID of the payer being credited.
    pub payer_did: String,
    /// Gross TNZO to move, in base units.
    pub amount_tnzo: u128,
    /// Idempotency key — typically the PSP charge id (e.g. a Stripe `pi_...`).
    pub external_ref: String,
    /// 32-byte anti-replay nonce.
    pub nonce: [u8; 32],
    /// Expiry as unix-ms; the node rejects authorizations past this.
    pub expiry: u64,
    /// Identifier of the signing key inside the app record.
    pub key_id: String,
}

impl SettlementAuthorization {
    /// Canonical signing preimage, byte-identical to
    /// `tenzro_types::settlement::SettlementAuthorization::signing_preimage`.
    /// The signature field is excluded (it is the output).
    pub fn signing_preimage(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(SETTLEMENT_AUTHORIZATION_DOMAIN);
        push_bytes(&mut out, self.app_id.as_bytes());
        out.extend_from_slice(&self.chain_id.to_be_bytes());
        push_bytes(&mut out, self.payer_did.as_bytes());
        out.extend_from_slice(&self.amount_tnzo.to_be_bytes());
        push_bytes(&mut out, self.external_ref.as_bytes());
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.expiry.to_be_bytes());
        push_bytes(&mut out, self.key_id.as_bytes());
        out
    }

    /// SHA-256 of the signing preimage — the 32-byte hash the developer's key signs.
    pub fn signing_hash(&self) -> [u8; 32] {
        params_hash(&self.signing_preimage())
    }
}

/// The DID envelope carried on mutating app-registry calls so the node can
/// verify the caller controls the developer DID.
#[derive(Debug, Clone)]
pub struct DidEnvelope {
    /// DID that signed the envelope.
    pub did: String,
    /// RPC method the envelope authorizes (e.g. `tenzro_registerApp`).
    pub method: String,
    /// SHA-256 of the method's canonical params.
    pub params_hash: [u8; 32],
    /// Unix-ms timestamp; the node rejects skew beyond ±60s.
    pub timestamp: u64,
    /// 16-byte anti-replay nonce (must not be all-zero).
    pub nonce: [u8; 16],
    /// Signature over [`Self::canonical_preimage`].
    pub signature: Vec<u8>,
}

impl DidEnvelope {
    /// Canonical preimage the DID signs, byte-identical to
    /// `tenzro_identity::envelope::canonical_preimage`.
    pub fn canonical_preimage(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(ENVELOPE_DOMAIN_V1);
        out.extend_from_slice(&(self.did.len() as u32).to_be_bytes());
        out.extend_from_slice(self.did.as_bytes());
        out.extend_from_slice(&(self.method.len() as u32).to_be_bytes());
        out.extend_from_slice(self.method.as_bytes());
        out.extend_from_slice(&self.params_hash);
        out.extend_from_slice(&self.timestamp.to_be_bytes());
        out.extend_from_slice(&self.nonce);
        out
    }

    /// Hex header value the node parses, byte-identical to
    /// `tenzro_identity::envelope::to_header_value`. Excludes the domain tag
    /// (which the verifier re-derives) but includes the signature.
    pub fn to_header_value(&self) -> String {
        let mut out = Vec::new();
        out.extend_from_slice(&(self.did.len() as u32).to_be_bytes());
        out.extend_from_slice(self.did.as_bytes());
        out.extend_from_slice(&(self.method.len() as u32).to_be_bytes());
        out.extend_from_slice(self.method.as_bytes());
        out.extend_from_slice(&self.params_hash);
        out.extend_from_slice(&self.timestamp.to_be_bytes());
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&(self.signature.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.signature);
        hex::encode(out)
    }
}

/// Canonical registration params, byte-identical to the node's
/// `AppRecord::canonical_params`. `created_at` is excluded (server-set).
pub fn app_registration_params(
    app_id: &str,
    developer_did: &str,
    app_wallet: &[u8],
    signing_pubkeys: &[AppSigningKeySpec],
    margin_bps: u32,
    min_balance: u128,
    active: bool,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(APP_REGISTRATION_DOMAIN);
    push_bytes(&mut out, app_id.as_bytes());
    push_bytes(&mut out, developer_did.as_bytes());
    push_bytes(&mut out, app_wallet);
    out.extend_from_slice(&(signing_pubkeys.len() as u32).to_be_bytes());
    for k in signing_pubkeys {
        push_bytes(&mut out, k.key_id.as_bytes());
        push_bytes(&mut out, &k.public_key);
        match k.daily_limit_tnzo {
            Some(l) => {
                out.push(1u8);
                out.extend_from_slice(&l.to_be_bytes());
            }
            None => out.push(0u8),
        }
    }
    out.extend_from_slice(&margin_bps.to_be_bytes());
    out.extend_from_slice(&min_balance.to_be_bytes());
    out.push(active as u8);
    out
}

/// Canonical set-status params, byte-identical to the node's
/// `canonical_status_params`.
pub fn app_status_params(app_id: &str, active: bool) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(APP_STATUS_DOMAIN);
    push_bytes(&mut out, app_id.as_bytes());
    out.push(active as u8);
    out
}

// ---------------------------------------------------------------------------
// Envelope signer
// ---------------------------------------------------------------------------

/// Signs the DID-envelope [`canonical_preimage`](DidEnvelope::canonical_preimage)
/// — the variable-length, domain-separated bytes the node verifies an Ed25519
/// signature over.
///
/// This is a distinct seam from [`crate::signer::Signer`] on purpose. The
/// generic `Signer` is a **32-byte-hash** signer (ERC-7579 user ops, settlement
/// authorizations — the input genuinely is a digest). The DID envelope, by
/// contrast, is Ed25519 over the *raw* preimage: the node's verifier does
/// `verify(pk, canonical_preimage(env), sig)`, and Ed25519 hashes the message
/// internally (SHA-512), so pre-hashing here would produce a signature the node
/// rejects. An implementation therefore receives the full preimage and signs it
/// directly with the developer's Ed25519 key.
#[async_trait]
pub trait EnvelopeSigner: Send + Sync {
    /// Sign `preimage` with the developer's Ed25519 key and return the raw
    /// 64-byte signature. The developer's key never leaves the implementation.
    async fn sign_preimage(&self, preimage: &[u8]) -> Result<Vec<u8>, SignerError>;
}

// ---------------------------------------------------------------------------
// Envelope construction (local signer path)
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn random_bytes<const N: usize>() -> SdkResult<[u8; N]> {
    let mut b = [0u8; N];
    getrandom::getrandom(&mut b)
        .map_err(|e| SdkError::InvalidParameter(format!("rng failure: {e}")))?;
    Ok(b)
}

/// Build a signed DID envelope for `method` over `canonical_params`, signing
/// the raw canonical preimage with `signer`. `did` is the signer's DID (e.g. a
/// `did:key`); the node re-derives the verifying key from it for `did:key`.
///
/// The signature covers the raw preimage (not its SHA-256): the node's verifier
/// does `verify(pk, canonical_preimage(env), sig)`.
pub(crate) async fn build_envelope(
    signer: &Arc<dyn EnvelopeSigner>,
    did: &str,
    method: &str,
    canonical_params: &[u8],
) -> SdkResult<DidEnvelope> {
    let ph = params_hash(canonical_params);
    let nonce: [u8; 16] = random_bytes()?;
    if nonce == [0u8; 16] {
        // Astronomically unlikely, but the node rejects an all-zero nonce.
        return Err(SdkError::InvalidParameter("rng produced zero nonce".into()));
    }
    let mut env = DidEnvelope {
        did: did.to_string(),
        method: method.to_string(),
        params_hash: ph,
        timestamp: now_ms(),
        nonce,
        signature: Vec::new(),
    };
    let preimage = env.canonical_preimage();
    env.signature = signer
        .sign_preimage(&preimage)
        .await
        .map_err(SdkError::from)?;
    Ok(env)
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client for the on-chain app registry + non-custodial settlement surface.
#[derive(Clone)]
pub struct AppClient {
    rpc: Arc<RpcClient>,
}

impl AppClient {
    /// Connect to a node's JSON-RPC endpoint (30s request timeout).
    pub async fn new(rpc_url: &str) -> SdkResult<Self> {
        let rpc = RpcClient::new(rpc_url, std::time::Duration::from_secs(30))?;
        Ok(Self {
            rpc: Arc::new(rpc),
        })
    }

    /// Build from an existing shared RPC client (used by `TenzroClient::app`).
    pub fn from_rpc(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    // ---- register_app ----------------------------------------------------

    /// Register (or update) an app with a pre-built DID-envelope header value.
    /// The developer's backend signs the envelope; the SDK never sees a secret.
    #[allow(clippy::too_many_arguments)]
    pub async fn register_app_presigned(
        &self,
        app_id: &str,
        developer_did: &str,
        app_wallet: &str,
        signing_pubkeys: Vec<AppSigningKeySpec>,
        margin_bps: u32,
        min_balance: u128,
        active: bool,
        envelope_header: &str,
    ) -> SdkResult<AppRecord> {
        self.validate_register(app_id, developer_did, &signing_pubkeys, margin_bps)?;
        let params = json!({
            "app_id": app_id,
            "developer_did": developer_did,
            "app_wallet": app_wallet,
            "signing_pubkeys": signing_pubkeys
                .iter()
                .map(|k| json!({
                    "key_id": k.key_id,
                    "public_key": hex::encode(&k.public_key),
                    "daily_limit_tnzo": k.daily_limit_tnzo.map(|l| l.to_string()),
                }))
                .collect::<Vec<_>>(),
            "margin_bps": margin_bps,
            "min_balance": min_balance.to_string(),
            "active": active,
            "envelope": envelope_header,
        });
        self.rpc.call("tenzro_registerApp", params).await
    }

    /// Register (or update) an app, signing the DID envelope locally with
    /// `signer`. `developer_did` is the DID that owns the app (e.g. the
    /// signer's `did:key`).
    #[allow(clippy::too_many_arguments)]
    pub async fn register_app(
        &self,
        signer: &Arc<dyn EnvelopeSigner>,
        app_id: &str,
        developer_did: &str,
        app_wallet: &str,
        signing_pubkeys: Vec<AppSigningKeySpec>,
        margin_bps: u32,
        min_balance: u128,
        active: bool,
    ) -> SdkResult<AppRecord> {
        self.validate_register(app_id, developer_did, &signing_pubkeys, margin_bps)?;
        let wallet_bytes = parse_hex(app_wallet)?;
        let params = app_registration_params(
            app_id,
            developer_did,
            &wallet_bytes,
            &signing_pubkeys,
            margin_bps,
            min_balance,
            active,
        );
        let env = build_envelope(signer, developer_did, METHOD_REGISTER_APP, &params).await?;
        self.register_app_presigned(
            app_id,
            developer_did,
            app_wallet,
            signing_pubkeys,
            margin_bps,
            min_balance,
            active,
            &env.to_header_value(),
        )
        .await
    }

    fn validate_register(
        &self,
        app_id: &str,
        developer_did: &str,
        signing_pubkeys: &[AppSigningKeySpec],
        margin_bps: u32,
    ) -> SdkResult<()> {
        if app_id.is_empty() || app_id.len() > 128 {
            return Err(SdkError::InvalidParameter(
                "app_id must be 1..=128 bytes".into(),
            ));
        }
        if developer_did.is_empty() {
            return Err(SdkError::InvalidParameter("developer_did is required".into()));
        }
        if signing_pubkeys.is_empty() {
            return Err(SdkError::InvalidParameter(
                "at least one signing key is required".into(),
            ));
        }
        for k in signing_pubkeys {
            if k.key_id.is_empty() || k.key_id.len() > 64 {
                return Err(SdkError::InvalidParameter(
                    "key_id must be 1..=64 bytes".into(),
                ));
            }
            if k.public_key.len() != 32 {
                return Err(SdkError::InvalidParameter(
                    "public_key must be exactly 32 bytes".into(),
                ));
            }
        }
        if margin_bps > MAX_DEVELOPER_MARGIN_BPS {
            return Err(SdkError::InvalidParameter(format!(
                "margin_bps {margin_bps} exceeds max {MAX_DEVELOPER_MARGIN_BPS}"
            )));
        }
        Ok(())
    }

    // ---- set_app_status --------------------------------------------------

    /// Activate or deactivate an app with a pre-built DID-envelope header value.
    pub async fn set_app_status_presigned(
        &self,
        app_id: &str,
        active: bool,
        envelope_header: &str,
    ) -> SdkResult<AppRecord> {
        let params = json!({
            "app_id": app_id,
            "active": active,
            "envelope": envelope_header,
        });
        self.rpc.call("tenzro_setAppStatus", params).await
    }

    /// Activate or deactivate an app, signing the DID envelope locally.
    pub async fn set_app_status(
        &self,
        signer: &Arc<dyn EnvelopeSigner>,
        developer_did: &str,
        app_id: &str,
        active: bool,
    ) -> SdkResult<AppRecord> {
        let params = app_status_params(app_id, active);
        let env = build_envelope(signer, developer_did, METHOD_SET_APP_STATUS, &params).await?;
        self.set_app_status_presigned(app_id, active, &env.to_header_value())
            .await
    }

    // ---- reads -----------------------------------------------------------

    /// Fetch an app record by id.
    pub async fn get_app(&self, app_id: &str) -> SdkResult<AppRecord> {
        self.rpc
            .call("tenzro_getApp", json!({ "app_id": app_id }))
            .await
    }

    /// List all registered apps.
    pub async fn list_apps(&self) -> SdkResult<Vec<AppRecord>> {
        let v: Value = self.rpc.call("tenzro_listApps", json!({})).await?;
        let apps = v
            .get("apps")
            .cloned()
            .unwrap_or(Value::Array(Vec::new()));
        serde_json::from_value(apps).map_err(|_| SdkError::SerializationError)
    }

    // ---- settle_authorized ----------------------------------------------

    /// Execute a settlement from a pre-built authorization signature (hex). The
    /// developer's backend signs [`SettlementAuthorization::signing_hash`] with
    /// the app key `auth.key_id`; the SDK never sees the secret. Idempotent on
    /// `(app_id, external_ref)`.
    pub async fn settle_authorized_presigned(
        &self,
        auth: &SettlementAuthorization,
        signature_hex: &str,
    ) -> SdkResult<SettleOutcome> {
        let params = json!({
            "app_id": auth.app_id,
            "chain_id": auth.chain_id,
            "payer_did": auth.payer_did,
            "amount_tnzo": auth.amount_tnzo.to_string(),
            "external_ref": auth.external_ref,
            "nonce": hex::encode(auth.nonce),
            "expiry": auth.expiry,
            "key_id": auth.key_id,
            "signature": signature_hex,
        });
        self.rpc.call("tenzro_settleAuthorized", params).await
    }

    /// Execute a settlement, signing the authorization locally with `signer`
    /// (must correspond to the app key `auth.key_id`). Idempotent on
    /// `(app_id, external_ref)`.
    pub async fn settle_authorized(
        &self,
        signer: &Arc<dyn Signer>,
        auth: &SettlementAuthorization,
    ) -> SdkResult<SettleOutcome> {
        let hash = auth.signing_hash();
        let sig = signer
            .sign(hash, &SignContext::default())
            .await
            .map_err(SdkError::from)?;
        self.settle_authorized_presigned(auth, &hex::encode(sig.bytes))
            .await
    }

    /// Look up a prior settlement outcome by `(app_id, external_ref)`.
    pub async fn get_settle_authorized_outcome(
        &self,
        app_id: &str,
        external_ref: &str,
    ) -> SdkResult<SettleOutcome> {
        self.rpc
            .call(
                "tenzro_getSettleAuthorizedOutcome",
                json!({ "app_id": app_id, "external_ref": external_ref }),
            )
            .await
    }
}

// ---------------------------------------------------------------------------
// did:key helper
// ---------------------------------------------------------------------------

/// Build a `did:key` identifier from a 32-byte Ed25519 verifying key. The node
/// verifies `did:key` envelopes without a registry lookup by re-deriving the
/// key from this DID (multicodec `0xed 0x01` + 32-byte key, base58btc after
/// `z`). This is `bs58` base58btc; the SDK ships a minimal encoder so callers
/// need no extra dependency.
pub fn did_key_from_ed25519(verifying_key: &[u8; 32]) -> String {
    let mut mc = Vec::with_capacity(34);
    mc.push(0xed);
    mc.push(0x01);
    mc.extend_from_slice(verifying_key);
    format!("did:key:z{}", base58btc_encode(&mc))
}

/// Minimal Bitcoin-alphabet base58 encoder (no checksum), matching the
/// `bs58` crate's default alphabet used by `did:key`.
fn base58btc_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 58] =
        b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    // Count leading zero bytes -> leading '1's.
    let zeros = input.iter().take_while(|&&b| b == 0).count();
    let mut digits: Vec<u8> = Vec::new();
    for &byte in input {
        let mut carry = byte as u32;
        for d in digits.iter_mut() {
            carry += (*d as u32) << 8;
            *d = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }
    let mut out = String::with_capacity(zeros + digits.len());
    for _ in 0..zeros {
        out.push('1');
    }
    for &d in digits.iter().rev() {
        out.push(ALPHABET[d as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settlement_signing_preimage_is_domain_prefixed() {
        let auth = SettlementAuthorization {
            app_id: "demo-app".into(),
            chain_id: 1337,
            payer_did: "did:tenzro:human:abc".into(),
            amount_tnzo: 1_000_000_000_000_000_000,
            external_ref: "pi_3Nqw8s".into(),
            nonce: [7u8; 32],
            expiry: 1_800_000_000_000,
            key_id: "backend-1".into(),
        };
        let pre = auth.signing_preimage();
        assert!(pre.starts_with(SETTLEMENT_AUTHORIZATION_DOMAIN));
        // Deterministic.
        assert_eq!(pre, auth.signing_preimage());
        // Hash is 32 bytes and stable.
        assert_eq!(auth.signing_hash(), auth.signing_hash());
    }

    #[test]
    fn settlement_hash_changes_when_any_field_changes() {
        let base = SettlementAuthorization {
            app_id: "a".into(),
            chain_id: 1,
            payer_did: "did:x".into(),
            amount_tnzo: 10,
            external_ref: "ref".into(),
            nonce: [1u8; 32],
            expiry: 100,
            key_id: "k".into(),
        };
        let h = base.signing_hash();

        let mut m = base.clone();
        m.chain_id = 2;
        assert_ne!(h, m.signing_hash());

        let mut m = base.clone();
        m.amount_tnzo = 11;
        assert_ne!(h, m.signing_hash());

        let mut m = base.clone();
        m.nonce = [2u8; 32];
        assert_ne!(h, m.signing_hash());

        let mut m = base.clone();
        m.external_ref = "ref2".into();
        assert_ne!(h, m.signing_hash());
    }

    #[test]
    fn registration_params_are_domain_prefixed_and_exclude_created_at() {
        let keys = vec![AppSigningKeySpec {
            key_id: "backend-1".into(),
            public_key: vec![9u8; 32],
            daily_limit_tnzo: Some(500),
        }];
        let p = app_registration_params(
            "my-app",
            "did:tenzro:human:abc",
            &[0u8; 20],
            &keys,
            500,
            0,
            true,
        );
        assert!(p.starts_with(APP_REGISTRATION_DOMAIN));
        // Daily limit tag `1` then 16 big-endian bytes are present.
        // margin_bps 500 = 0x01F4 big-endian appears near the tail.
        assert!(p.windows(4).any(|w| w == 500u32.to_be_bytes()));
    }

    #[test]
    fn status_params_are_domain_prefixed() {
        let p = app_status_params("my-app", false);
        assert!(p.starts_with(APP_STATUS_DOMAIN));
        assert_eq!(*p.last().unwrap(), 0u8); // active=false
        let p = app_status_params("my-app", true);
        assert_eq!(*p.last().unwrap(), 1u8);
    }

    #[test]
    fn envelope_preimage_and_header_are_domain_correct() {
        let env = DidEnvelope {
            did: "did:key:zABC".into(),
            method: "tenzro_registerApp".into(),
            params_hash: [3u8; 32],
            timestamp: 1_700_000_000_000,
            nonce: [4u8; 16],
            signature: vec![5u8; 64],
        };
        let pre = env.canonical_preimage();
        assert!(pre.starts_with(ENVELOPE_DOMAIN_V1));
        // Header value excludes the domain tag but includes the signature.
        let header = env.to_header_value();
        let decoded = hex::decode(&header).unwrap();
        assert!(!decoded.starts_with(ENVELOPE_DOMAIN_V1));
        // Ends with the 64-byte signature preceded by its u32 length.
        assert!(decoded.ends_with(&[5u8; 64]));
    }

    #[test]
    fn did_key_round_trips_multicodec_prefix() {
        let vk = [1u8; 32];
        let did = did_key_from_ed25519(&vk);
        assert!(did.starts_with("did:key:z"));
        // Decode the base58 body and check the multicodec prefix.
        // (Re-decode via a tiny reference decoder for the test only.)
        let body = &did["did:key:z".len()..];
        let decoded = base58btc_decode(body);
        assert_eq!(&decoded[..2], &[0xed, 0x01]);
        assert_eq!(&decoded[2..], &vk);
    }

    /// Test-only base58 decoder mirroring `base58btc_encode`.
    fn base58btc_decode(input: &str) -> Vec<u8> {
        const ALPHABET: &[u8; 58] =
            b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        let mut map = [255u8; 128];
        for (i, &c) in ALPHABET.iter().enumerate() {
            map[c as usize] = i as u8;
        }
        let zeros = input.bytes().take_while(|&b| b == b'1').count();
        let mut bytes: Vec<u8> = Vec::new();
        for c in input.bytes() {
            let mut carry = map[c as usize] as u32;
            for b in bytes.iter_mut() {
                carry += (*b as u32) * 58;
                *b = (carry & 0xff) as u8;
                carry >>= 8;
            }
            while carry > 0 {
                bytes.push((carry & 0xff) as u8);
                carry >>= 8;
            }
        }
        let mut out = vec![0u8; zeros];
        out.extend(bytes.iter().rev());
        out
    }

    #[test]
    fn validate_register_rejects_bad_inputs() {
        let client_keys = vec![AppSigningKeySpec {
            key_id: "k".into(),
            public_key: vec![0u8; 32],
            daily_limit_tnzo: None,
        }];
        // Build a throwaway client without a live connection — validation is
        // pure and does not touch the RPC.
        let rpc =
            Arc::new(RpcClient::new("http://127.0.0.1:1", std::time::Duration::from_secs(1)).unwrap());
        let c = AppClient::from_rpc(rpc);

        assert!(c.validate_register("", "did:x", &client_keys, 500).is_err());
        assert!(c.validate_register("app", "", &client_keys, 500).is_err());
        assert!(c.validate_register("app", "did:x", &[], 500).is_err());
        assert!(
            c.validate_register("app", "did:x", &client_keys, MAX_DEVELOPER_MARGIN_BPS + 1)
                .is_err()
        );
        assert!(
            c.validate_register("app", "did:x", &client_keys, 500)
                .is_ok()
        );

        let bad_key = vec![AppSigningKeySpec {
            key_id: "k".into(),
            public_key: vec![0u8; 31], // wrong length
            daily_limit_tnzo: None,
        }];
        assert!(c.validate_register("app", "did:x", &bad_key, 500).is_err());
    }

    /// The `EnvelopeSigner` path must sign the *raw* canonical preimage, not its
    /// SHA-256 — the node verifies Ed25519 over those exact bytes. This test
    /// signs with a real Ed25519 key (dev-dep), then verifies the signature over
    /// the raw preimage the way the node does, and confirms a signature over
    /// `SHA-256(preimage)` would NOT verify (guarding the prior bug).
    #[tokio::test]
    async fn envelope_signer_signs_raw_preimage() {
        use tenzro_crypto::signatures::{verify, Signer as CryptoSigner};
        use tenzro_crypto::{Ed25519SignerImpl, KeyPair, KeyType, PublicKey, Signature as CryptoSig};

        struct RealEd25519(Ed25519SignerImpl);
        #[async_trait]
        impl EnvelopeSigner for RealEd25519 {
            async fn sign_preimage(&self, preimage: &[u8]) -> Result<Vec<u8>, SignerError> {
                Ok(self
                    .0
                    .sign(preimage)
                    .map_err(|e| SignerError::BackendUnavailable(e.to_string()))?
                    .as_bytes()
                    .to_vec())
            }
        }

        let kp = KeyPair::generate(KeyType::Ed25519).unwrap();
        let vk: [u8; 32] = kp.public_key().as_bytes().try_into().unwrap();
        let did = did_key_from_ed25519(&vk);
        let signer: Arc<dyn EnvelopeSigner> = Arc::new(RealEd25519(Ed25519SignerImpl::new(kp).unwrap()));

        let params = app_status_params("demo", true);
        let env = build_envelope(&signer, &did, METHOD_SET_APP_STATUS, &params)
            .await
            .unwrap();

        let preimage = env.canonical_preimage();
        let pk = PublicKey::new(KeyType::Ed25519, vk.to_vec());
        // Node scheme: Ed25519 over the raw preimage — must verify.
        let sig = CryptoSig::new(KeyType::Ed25519, env.signature.clone());
        assert!(verify(&pk, &preimage, &sig).is_ok());
        // A signature over SHA-256(preimage) would be a different message and
        // must NOT verify against the raw preimage.
        assert!(verify(&pk, &params_hash(&preimage), &sig).is_err());
    }
}
