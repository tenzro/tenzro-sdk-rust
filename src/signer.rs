//! Pluggable signing surface for custom wallet developers.
//!
//! This module is the **low-level** half of the SDK's wallet API. The
//! high-level half — `passkey::PasskeyWallet` (a default composition that
//! uses `WebAuthnSigner` + platform-authenticator `KeyStorage` +
//! `WebAuthnValidator`) — is built on top of these traits. There is no
//! internal-only API: the same surface that the SDK's defaults consume is
//! exposed for custom wallet authors.
//!
//! See `docs/SPECIFICATION.md` §15.10.2 for the trait contract and §15.10.3
//! for the reference compositions (`passkey-only`, `passkey + tee`,
//! `frost-multi-device`, `tee-only-agent`, `delegated-session`,
//! `air-gapped`).
//!
//! # Trait surface
//!
//! - [`Signer`] — produces a [`Signature`] over a 32-byte hash.
//! - [`Validator`] — describes the on-chain ERC-7579 module that will
//!   verify the signature, and assembles the `validatorData` bytes that
//!   the EntryPoint passes to the module.
//! - [`KeyStorage`] — abstracts where the secret material actually lives
//!   (Secure Enclave, TPM, encrypted disk file, OS keychain, HSM, ...).
//! - [`RecoveryGuardian`] — propose / approve / execute social recovery
//!   under the `SocialRecoveryValidator` module.
//!
//! # Why traits, not concrete types
//!
//! The five reference compositions in the spec all share the same
//! surface. Locking the SDK to a concrete `Wallet` struct would force
//! every custom MPC topology, custom HSM integration, air-gapped flow,
//! or social-recovery topology to fork the SDK. Traits keep the
//! extension points stable while letting implementors pick their own
//! cryptography, transport, and storage backend.
//!
//! # Why this lives in the SDK and not in `tenzro-vm`
//!
//! `tenzro-vm` owns the on-chain side: the validator modules
//! themselves (`WebAuthnValidator`, `Ed25519Validator`,
//! `DelegationScopeValidator`, `TeeBoundValidator`) and the EntryPoint
//! that calls them. The SDK owns the off-chain side: how a wallet
//! produces the signature and the `validatorData` blob the user op
//! carries. Keeping the trait surface in the SDK keeps `tenzro-vm`
//! free of any reference to platform key stores, WebAuthn ceremonies,
//! or HSM SDKs.

use async_trait::async_trait;

use crate::error::SdkError;
use crate::types::Address;

/// What the [`Signer`] is — used by callers to pick the right
/// [`Validator`] module on the other end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignerKind {
    /// WebAuthn / passkey (P-256 over SHA-256 of `clientDataJSON`).
    /// Verified on-chain by `WebAuthnValidator` (RIP-7212 precompile).
    WebAuthn { credential_id: Vec<u8> },
    /// Plain Ed25519. Verified on-chain by `Ed25519Validator`.
    Ed25519,
    /// FROST-Ed25519 threshold signer. Produces a single aggregate
    /// signature that verifies under `Ed25519Validator` — the on-chain
    /// side does not see the threshold structure.
    Frost { threshold: u16, total: u16 },
    /// Key sealed inside a TEE; signature accompanied by an attestation.
    /// Verified on-chain by `TeeBoundValidator`.
    Tee { backend: TeeBackend },
    /// HSM-resident key (PKCS#11, vendor-specific, ...).
    /// Maps to whichever validator the HSM produces a signature for.
    Hsm { vendor: String },
    /// Custom signer not covered by the SDK's reference compositions.
    /// The composition author is responsible for picking the right
    /// [`Validator`].
    Custom(String),
}

/// Which TEE attests the [`SignerKind::Tee`] signer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeeBackend {
    IntelTdx,
    AmdSevSnp,
    AwsNitro,
    NvidiaCc,
    AppleSecureEnclave,
    AndroidStrongBox,
    WindowsTpm,
    LinuxTpm,
}

/// On-chain ERC-7579 module type. Matches `tenzro-vm::aa_validators::ModuleType`
/// in numeric value but kept separate so the SDK does not depend on the VM
/// crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Erc7579ModuleType {
    Validator = 1,
    Executor = 2,
    Fallback = 3,
    Hook = 4,
}

/// What the SDK calls a "raw" 4337 user operation. The SDK does not
/// re-export `tenzro_vm::PackedUserOperation` — the VM's encoding is
/// the source of truth, but the trait surface only needs the bytes.
///
/// Implementations that build a real op should produce the EIP-712
/// hash externally and feed it to [`Signer::sign`].
#[derive(Debug, Clone)]
pub struct PackedUserOperation {
    /// The 32-byte EIP-712 hash of the user op. This is what the
    /// [`Signer`] signs.
    pub op_hash: [u8; 32],
    /// Raw EntryPoint-format bytes for the validator module to consume
    /// when assembling its `validatorData` field. Empty if the
    /// validator does not need them.
    pub raw_op: Vec<u8>,
}

/// Per-call signing context. Lets the signer make policy decisions
/// (e.g. require biometric prompt for high-value ops) without the
/// caller having to know the signer's internals.
#[derive(Debug, Clone, Default)]
pub struct SignContext {
    /// Optional user-facing reason shown in the biometric prompt.
    pub prompt_reason: Option<String>,
    /// Domain tag the signer uses to scope the hash. Implementations
    /// MUST refuse to sign if this is `Some(tag)` and the tag does not
    /// match what the validator expects.
    pub domain_tag: Option<Vec<u8>>,
    /// Wall-clock deadline; the signer SHOULD return
    /// [`SignerError::Timeout`] if signing has not produced a result
    /// by then.
    pub deadline_ms: Option<u64>,
}

/// A finished signature. The shape varies by signer — WebAuthn gives
/// you `clientDataJSON + authenticatorData + DER sig`, Ed25519 gives
/// you 64 raw bytes — so we keep this opaque and let the [`Validator`]
/// figure it out at `build_validator_data` time.
#[derive(Debug, Clone)]
pub struct Signature {
    /// Raw signature payload. Format is signer-specific.
    pub bytes: Vec<u8>,
    /// Optional ancillary data the validator needs alongside `bytes`.
    /// For WebAuthn this carries `clientDataJSON` + `authenticatorData`;
    /// for TEE-bound signers it carries the attestation report.
    pub aux: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum SignerError {
    #[error("user cancelled the signing prompt")]
    UserCancelled,
    #[error("biometric / hardware authentication failed")]
    AuthenticationFailed,
    #[error("signing timed out")]
    Timeout,
    #[error("domain tag mismatch — refusing to sign")]
    DomainTagMismatch,
    #[error("signer backend unavailable: {0}")]
    BackendUnavailable(String),
    #[error("transport error: {0}")]
    Transport(String),
}

impl From<SignerError> for SdkError {
    fn from(e: SignerError) -> Self {
        SdkError::WalletError(e.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidatorError {
    #[error("validator does not support this user op shape: {0}")]
    Unsupported(String),
    #[error("failed to assemble validator data: {0}")]
    AssemblyFailed(String),
}

impl From<ValidatorError> for SdkError {
    fn from(e: ValidatorError) -> Self {
        SdkError::WalletError(e.to_string())
    }
}

/// Bytes stored under [`KeyId`] in [`KeyStorage`]. We keep this as a
/// newtype so consumers cannot accidentally mix up a key blob with an
/// arbitrary `Vec<u8>`.
#[derive(Debug, Clone)]
pub struct KeyBlob(pub Vec<u8>);

/// Stable identifier for a key inside [`KeyStorage`]. Format is
/// storage-specific (e.g. for the Secure Enclave it's the keychain
/// label; for an HSM it's the slot/object-id pair).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyId(pub String);

/// What the storage backend can promise about a stored key.
#[derive(Debug, Clone, Copy, Default)]
pub struct StorageCapabilities {
    /// True if the secret material never exists outside dedicated
    /// hardware (Secure Enclave / TPM / HSM).
    pub hardware_backed: bool,
    /// True if every signing operation forces a biometric / user-presence
    /// check.
    pub biometric_gated: bool,
    /// True if the key can be exported as raw bytes. Hardware-backed
    /// keys MUST set this to false.
    pub exportable: bool,
    /// True if the key survives a device reboot.
    pub persistent: bool,
}

/// Per-key policy attached at store time.
#[derive(Debug, Clone, Default)]
pub struct StoragePolicy {
    /// Require a biometric / user-presence check on every load/use.
    pub biometric_required: bool,
    /// Refuse to back up this key to cloud storage even if the
    /// platform offers it.
    pub no_cloud_backup: bool,
    /// Bind the key to this device — refuse to migrate even if the
    /// user signs in to the same account on another device.
    pub device_bound: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("key not found: {0}")]
    NotFound(String),
    #[error("storage backend rejected the operation: {0}")]
    Rejected(String),
    #[error("storage backend unavailable: {0}")]
    Unavailable(String),
}

impl From<StorageError> for SdkError {
    fn from(e: StorageError) -> Self {
        SdkError::WalletError(e.to_string())
    }
}

/// Produces a [`Signature`] over a 32-byte hash. Implementations are
/// expected to be stateless wrt the hash — all per-call state lives
/// in the [`SignContext`].
#[async_trait]
pub trait Signer: Send + Sync {
    /// Self-describes what kind of signer this is so callers can pick
    /// the matching [`Validator`].
    fn describe(&self) -> SignerKind;

    /// Signs `hash` and returns the [`Signature`]. The exact
    /// hashing scheme (raw, EIP-712, WebAuthn `clientDataJSON`)
    /// depends on the [`SignerKind`].
    async fn sign(
        &self,
        hash: [u8; 32],
        context: &SignContext,
    ) -> Result<Signature, SignerError>;
}

/// Builds the `validatorData` blob that an ERC-7579 validator module
/// expects in `UserOperation.signature`. This is the off-chain mirror
/// of the on-chain validator the wallet has installed.
#[async_trait]
pub trait Validator: Send + Sync {
    /// Address of the on-chain ERC-7579 module that will verify the
    /// signature.
    fn module_address(&self) -> Address;

    /// Module type — Validator/Executor/Fallback/Hook.
    fn module_type(&self) -> Erc7579ModuleType;

    /// Assembles the `validatorData` blob for `user_op`. May call
    /// out to the configured [`Signer`] internally.
    async fn build_validator_data(
        &self,
        user_op: &PackedUserOperation,
    ) -> Result<Vec<u8>, ValidatorError>;
}

/// Where secret material actually lives. Platform-specific
/// implementations live in the SDK's `passkey`, `tee`, and `hsm`
/// modules; custom implementations live in user crates.
#[async_trait]
pub trait KeyStorage: Send + Sync {
    /// Stores `blob` under `key_id` with the given `policy`. Returns
    /// `Ok(())` only if the storage backend has durably committed it.
    async fn store(
        &self,
        key_id: &KeyId,
        blob: &[u8],
        policy: StoragePolicy,
    ) -> Result<(), StorageError>;

    /// Loads the bytes previously stored under `key_id`. Hardware-
    /// backed implementations MAY refuse this and only expose
    /// `sign`-shaped operations through a separate channel.
    async fn load(&self, key_id: &KeyId) -> Result<Vec<u8>, StorageError>;

    /// Capabilities the backend offers — callers use this to decide
    /// whether a key is suitable for a given threat model.
    fn capabilities(&self) -> StorageCapabilities;
}

/// Recovery proposal as carried by the social-recovery flow.
#[derive(Debug, Clone)]
pub struct RecoveryProposal {
    pub account: Address,
    /// 33- or 65-byte uncompressed/compressed public key for the new
    /// owner.
    pub new_owner: Vec<u8>,
    /// Opaque proposal ID — implementations choose the format
    /// (UUID, on-chain proposal hash, ...).
    pub proposal_id: Vec<u8>,
}

/// Per-guardian signature on a [`RecoveryProposal`]. Format is
/// guardian-specific (Ed25519 over `proposal_id`, EIP-712 typed
/// data, ...).
#[derive(Debug, Clone)]
pub struct GuardianSignature {
    pub guardian_address: Address,
    pub signature: Vec<u8>,
}

/// Hex-encoded transaction hash returned after `execute_recovery`.
#[derive(Debug, Clone)]
pub struct TxHash(pub String);

#[derive(Debug, thiserror::Error)]
pub enum RecoveryError {
    #[error("not enough guardian signatures: have {have}, need {need}")]
    Threshold { have: usize, need: usize },
    #[error("guardian signature invalid: {0}")]
    InvalidSignature(String),
    #[error("recovery proposal not found")]
    NotFound,
    #[error("transport error: {0}")]
    Transport(String),
}

impl From<RecoveryError> for SdkError {
    fn from(e: RecoveryError) -> Self {
        SdkError::WalletError(e.to_string())
    }
}

/// Social-recovery surface for the `SocialRecoveryValidator` module.
/// Each guardian implementation may proxy the call to a remote
/// guardian device, an off-chain quorum service, or a local hardware
/// module — the trait does not constrain the transport.
#[async_trait]
pub trait RecoveryGuardian: Send + Sync {
    async fn propose_recovery(
        &self,
        account: Address,
        new_owner: Vec<u8>,
    ) -> Result<RecoveryProposal, RecoveryError>;

    async fn approve_recovery(
        &self,
        proposal: &RecoveryProposal,
    ) -> Result<GuardianSignature, RecoveryError>;

    async fn execute_recovery(
        &self,
        proposal: RecoveryProposal,
        sigs: Vec<GuardianSignature>,
    ) -> Result<TxHash, RecoveryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirms the trait surface is object-safe — composition authors
    /// will store these as `Arc<dyn Signer>` etc., so the methods must
    /// not have generic parameters or `where Self: Sized` bounds.
    #[test]
    fn trait_surface_is_object_safe() {
        fn _accept_signer(_s: &dyn Signer) {}
        fn _accept_validator(_v: &dyn Validator) {}
        fn _accept_storage(_k: &dyn KeyStorage) {}
        fn _accept_recovery(_r: &dyn RecoveryGuardian) {}
    }

    /// An echo signer for downstream tests — produces a deterministic
    /// signature so a [`Validator`] composition test can run without
    /// real cryptography. Lives here (not in dev-deps) because
    /// composition authors will reach for it as a reference.
    pub struct EchoSigner;
    #[async_trait]
    impl Signer for EchoSigner {
        fn describe(&self) -> SignerKind {
            SignerKind::Custom("echo".into())
        }
        async fn sign(
            &self,
            hash: [u8; 32],
            _ctx: &SignContext,
        ) -> Result<Signature, SignerError> {
            Ok(Signature {
                bytes: hash.to_vec(),
                aux: vec![],
            })
        }
    }

    #[tokio::test]
    async fn echo_signer_round_trips() {
        let s = EchoSigner;
        let sig = s
            .sign([0x42; 32], &SignContext::default())
            .await
            .unwrap();
        assert_eq!(sig.bytes, vec![0x42; 32]);
        match s.describe() {
            SignerKind::Custom(name) => assert_eq!(name, "echo"),
            _ => panic!("wrong SignerKind"),
        }
    }

    #[test]
    fn signer_error_maps_to_sdk_error() {
        let e: SdkError = SignerError::UserCancelled.into();
        match e {
            SdkError::WalletError(msg) => assert!(msg.contains("user cancelled")),
            _ => panic!("wrong SdkError variant"),
        }
    }
}
