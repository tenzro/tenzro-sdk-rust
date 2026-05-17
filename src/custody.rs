//! Key Custody & Wallet Security SDK for Tenzro Network
//!
//! This module provides MPC threshold wallet management, encrypted keystore
//! import/export, key share rotation, spending limits, and session key
//! authorization.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Client for key custody and wallet security operations
///
/// Provides MPC wallet creation, keystore management, key rotation,
/// spending policies, and session key authorization.
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let custody = client.custody();
///
/// // Create a 2-of-3 MPC wallet
/// let wallet = custody.create_mpc_wallet(2, 3, "ed25519").await?;
/// println!("Wallet: {} ({})", wallet.address, wallet.wallet_id);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct CustodyClient {
    rpc: Arc<RpcClient>,
}

impl CustodyClient {
    /// Creates a new custody client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Creates a new MPC threshold wallet
    ///
    /// # Arguments
    ///
    /// * `threshold` - Minimum number of shares required to sign (e.g., 2)
    /// * `total_shares` - Total number of key shares (e.g., 3)
    /// * `key_type` - Key algorithm: "ed25519" or "secp256k1"
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let custody = client.custody();
    /// let wallet = custody.create_mpc_wallet(2, 3, "ed25519").await?;
    /// println!("Address: {}", wallet.address);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_mpc_wallet(
        &self,
        threshold: u8,
        total_shares: u8,
        key_type: &str,
    ) -> SdkResult<MpcWallet> {
        self.rpc
            .call(
                "tenzro_createMpcWallet",
                serde_json::json!([{
                    "threshold": threshold,
                    "total_shares": total_shares,
                    "key_type": key_type,
                }]),
            )
            .await
    }

    /// Exports an encrypted keystore
    ///
    /// The keystore is encrypted using Argon2id KDF + AES-256-GCM.
    ///
    /// # Arguments
    ///
    /// * `wallet_id` - Wallet identifier
    /// * `password` - Password to encrypt the keystore
    pub async fn export_keystore(
        &self,
        wallet_id: &str,
        password: &str,
    ) -> SdkResult<EncryptedKeystore> {
        self.rpc
            .call(
                "tenzro_exportKeystore",
                serde_json::json!([{
                    "wallet_id": wallet_id,
                    "password": password,
                }]),
            )
            .await
    }

    /// Imports a wallet from an encrypted keystore
    ///
    /// # Arguments
    ///
    /// * `keystore` - Encrypted keystore JSON string
    /// * `password` - Password to decrypt the keystore
    pub async fn import_keystore(
        &self,
        keystore: &str,
        password: &str,
    ) -> SdkResult<MpcWallet> {
        self.rpc
            .call(
                "tenzro_importKeystore",
                serde_json::json!([{
                    "keystore": keystore,
                    "password": password,
                }]),
            )
            .await
    }

    /// Gets key share metadata for a wallet
    ///
    /// Returns metadata about each key share (index, creation time).
    /// Does NOT return the actual key share material.
    ///
    /// # Arguments
    ///
    /// * `wallet_id` - Wallet identifier
    pub async fn get_key_shares(&self, wallet_id: &str) -> SdkResult<Vec<KeyShare>> {
        self.rpc
            .call(
                "tenzro_getKeyShares",
                serde_json::json!([{ "wallet_id": wallet_id }]),
            )
            .await
    }

    /// Rotates MPC key shares
    ///
    /// Generates new key shares while preserving the same public key and address.
    /// Old shares are invalidated.
    ///
    /// # Arguments
    ///
    /// * `wallet_id` - Wallet identifier
    pub async fn rotate_keys(&self, wallet_id: &str) -> SdkResult<RotationResult> {
        self.rpc
            .call(
                "tenzro_rotateKeys",
                serde_json::json!([{ "wallet_id": wallet_id }]),
            )
            .await
    }

    /// Sets spending limits for a wallet
    ///
    /// Configures daily and per-transaction spending limits.
    ///
    /// # Arguments
    ///
    /// * `wallet_id` - Wallet identifier
    /// * `daily_limit` - Maximum daily spending (in smallest unit)
    /// * `per_tx_limit` - Maximum per-transaction spending (in smallest unit)
    pub async fn set_spending_limits(
        &self,
        wallet_id: &str,
        daily_limit: u128,
        per_tx_limit: u128,
    ) -> SdkResult<SpendingPolicy> {
        self.rpc
            .call(
                "tenzro_setSpendingLimits",
                serde_json::json!([{
                    "wallet_id": wallet_id,
                    "daily_limit": daily_limit.to_string(),
                    "per_tx_limit": per_tx_limit.to_string(),
                }]),
            )
            .await
    }

    /// Revokes an active session key
    ///
    /// Immediately invalidates a session key, preventing any further
    /// operations using that session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session key identifier to revoke
    pub async fn revoke_session(&self, session_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_revokeSession",
                serde_json::json!([{ "session_id": session_id }]),
            )
            .await
    }

    /// Gets current spending limits for a wallet
    ///
    /// # Arguments
    ///
    /// * `wallet_id` - Wallet identifier
    pub async fn get_spending_limits(&self, wallet_id: &str) -> SdkResult<SpendingPolicy> {
        self.rpc
            .call(
                "tenzro_getSpendingLimits",
                serde_json::json!([{ "wallet_id": wallet_id }]),
            )
            .await
    }

    /// Creates a session key with scoped permissions
    ///
    /// Session keys allow temporary, limited access to wallet operations
    /// without exposing the master key shares.
    ///
    /// # Arguments
    ///
    /// * `wallet_id` - Wallet identifier
    /// * `duration_secs` - Session validity duration in seconds
    /// * `operations` - Allowed operations (e.g., "transfer", "stake", "governance")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let custody = client.custody();
    /// let session = custody.authorize_session(
    ///     "wallet-123",
    ///     3600, // 1 hour
    ///     vec!["transfer".to_string(), "stake".to_string()],
    /// ).await?;
    /// println!("Session: {} (expires: {})", session.session_id, session.expires_at);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn authorize_session(
        &self,
        wallet_id: &str,
        duration_secs: u64,
        operations: Vec<String>,
    ) -> SdkResult<SessionKey> {
        self.rpc
            .call(
                "tenzro_authorizeSession",
                serde_json::json!([{
                    "wallet_id": wallet_id,
                    "duration_secs": duration_secs,
                    "operations": operations,
                }]),
            )
            .await
    }

    // -----------------------------------------------------------------
    // ML-DSA-65 (FIPS 204) — post-quantum wallet signing surface
    // -----------------------------------------------------------------
    //
    // These methods call the `/wallet/mldsa/*` Web API endpoints (not
    // JSON-RPC). Each call requires a caller-supplied DPoP-bound JWT
    // and a fresh DPoP proof signed over the request's `(method, htu)`.
    // The proof is opaque to the SDK — the wallet kernel constructs it.

    /// Discover the node's ML-DSA-65 signing mode.
    ///
    /// Always `tee-only` on testnet. The wallet uses this to decide
    /// whether to invoke threshold-coordination methods (skipped in
    /// `tee-only`) or fall through to the single-shot `mldsa_sign`.
    ///
    /// Required AAP capability: `wallet.mldsa.sign`.
    pub async fn mldsa_capabilities(
        &self,
        bearer_jwt: &str,
        dpop_proof: &str,
    ) -> SdkResult<MlDsaCapabilities> {
        self.rpc
            .get_with_auth("/wallet/mldsa/capabilities", bearer_jwt, dpop_proof)
            .await
    }

    /// Sign a preimage with the node-held ML-DSA-65 key bound to
    /// `(did, surface_key)`.
    ///
    /// `preimage` is the raw bytes to be signed (no length cap beyond
    /// the Web API's 2 MB request body limit). The returned signature
    /// is 3309 bytes per FIPS 204 §4 Table 2.
    ///
    /// Required AAP capability: `wallet.mldsa.sign`.
    pub async fn mldsa_sign(
        &self,
        bearer_jwt: &str,
        dpop_proof: &str,
        did: &str,
        surface_key: &str,
        preimage: &[u8],
        purpose: Option<&str>,
    ) -> SdkResult<MlDsaSignature> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let body = serde_json::json!({
            "did": did,
            "surface_key": surface_key,
            "preimage_b64": URL_SAFE_NO_PAD.encode(preimage),
            "purpose": purpose,
        });
        self.rpc
            .post_with_auth("/wallet/mldsa/sign", &body, bearer_jwt, dpop_proof)
            .await
    }

    // -----------------------------------------------------------------
    // FROST (RFC 9591) — threshold Schnorr signing surface
    // -----------------------------------------------------------------
    //
    // Per-curve `:scheme` path dispatch (`ed25519` | `secp256k1`). The
    // node holds one share, the wallet holds the other (2-of-2). The
    // wallet drives the protocol round-by-round; the node is purely
    // reactive. Each call is gated by the AAP capability
    // `wallet.frost.sign` and requires a fresh DPoP proof bound to the
    // request `(method, htu)`.
    //
    // Wire bytes (`*_b64` fields) are the FROST crate's canonical
    // `.serialize()` of the corresponding round structure — the wallet
    // and the node are running the same `frost-{ed25519,secp256k1}`
    // version, so they round-trip cleanly.

    /// Start a FROST signing session.
    ///
    /// Server allocates a session, runs Round 1, and returns its
    /// commitments together with both participant identifiers. The
    /// wallet then runs its own Round 1 against the same `preimage`
    /// and the returned identifiers, and submits its commitments via
    /// [`Self::frost_commit`].
    ///
    /// Required AAP capability: `wallet.frost.sign`.
    pub async fn frost_start(
        &self,
        bearer_jwt: &str,
        dpop_proof: &str,
        scheme: FrostScheme,
        did: &str,
        surface_key: &str,
        preimage: &[u8],
    ) -> SdkResult<FrostStartResponse> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let body = serde_json::json!({
            "did": did,
            "surface_key": surface_key,
            "preimage_b64": URL_SAFE_NO_PAD.encode(preimage),
        });
        let path = format!("/wallet/frost/{}/start", scheme.as_str());
        self.rpc
            .post_with_auth(&path, &body, bearer_jwt, dpop_proof)
            .await
    }

    /// Submit the wallet's Round 1 commitments.
    ///
    /// Transitions the session from `pending` to `committed`. After
    /// this call the wallet should call [`Self::frost_await_challenge`]
    /// to receive the `SigningPackage` it must feed into the FROST
    /// crate's `round2::sign`.
    ///
    /// Required AAP capability: `wallet.frost.sign`.
    pub async fn frost_commit(
        &self,
        bearer_jwt: &str,
        dpop_proof: &str,
        scheme: FrostScheme,
        session_id: &str,
        device_commitments: &[u8],
    ) -> SdkResult<FrostStateResponse> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let body = serde_json::json!({
            "session_id": session_id,
            "device_commitments_b64": URL_SAFE_NO_PAD.encode(device_commitments),
        });
        let path = format!("/wallet/frost/{}/commit", scheme.as_str());
        self.rpc
            .post_with_auth(&path, &body, bearer_jwt, dpop_proof)
            .await
    }

    /// Long-poll for the challenge (`SigningPackage`).
    ///
    /// Returns immediately if the session is already `committed`. Polls
    /// for up to ~5s otherwise. The wallet feeds `signing_package_b64`
    /// into `round2::sign(signing_package, signer_nonces, key_package)`
    /// to produce its signature share.
    ///
    /// Required AAP capability: `wallet.frost.sign`.
    pub async fn frost_await_challenge(
        &self,
        bearer_jwt: &str,
        dpop_proof: &str,
        scheme: FrostScheme,
        session_id: &str,
    ) -> SdkResult<FrostChallengeResponse> {
        let body = serde_json::json!({ "session_id": session_id });
        let path = format!("/wallet/frost/{}/await-challenge", scheme.as_str());
        self.rpc
            .post_with_auth(&path, &body, bearer_jwt, dpop_proof)
            .await
    }

    /// Submit the wallet's Round 2 signature share.
    ///
    /// Server runs its own Round 2, aggregates the two shares, and
    /// transitions the session to `finalized`. The aggregated signature
    /// is then retrievable via [`Self::frost_finalize`].
    ///
    /// Required AAP capability: `wallet.frost.sign`.
    pub async fn frost_respond(
        &self,
        bearer_jwt: &str,
        dpop_proof: &str,
        scheme: FrostScheme,
        session_id: &str,
        device_signature_share: &[u8],
    ) -> SdkResult<FrostStateResponse> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let body = serde_json::json!({
            "session_id": session_id,
            "device_signature_share_b64": URL_SAFE_NO_PAD.encode(device_signature_share),
        });
        let path = format!("/wallet/frost/{}/respond", scheme.as_str());
        self.rpc
            .post_with_auth(&path, &body, bearer_jwt, dpop_proof)
            .await
    }

    /// Long-poll for the aggregated signature.
    ///
    /// Returns immediately if the session is already `finalized`. Polls
    /// for up to ~5s otherwise. `signature_b64` is the canonical Schnorr
    /// signature: 64 bytes for Ed25519, 65 for secp256k1 (Taproot).
    ///
    /// Required AAP capability: `wallet.frost.sign`.
    pub async fn frost_finalize(
        &self,
        bearer_jwt: &str,
        dpop_proof: &str,
        scheme: FrostScheme,
        session_id: &str,
    ) -> SdkResult<FrostFinalizeResponse> {
        let body = serde_json::json!({ "session_id": session_id });
        let path = format!("/wallet/frost/{}/finalize", scheme.as_str());
        self.rpc
            .post_with_auth(&path, &body, bearer_jwt, dpop_proof)
            .await
    }

    /// Abort an in-flight FROST session.
    ///
    /// Idempotent: an already-aborted session returns `aborted`. A
    /// session that has already finalized stays `finalized` (the abort
    /// is a no-op rather than an error).
    ///
    /// Required AAP capability: `wallet.frost.sign`.
    pub async fn frost_abort(
        &self,
        bearer_jwt: &str,
        dpop_proof: &str,
        scheme: FrostScheme,
        session_id: &str,
    ) -> SdkResult<FrostStateResponse> {
        let body = serde_json::json!({ "session_id": session_id });
        let path = format!("/wallet/frost/{}/abort", scheme.as_str());
        self.rpc
            .post_with_auth(&path, &body, bearer_jwt, dpop_proof)
            .await
    }

    // -----------------------------------------------------------------
    // Passkey share-unwrap surface (`/wallet/share/*`)
    // -----------------------------------------------------------------
    //
    // Three-step flow:
    //   1. `share_envelope`       — fetch the wrapped FROST share blob.
    //   2. `share_escrow_challenge` — mint a single-use 30s nonce.
    //   3. `share_escrow_unwrap`  — submit the WebAuthn assertion + nonce
    //                                to receive `(wrapped_share, pepper)`.
    //
    // The pepper is mixed into the wallet's local unwrap KDF; without
    // it the wrapped share is gibberish even to a caller that holds a
    // valid AAP token. All endpoints require capability
    // `wallet.share.unwrap`.
    //
    // Wire bytes are base64url no-pad. The `assertion` payload is the
    // raw output of `navigator.credentials.get()` after extraction —
    // the wallet kernel performs the WebAuthn ceremony; the SDK is
    // transport-only.

    /// Fetch the wrapped FROST share for `(credential_id, surface_key)`.
    ///
    /// Idempotent — repeated calls return identical bytes for the same
    /// pair. The returned blob is useless on its own; the wallet must
    /// also obtain the per-assertion pepper via [`Self::share_escrow_unwrap`]
    /// and combine the two through its local KDF to recover the
    /// cleartext share.
    ///
    /// Required AAP capability: `wallet.share.unwrap`.
    pub async fn share_envelope(
        &self,
        bearer_jwt: &str,
        dpop_proof: &str,
        credential_id: &str,
        surface_key: &str,
    ) -> SdkResult<ShareEnvelopeResponse> {
        let path = format!(
            "/wallet/share/envelope?credential_id={}&surface_key={}",
            urlencode(credential_id),
            urlencode(surface_key),
        );
        self.rpc.get_with_auth(&path, bearer_jwt, dpop_proof).await
    }

    /// Mint a single-use, 30-second-TTL nonce for an upcoming WebAuthn
    /// ceremony.
    ///
    /// The wallet must use the returned `nonce_b64` value verbatim as
    /// the WebAuthn `challenge` field when prompting the user's
    /// passkey. Server-side the nonce is held in an in-memory escrow;
    /// it is consumed by [`Self::share_escrow_unwrap`] regardless of
    /// whether the assertion verifies.
    ///
    /// Required AAP capability: `wallet.share.unwrap`.
    pub async fn share_escrow_challenge(
        &self,
        bearer_jwt: &str,
        dpop_proof: &str,
        credential_id: &str,
        surface_key: &str,
    ) -> SdkResult<ShareEscrowChallengeResponse> {
        let body = serde_json::json!({
            "credential_id": credential_id,
            "surface_key": surface_key,
        });
        self.rpc
            .post_with_auth("/wallet/share/escrow/challenge", &body, bearer_jwt, dpop_proof)
            .await
    }

    /// Verify a WebAuthn assertion, consume the escrow nonce, and
    /// return `(wrapped_share, pepper)`.
    ///
    /// Single-use: the nonce is removed from the escrow before the
    /// assertion is verified, so a successful unwrap cannot be replayed
    /// and a failed verification still consumes the nonce. The wallet
    /// must request a fresh challenge before retrying.
    ///
    /// Required AAP capability: `wallet.share.unwrap`.
    pub async fn share_escrow_unwrap(
        &self,
        bearer_jwt: &str,
        dpop_proof: &str,
        credential_id: &str,
        surface_key: &str,
        nonce_b64: &str,
        assertion: PasskeyAssertion,
    ) -> SdkResult<ShareEscrowUnwrapResponse> {
        let body = serde_json::json!({
            "credential_id": credential_id,
            "surface_key": surface_key,
            "nonce_b64": nonce_b64,
            "assertion": assertion,
        });
        self.rpc
            .post_with_auth("/wallet/share/escrow/unwrap", &body, bearer_jwt, dpop_proof)
            .await
    }
}

/// Minimal RFC 3986 unreserved-only encoder for query string values.
/// `credential_id` and `surface_key` are wallet-defined identifiers —
/// in practice base64url, dot-segmented, or simple ASCII slugs — so
/// the percent-encoded set we need to handle is small. Implemented
/// inline so the SDK does not pull in `urlencoding`.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

/// Discovery response from `GET /wallet/mldsa/capabilities`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlDsaCapabilities {
    /// Mode of the ML-DSA-65 signing surface. `"tee-only"` on testnet.
    pub mode: String,
}

/// Sign-response from `POST /wallet/mldsa/sign`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlDsaSignature {
    /// 3309-byte ML-DSA-65 signature, base64url no-pad.
    pub signature_b64: String,
}

/// FROST signing scheme — selects the curve via the `:scheme` path
/// segment of the `/wallet/frost/:scheme/*` endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrostScheme {
    /// Ed25519 — RFC 9591 §6.1, used for Tenzro-native signing.
    Ed25519,
    /// secp256k1 — RFC 9591 §6.2, used for EVM/Bitcoin/Taproot signing.
    Secp256k1,
}

impl FrostScheme {
    /// Path segment used in the URL: `"ed25519"` or `"secp256k1"`.
    pub fn as_str(&self) -> &'static str {
        match self {
            FrostScheme::Ed25519 => "ed25519",
            FrostScheme::Secp256k1 => "secp256k1",
        }
    }
}

/// Session-state discriminator returned by every FROST endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrostSessionState {
    /// Session created; node has run Round 1; waiting for the wallet
    /// to submit its commitments.
    Pending,
    /// Both parties have submitted Round 1 commitments; the
    /// `SigningPackage` is ready and the wallet should run Round 2.
    Committed,
    /// Both signature shares aggregated; signature is available via
    /// `frost_finalize`.
    Finalized,
    /// Session terminated before completion (either explicit abort or
    /// 5-minute TTL expiry).
    Aborted,
}

/// Response from `POST /wallet/frost/:scheme/start`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostStartResponse {
    /// Opaque session identifier — pass back into every subsequent call.
    #[serde(default)]
    pub session_id: String,
    /// Unix-millis at which the session is evicted regardless of state.
    #[serde(default)]
    pub expires_at_ms: u64,
    /// Stable participant identifier the node uses for itself.
    #[serde(default)]
    pub node_identifier_b64: String,
    /// Stable participant identifier the wallet must use for itself
    /// (must match what the node will look up in the `KeyPackage`).
    #[serde(default)]
    pub device_identifier_b64: String,
    /// Node's Round 1 `SigningCommitments`, serialized via the FROST
    /// crate's canonical `.serialize()`.
    #[serde(default)]
    pub node_commitments_b64: String,
}

/// Response from endpoints that only carry a state transition
/// (`commit`, `respond`, `abort`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostStateResponse {
    pub state: FrostSessionState,
}

/// Response from `POST /wallet/frost/:scheme/await-challenge`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostChallengeResponse {
    pub state: FrostSessionState,
    /// Present only when `state == Committed`. Serialized
    /// `SigningPackage` — feed straight into the FROST crate's
    /// `round2::sign(signing_package, signer_nonces, key_package)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signing_package_b64: Option<String>,
}

/// Response from `POST /wallet/frost/:scheme/finalize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostFinalizeResponse {
    pub state: FrostSessionState,
    /// Present only when `state == Finalized`. The aggregated Schnorr
    /// signature: 64 bytes for Ed25519, 65 bytes for secp256k1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_b64: Option<String>,
}

/// MPC threshold wallet information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpcWallet {
    /// Wallet identifier
    #[serde(default)]
    pub wallet_id: String,
    /// Wallet address (hex)
    #[serde(default)]
    pub address: String,
    /// Signing threshold (e.g., 2)
    #[serde(default)]
    pub threshold: u8,
    /// Total number of key shares (e.g., 3)
    #[serde(default)]
    pub total_shares: u8,
}

/// Encrypted keystore export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKeystore {
    /// Encrypted keystore data (JSON string)
    #[serde(default)]
    pub encrypted: String,
    /// Key derivation function used ("argon2id")
    #[serde(default)]
    pub kdf: String,
    /// Cipher used ("aes-256-gcm")
    #[serde(default)]
    pub cipher: String,
}

/// Key share metadata (not the actual share material)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyShare {
    /// Share index (1-based)
    #[serde(default)]
    pub index: u8,
    /// When this share was created
    #[serde(default)]
    pub created_at: String,
}

/// Result of a key rotation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationResult {
    /// Whether the rotation succeeded
    #[serde(default)]
    pub success: bool,
    /// Number of shares rotated
    #[serde(default)]
    pub shares_rotated: u8,
    /// New rotation epoch
    #[serde(default)]
    pub epoch: u64,
}

/// Wallet spending policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingPolicy {
    /// Maximum daily spending (in smallest unit)
    #[serde(default)]
    pub daily_limit: u128,
    /// Maximum per-transaction spending (in smallest unit)
    #[serde(default)]
    pub per_tx_limit: u128,
    /// Amount already spent today (in smallest unit)
    #[serde(default)]
    pub daily_spent: u128,
}

/// Scoped session key for temporary wallet access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionKey {
    /// Session key identifier
    #[serde(default)]
    pub session_id: String,
    /// When the session expires (ISO 8601)
    #[serde(default)]
    pub expires_at: String,
    /// Allowed operations
    #[serde(default)]
    pub operations: Vec<String>,
}

/// WebAuthn assertion submitted to `/wallet/share/escrow/unwrap`.
///
/// All three fields are base64url no-pad. The wallet kernel performs
/// the WebAuthn ceremony (`navigator.credentials.get()`) and forwards
/// the resulting bytes verbatim — the SDK is transport-only and does
/// not parse or validate the assertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyAssertion {
    /// `AuthenticatorAssertionResponse.authenticatorData`.
    pub authenticator_data_b64: String,
    /// `AuthenticatorAssertionResponse.clientDataJSON`. The embedded
    /// `challenge` field must equal the `nonce_b64` returned by
    /// `share_escrow_challenge`.
    pub client_data_json_b64: String,
    /// `AuthenticatorAssertionResponse.signature` — Ed25519 (COSE alg
    /// `-8`) signature over `authenticatorData || SHA-256(clientDataJSON)`
    /// per WebAuthn L3 §7.2.
    pub signature_b64: String,
}

/// Response from `GET /wallet/share/envelope`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareEnvelopeResponse {
    /// Wrapped FROST share — AES-256-GCM ciphertext of the cleartext
    /// share, AAD-bound to `(credential_id, surface_key)`. Base64url
    /// no-pad.
    #[serde(default)]
    pub wrapped_share_b64: String,
    /// Wrap algorithm identifier — `"aes-256-gcm"` on testnet.
    #[serde(default)]
    pub alg: String,
    /// Salt used by the wallet's local unwrap KDF, base64url no-pad.
    #[serde(default)]
    pub salt_b64: String,
}

/// Response from `POST /wallet/share/escrow/challenge`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareEscrowChallengeResponse {
    /// 32-byte random nonce, base64url no-pad. Must be passed verbatim
    /// as the WebAuthn `challenge` field.
    #[serde(default)]
    pub nonce_b64: String,
    /// Unix-millis at which the escrow entry is swept (30s TTL).
    #[serde(default)]
    pub expires_at_ms: u64,
}

/// Response from `POST /wallet/share/escrow/unwrap`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareEscrowUnwrapResponse {
    /// Same wrapped FROST share returned by `share_envelope`.
    /// Returned again here so a wallet can perform challenge → unwrap
    /// in a single round-trip without holding the envelope client-side.
    #[serde(default)]
    pub wrapped_share_b64: String,
    /// Per-assertion entropy mixed into the wallet's local unwrap KDF.
    /// Without this value the wrapped share is gibberish even to a
    /// caller holding a valid AAP token. Base64url no-pad.
    #[serde(default)]
    pub pepper_b64: String,
}
