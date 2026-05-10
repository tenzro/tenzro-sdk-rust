//! Passkey wallet — the high-level default composition.
//!
//! This is the "three-line" surface from `docs/SPECIFICATION.md` §15.10.1:
//!
//! ```ignore
//! use tenzro_sdk::passkey::{PasskeyWallet, PasskeyConfig};
//!
//! let wallet = PasskeyWallet::create(
//!     PasskeyConfig::production("keys.tenzro.network"),
//!     authenticator,
//! ).await?;
//! let sig = wallet.sign_user_op(user_op).await?;
//! ```
//!
//! Internally `PasskeyWallet` is a composition over the trait surface
//! from [`crate::signer`]:
//!
//! ```text
//!   PasskeyWallet
//!   ├── PlatformAuthenticator  (what touches the actual passkey)
//!   ├── Signer = WebAuthnSigner (wraps the authenticator)
//!   └── Validator = WebAuthnValidator (off-chain mirror of the
//!                                      on-chain `WebAuthnValidator`
//!                                      module the wallet has installed)
//! ```
//!
//! # Why a `PlatformAuthenticator` trait
//!
//! WebAuthn ceremonies live in different places on different hosts:
//!
//! - **Browser:** `navigator.credentials.create/get` (TS SDK only).
//! - **Tauri desktop:** Rust-side commands that call
//!   `security-framework` / `tss-esapi` / JNI (the `device_*`
//!   commands implemented in `apps/tenzro-desktop/src-tauri`).
//! - **Headless server / CLI:** software P-256 fallback (the
//!   `SoftwareP256Authenticator` shipped here for tests and CI).
//!
//! Pinning the SDK to `security-framework` would force every
//! consumer to link macOS frameworks. Trait the boundary instead
//! and let each frontend supply its own authenticator.
//!
//! # Cross-device QR flow
//!
//! When the local device has no platform authenticator
//! (`is_platform_authenticator_available() == false` in TS), the
//! WebAuthn spec mandates a cross-platform authenticator with
//! `userVerification: "required"`. On native, that means the FIDO
//! hybrid transport (caBLE) — the SDK exposes this via
//! [`PasskeyWallet::start_cross_device_link`], which returns a QR
//! payload + a future that resolves once the remote authenticator
//! completes the ceremony. The actual caBLE wire is implemented
//! by the Tauri command `device_start_cross_device_link`; the SDK
//! ships only the surface.

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::{SdkError, SdkResult};
use crate::signer::{
    Erc7579ModuleType, PackedUserOperation, SignContext, Signer, SignerError, SignerKind,
    Validator, ValidatorError,
};
use crate::types::Address;

/// Bytes of a registered passkey credential — the `credential_id`
/// returned by the authenticator at create time, plus the public key
/// the verifier uses to check signatures.
#[derive(Debug, Clone)]
pub struct PasskeyCredential {
    /// Authenticator-assigned credential ID (opaque blob, typically
    /// 16-32 bytes for platform authenticators, longer for security
    /// keys).
    pub credential_id: Vec<u8>,
    /// 64-byte uncompressed P-256 public key (`x ‖ y`, no SEC1
    /// 0x04 prefix).
    pub public_key: [u8; 64],
}

/// Configuration for the WebAuthn ceremony — relying-party identity
/// and security policy.
#[derive(Debug, Clone)]
pub struct PasskeyConfig {
    /// Relying-party ID. MUST be a registrable domain (e.g.
    /// `keys.tenzro.network`); MUST NOT include scheme or path.
    pub rp_id: String,
    /// Human-readable RP name shown in the OS prompt
    /// ("Tenzro Network").
    pub rp_name: String,
    /// Require user verification (biometric or PIN). Production code
    /// MUST set this to `true` — the spec says so and the on-chain
    /// `WebAuthnValidator` checks the `UV` flag.
    pub require_user_verification: bool,
    /// Require a platform-bound authenticator (Touch ID / Face ID /
    /// Windows Hello). When `false`, the SDK falls back to a
    /// cross-platform authenticator over caBLE.
    pub require_platform_authenticator: bool,
}

impl PasskeyConfig {
    /// Production preset: `keys.tenzro.network` with strict UV and
    /// platform attachment.
    pub fn production(rp_id: impl Into<String>) -> Self {
        Self {
            rp_id: rp_id.into(),
            rp_name: "Tenzro Network".to_string(),
            require_user_verification: true,
            require_platform_authenticator: true,
        }
    }

    /// Development preset: same as production but allows cross-platform
    /// authenticators (so a developer on a Linux box without Touch ID
    /// can still use a YubiKey).
    pub fn development(rp_id: impl Into<String>) -> Self {
        Self {
            rp_id: rp_id.into(),
            rp_name: "Tenzro Network (dev)".to_string(),
            require_user_verification: true,
            require_platform_authenticator: false,
        }
    }
}

/// Per-ceremony output — what the authenticator returns when the user
/// completes a `create` ceremony.
#[derive(Debug, Clone)]
pub struct AuthenticatorRegistration {
    pub credential: PasskeyCredential,
    /// Raw `attestationObject` bytes (CBOR). Empty for platform
    /// authenticators that emit `none` attestation; non-empty for
    /// security-key attestations.
    pub attestation_object: Vec<u8>,
}

/// Per-ceremony output — what the authenticator returns when the user
/// completes a `get` (assertion) ceremony.
#[derive(Debug, Clone)]
pub struct AuthenticatorAssertion {
    /// Raw P-256 signature (64 bytes, `r ‖ s`).
    pub signature: [u8; 64],
    /// Bytes the authenticator hashed alongside `clientDataHash` —
    /// the WebAuthn `authenticatorData` blob.
    pub authenticator_data: Vec<u8>,
    /// `clientDataJSON` the authenticator built. This is what the
    /// challenge actually got embedded into; the on-chain validator
    /// recomputes the hash from this.
    pub client_data_json: Vec<u8>,
}

/// QR payload + future for the cross-device hybrid (caBLE) flow.
#[derive(Debug, Clone)]
pub struct CrossDeviceLink {
    /// QR-encoded `FIDO:/...` URI per the FIDO hybrid spec. Render
    /// this as a QR code; the user scans with their phone and the
    /// remote authenticator completes the ceremony.
    pub qr_uri: String,
    /// Channel ID the SDK uses to await the remote completion.
    pub channel_id: String,
}

/// Trait every host implements once and reuses for every wallet. The
/// SDK does not depend on any platform-specific crate — the desktop
/// app's `device_key.rs` implements this against `security-framework`,
/// the browser implementation calls `navigator.credentials`, and the
/// reference [`SoftwareP256Authenticator`] uses an in-memory P-256
/// keypair.
#[async_trait]
pub trait PlatformAuthenticator: Send + Sync {
    /// Whether the host has a platform-bound authenticator
    /// (Touch ID / Face ID / Windows Hello / Android biometric).
    /// When `false`, the wallet MUST switch to the cross-device QR
    /// flow per Spec §15.10.1 step 1.
    async fn is_platform_authenticator_available(&self) -> bool;

    /// Performs a WebAuthn `create` ceremony.
    async fn create_credential(
        &self,
        config: &PasskeyConfig,
        challenge: &[u8],
    ) -> Result<AuthenticatorRegistration, SignerError>;

    /// Performs a WebAuthn `get` (assertion) ceremony against the
    /// supplied credential.
    async fn sign_assertion(
        &self,
        config: &PasskeyConfig,
        credential_id: &[u8],
        challenge: &[u8],
    ) -> Result<AuthenticatorAssertion, SignerError>;

    /// Starts a cross-device hybrid (caBLE) ceremony. Returns the QR
    /// payload immediately; the caller awaits completion via the
    /// channel ID.
    async fn start_cross_device_link(
        &self,
        config: &PasskeyConfig,
    ) -> Result<CrossDeviceLink, SignerError>;
}

/// [`Signer`] impl that delegates to a [`PlatformAuthenticator`].
pub struct WebAuthnSigner {
    authenticator: Arc<dyn PlatformAuthenticator>,
    credential: PasskeyCredential,
    config: PasskeyConfig,
}

impl WebAuthnSigner {
    pub fn new(
        authenticator: Arc<dyn PlatformAuthenticator>,
        credential: PasskeyCredential,
        config: PasskeyConfig,
    ) -> Self {
        Self {
            authenticator,
            credential,
            config,
        }
    }

    pub fn credential(&self) -> &PasskeyCredential {
        &self.credential
    }
}

#[async_trait]
impl Signer for WebAuthnSigner {
    fn describe(&self) -> SignerKind {
        SignerKind::WebAuthn {
            credential_id: self.credential.credential_id.clone(),
        }
    }

    async fn sign(
        &self,
        hash: [u8; 32],
        _ctx: &SignContext,
    ) -> Result<crate::signer::Signature, SignerError> {
        let assertion = self
            .authenticator
            .sign_assertion(&self.config, &self.credential.credential_id, &hash)
            .await?;
        // The Validator side will need authenticatorData + clientDataJSON
        // to reconstruct the signed payload; pack them into `aux` so
        // the validator can pick them up without a second round-trip.
        let mut aux = Vec::with_capacity(
            8 + assertion.authenticator_data.len() + assertion.client_data_json.len(),
        );
        aux.extend_from_slice(&(assertion.authenticator_data.len() as u32).to_le_bytes());
        aux.extend_from_slice(&assertion.authenticator_data);
        aux.extend_from_slice(&(assertion.client_data_json.len() as u32).to_le_bytes());
        aux.extend_from_slice(&assertion.client_data_json);
        Ok(crate::signer::Signature {
            bytes: assertion.signature.to_vec(),
            aux,
        })
    }
}

/// [`Validator`] impl that targets the on-chain `WebAuthnValidator`
/// module installed on the wallet's smart account.
pub struct WebAuthnValidator {
    module_address: Address,
    signer: Arc<WebAuthnSigner>,
}

impl WebAuthnValidator {
    pub fn new(module_address: Address, signer: Arc<WebAuthnSigner>) -> Self {
        Self {
            module_address,
            signer,
        }
    }
}

#[async_trait]
impl Validator for WebAuthnValidator {
    fn module_address(&self) -> Address {
        self.module_address.clone()
    }

    fn module_type(&self) -> Erc7579ModuleType {
        Erc7579ModuleType::Validator
    }

    async fn build_validator_data(
        &self,
        user_op: &PackedUserOperation,
    ) -> Result<Vec<u8>, ValidatorError> {
        let sig = self
            .signer
            .sign(user_op.op_hash, &SignContext::default())
            .await
            .map_err(|e| ValidatorError::AssemblyFailed(e.to_string()))?;
        // Wire format consumed by `aa_webauthn_validator::WebAuthnValidator`:
        //   sig.bytes (raw 64-byte P-256 sig) || sig.aux (length-prefixed
        //   authenticatorData + clientDataJSON).
        let mut out = Vec::with_capacity(sig.bytes.len() + sig.aux.len());
        out.extend_from_slice(&sig.bytes);
        out.extend_from_slice(&sig.aux);
        Ok(out)
    }
}

/// The high-level "three-line" wallet. Composes a [`PasskeyConfig`],
/// a [`PlatformAuthenticator`], and a freshly-registered passkey into
/// a single object that exposes one method: [`PasskeyWallet::sign_user_op`].
pub struct PasskeyWallet {
    config: PasskeyConfig,
    authenticator: Arc<dyn PlatformAuthenticator>,
    signer: Arc<WebAuthnSigner>,
    /// On-chain `WebAuthnValidator` module address. Set after the
    /// first transaction registers the module — `None` means the
    /// wallet has not been bootstrapped yet (caller should call
    /// [`PasskeyWallet::bind_validator_module`] after the bootstrap
    /// op lands).
    validator_module: Option<Address>,
}

impl std::fmt::Debug for PasskeyWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasskeyWallet")
            .field("config", &self.config)
            .field("credential_id_len", &self.signer.credential().credential_id.len())
            .field("validator_module", &self.validator_module)
            .finish()
    }
}

impl PasskeyWallet {
    /// One-shot create: runs the WebAuthn ceremony, returns the
    /// composed wallet. Per Spec §15.10.1 step 1, when no platform
    /// authenticator is available the SDK MUST switch to the
    /// cross-device flow rather than silently falling back to a
    /// software key — this is enforced here.
    pub async fn create(
        config: PasskeyConfig,
        authenticator: Arc<dyn PlatformAuthenticator>,
    ) -> SdkResult<Self> {
        if config.require_platform_authenticator
            && !authenticator.is_platform_authenticator_available().await
        {
            return Err(SdkError::WalletError(
                "no platform authenticator available — use start_cross_device_link \
                 to render a QR for the FIDO hybrid (caBLE) flow"
                    .to_string(),
            ));
        }

        // Per WebAuthn spec, the registration challenge is 32 random
        // bytes. We do not anchor it on-chain at create time — the
        // first user-op that installs the validator module carries
        // its own EIP-712 hash that the authenticator signs over.
        let challenge = random_challenge();
        let registration = authenticator
            .create_credential(&config, &challenge)
            .await
            .map_err(SdkError::from)?;

        let signer = Arc::new(WebAuthnSigner::new(
            authenticator.clone(),
            registration.credential,
            config.clone(),
        ));

        Ok(Self {
            config,
            authenticator,
            signer,
            validator_module: None,
        })
    }

    /// Bind the on-chain `WebAuthnValidator` module address after the
    /// bootstrap op has installed it. Until this is called, the
    /// wallet can sign hashes but cannot produce a `validatorData`
    /// blob for an on-chain user op.
    pub fn bind_validator_module(&mut self, address: Address) {
        self.validator_module = Some(address);
    }

    /// The credential the wallet holds. Useful for displaying
    /// "Touch ID with key XX:XX:..." in the UI.
    pub fn credential(&self) -> &PasskeyCredential {
        self.signer.credential()
    }

    /// Sign a user op. Returns the bytes ready to drop into
    /// `UserOperation.signature`.
    pub async fn sign_user_op(
        &self,
        user_op: &PackedUserOperation,
    ) -> SdkResult<Vec<u8>> {
        let module = self.validator_module.clone().ok_or_else(|| {
            SdkError::WalletError(
                "validator module not yet bound — call bind_validator_module \
                 after the bootstrap op lands"
                    .to_string(),
            )
        })?;
        let validator = WebAuthnValidator::new(module, self.signer.clone());
        validator
            .build_validator_data(user_op)
            .await
            .map_err(SdkError::from)
    }

    /// Render a cross-device QR for the FIDO hybrid (caBLE) flow.
    /// The actual transport lives in the host's
    /// [`PlatformAuthenticator`] impl.
    pub async fn start_cross_device_link(&self) -> SdkResult<CrossDeviceLink> {
        self.authenticator
            .start_cross_device_link(&self.config)
            .await
            .map_err(SdkError::from)
    }
}

/// Spec §15.10.1: WebAuthn challenges are 32 random bytes. We use
/// `getrandom` rather than a CSPRNG-backed `rand` chain to keep the
/// dependency tree small.
fn random_challenge() -> [u8; 32] {
    let mut buf = [0u8; 32];
    // `getrandom` is a transitive dep of `reqwest` / `tokio`, so this
    // is free.
    getrandom::getrandom(&mut buf).expect("OS RNG must be available");
    buf
}

// --- Reference software implementation -------------------------------

/// In-memory P-256 authenticator for tests, CI, and headless servers.
///
/// **Not for production wallets.** The whole point of a passkey is
/// that the secret never leaves dedicated hardware; a software
/// keypair gives you the API shape but none of the security. The SDK
/// exposes this so:
///
/// - Unit tests can run the full `create` → `sign_user_op` path
///   without a Touch ID prompt.
/// - Headless services (bots, CI runners, agent runtimes) that
///   already trust the host can use the same composition.
/// - Custom wallet authors have a working reference to copy from.
///
/// The `is_platform_authenticator_available()` method returns
/// `false` so that any caller using a production [`PasskeyConfig`]
/// will be forced into the cross-device flow rather than silently
/// adopting this software key. That is the safe default: opting into
/// software keys requires using [`PasskeyConfig::development`] or
/// equivalent.
pub struct SoftwareP256Authenticator {
    signing_key: p256::ecdsa::SigningKey,
    public_key_bytes: [u8; 64],
    /// The credential ID we hand out at `create` time.
    credential_id: Vec<u8>,
}

impl SoftwareP256Authenticator {
    pub fn new() -> Self {
        use p256::ecdsa::SigningKey;
        let signing_key = SigningKey::random(&mut p256_rand::OsRng);
        let verifying_key = signing_key.verifying_key();
        // SEC1 uncompressed encoding is `0x04 || x(32) || y(32)`. We
        // store the 64-byte form expected by the on-chain
        // WebAuthnValidator (no leading 0x04).
        let encoded = verifying_key.to_encoded_point(false);
        let bytes = encoded.as_bytes();
        debug_assert_eq!(bytes.len(), 65);
        debug_assert_eq!(bytes[0], 0x04);
        let mut public_key_bytes = [0u8; 64];
        public_key_bytes.copy_from_slice(&bytes[1..]);

        // Credential IDs are opaque to the relying party; pick any
        // 16-byte value that is unique per authenticator instance.
        let mut credential_id = [0u8; 16];
        getrandom::getrandom(&mut credential_id).expect("OS RNG must be available");
        Self {
            signing_key,
            public_key_bytes,
            credential_id: credential_id.to_vec(),
        }
    }
}

/// Local re-export so we can use `p256`'s OS RNG without depending on
/// `rand_core` directly. `p256` re-exports `rand_core::OsRng` via its
/// own surface.
mod p256_rand {
    pub use p256::elliptic_curve::rand_core::OsRng;
}

impl Default for SoftwareP256Authenticator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformAuthenticator for SoftwareP256Authenticator {
    async fn is_platform_authenticator_available(&self) -> bool {
        // Software-backed; not a platform authenticator. Production
        // configs will refuse to use this and force the cross-device
        // flow — which is what we want.
        false
    }

    async fn create_credential(
        &self,
        _config: &PasskeyConfig,
        _challenge: &[u8],
    ) -> Result<AuthenticatorRegistration, SignerError> {
        Ok(AuthenticatorRegistration {
            credential: PasskeyCredential {
                credential_id: self.credential_id.clone(),
                public_key: self.public_key_bytes,
            },
            attestation_object: vec![],
        })
    }

    async fn sign_assertion(
        &self,
        _config: &PasskeyConfig,
        credential_id: &[u8],
        challenge: &[u8],
    ) -> Result<AuthenticatorAssertion, SignerError> {
        if credential_id != self.credential_id.as_slice() {
            return Err(SignerError::AuthenticationFailed);
        }
        // WebAuthn signs over `authenticatorData || SHA-256(clientDataJSON)`.
        // The reference impl uses dummy authenticatorData (RP ID hash + flags
        // = UP|UV, signCount = 0) and a minimal clientDataJSON. The on-chain
        // verifier reconstructs the same payload from these bytes — so any
        // changes here MUST stay in lockstep with `aa_webauthn_validator`.
        let mut authenticator_data = Vec::with_capacity(37);
        authenticator_data.extend_from_slice(&[0u8; 32]); // rpIdHash placeholder
        authenticator_data.push(0b0000_0101); // UP=1, UV=1
        authenticator_data.extend_from_slice(&[0u8; 4]); // signCount = 0

        let client_data_json = format!(
            "{{\"type\":\"webauthn.get\",\"challenge\":\"{}\",\"origin\":\"https://localhost\"}}",
            base64::Engine::encode(
                &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                challenge
            )
        );
        let client_data_bytes = client_data_json.into_bytes();

        // WebAuthn signed payload = authenticatorData || SHA-256(clientDataJSON)
        use sha2::{Digest, Sha256};
        let client_data_hash = Sha256::digest(&client_data_bytes);
        let mut signed = Vec::with_capacity(authenticator_data.len() + 32);
        signed.extend_from_slice(&authenticator_data);
        signed.extend_from_slice(&client_data_hash);

        // ECDSA over P-256. The `Signature::from_slice` round-trip
        // guarantees we emit the canonical 64-byte `r || s` form the
        // on-chain validator expects.
        use p256::ecdsa::signature::Signer as _;
        let sig: p256::ecdsa::Signature = self.signing_key.sign(&signed);
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&sig.to_bytes());

        Ok(AuthenticatorAssertion {
            signature: sig_bytes,
            authenticator_data,
            client_data_json: client_data_bytes,
        })
    }

    async fn start_cross_device_link(
        &self,
        _config: &PasskeyConfig,
    ) -> Result<CrossDeviceLink, SignerError> {
        // The reference software authenticator does not implement
        // caBLE. Production hosts (Tauri / browser) implement this
        // against the FIDO hybrid transport.
        Err(SignerError::BackendUnavailable(
            "cross-device link is host-specific (Tauri / browser); software \
             reference does not implement caBLE"
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_with_production_config_refuses_software_authenticator() {
        let auth = Arc::new(SoftwareP256Authenticator::new());
        let cfg = PasskeyConfig::production("keys.tenzro.network");
        let err = PasskeyWallet::create(cfg, auth).await.unwrap_err();
        match err {
            SdkError::WalletError(msg) => assert!(msg.contains("no platform authenticator")),
            _ => panic!("wrong error variant"),
        }
    }

    #[tokio::test]
    async fn create_with_development_config_accepts_software_authenticator() {
        let auth = Arc::new(SoftwareP256Authenticator::new());
        let cfg = PasskeyConfig::development("keys.tenzro.network");
        let wallet = PasskeyWallet::create(cfg, auth).await.expect("create ok");
        assert_eq!(wallet.credential().public_key.len(), 64);
        assert!(!wallet.credential().credential_id.is_empty());
    }

    #[tokio::test]
    async fn sign_user_op_fails_until_validator_module_bound() {
        let auth = Arc::new(SoftwareP256Authenticator::new());
        let cfg = PasskeyConfig::development("keys.tenzro.network");
        let wallet = PasskeyWallet::create(cfg, auth).await.unwrap();
        let op = PackedUserOperation {
            op_hash: [0x42; 32],
            raw_op: vec![],
        };
        let err = wallet.sign_user_op(&op).await.unwrap_err();
        match err {
            SdkError::WalletError(msg) => assert!(msg.contains("validator module not yet bound")),
            _ => panic!("wrong error variant"),
        }
    }

    #[tokio::test]
    async fn full_round_trip_after_binding() {
        let auth = Arc::new(SoftwareP256Authenticator::new());
        let cfg = PasskeyConfig::development("keys.tenzro.network");
        let mut wallet = PasskeyWallet::create(cfg, auth).await.unwrap();
        wallet.bind_validator_module(Address::zero());
        let op = PackedUserOperation {
            op_hash: [0xAB; 32],
            raw_op: vec![],
        };
        let sig = wallet.sign_user_op(&op).await.expect("sign ok");
        // 64 raw P-256 sig bytes + 4-byte authenticator_data length +
        // ~37 bytes authenticator_data + 4-byte clientDataJSON length +
        // clientDataJSON. Lower bound conservative.
        assert!(sig.len() > 64 + 8 + 37);
    }

    #[tokio::test]
    async fn cross_device_link_surface_exists() {
        let auth = Arc::new(SoftwareP256Authenticator::new());
        let cfg = PasskeyConfig::development("keys.tenzro.network");
        let wallet = PasskeyWallet::create(cfg, auth).await.unwrap();
        // Reference impl returns BackendUnavailable; host-specific
        // impls (Tauri / browser) return a real QR. The point of
        // this test is that the surface compiles and routes to the
        // authenticator.
        let err = wallet.start_cross_device_link().await.unwrap_err();
        assert!(err.to_string().contains("cross-device link is host-specific"));
    }
}
