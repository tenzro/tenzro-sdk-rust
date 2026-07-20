//! Passkey-first wallet RPC client.
//!
//! Mirrors the `tenzro_*Passkey*` / `tenzro_*Recovery*` /
//! `tenzro_*SessionKey*` / `tenzro_*HardwareSigner*` /
//! `tenzro_*SmartAccount*` RPC surface defined in
//! `crates/tenzro-node/src/passkey_rpc.rs`. Every method maps 1:1 to a
//! single JSON-RPC call so application code can wire passkey onboarding,
//! signing, social recovery, and session-key grants without re-hashing
//! the parameter envelopes.
//!
//! The companion [`crate::passkey::PasskeyWallet`] composes a
//! platform authenticator + signer + validator for the local
//! signing path; this module is the *network* counterpart that
//! talks to a Tenzro node.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct PasskeyClient {
    rpc: Arc<RpcClient>,
}

impl PasskeyClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Enroll a passkey-bound smart account. The node creates a TDIP human
    /// identity, deploys a smart account via the shared `AccountFactory`,
    /// installs the `WebAuthnValidator` as the primary signer with the
    /// supplied P-256 public key + ML-DSA-65 verifying key, and persists
    /// the smart account + identity binding.
    pub async fn enroll(
        &self,
        params: EnrollPasskeyParams,
    ) -> SdkResult<EnrollPasskeyResponse> {
        self.rpc.call("tenzro_enrollPasskey", serde_json::to_value(params)?).await
    }

    /// Verify a WebAuthn assertion against the registered passkey on a
    /// smart account. Returns `verified: true` iff the P-256 leg validates
    /// and the embedded challenge matches the supplied user-op hash.
    pub async fn sign(
        &self,
        params: SignWithPasskeyParams,
    ) -> SdkResult<SignWithPasskeyResponse> {
        self.rpc.call("tenzro_signWithPasskey", serde_json::to_value(params)?).await
    }

    /// Set the account's second-factor policy. `"two_credentials"` requires
    /// every UserOp signature bundle to carry assertions from two distinct
    /// enrolled passkeys; upgrading requires at least two enrolled
    /// credentials.
    pub async fn set_policy(
        &self,
        params: SetPasskeyPolicyParams,
    ) -> SdkResult<PasskeyPolicyResponse> {
        self.rpc.call("tenzro_setPasskeyPolicy", serde_json::to_value(params)?).await
    }

    /// Fetch the account's second-factor policy + enrolled credential count.
    pub async fn get_policy(
        &self,
        account_address: &str,
    ) -> SdkResult<PasskeyPolicyResponse> {
        self.rpc
            .call(
                "tenzro_getPasskeyPolicy",
                serde_json::json!({ "account_address": account_address }),
            )
            .await
    }

    /// Add (or update) a social-recovery guardian on a smart account.
    /// `threshold` is optional — when omitted the previous threshold is
    /// preserved.
    pub async fn add_guardian(
        &self,
        params: AddGuardianParams,
    ) -> SdkResult<AddGuardianResponse> {
        self.rpc.call("tenzro_addGuardian", serde_json::to_value(params)?).await
    }

    /// Start a social-recovery ceremony. Returns a `recovery_id` and the
    /// 32-byte `recovery_op_hash_hex` the guardians must sign with their
    /// composite (Ed25519 + ML-DSA-65) keys.
    pub async fn initiate_recovery(
        &self,
        params: InitiateRecoveryParams,
    ) -> SdkResult<InitiateRecoveryResponse> {
        self.rpc.call("tenzro_initiateRecovery", serde_json::to_value(params)?).await
    }

    /// Submit one guardian's composite signature against an in-flight recovery.
    pub async fn submit_recovery_signature(
        &self,
        params: SubmitRecoverySignatureParams,
    ) -> SdkResult<SubmitRecoverySignatureResponse> {
        self.rpc
            .call("tenzro_submitRecoverySignature", serde_json::to_value(params)?)
            .await
    }

    /// Finalize a recovery once quorum is reached. The node installs the
    /// new passkey as the smart account's primary `WebAuthnValidator`.
    pub async fn finalize_recovery(
        &self,
        params: FinalizeRecoveryParams,
    ) -> SdkResult<FinalizeRecoveryResponse> {
        self.rpc.call("tenzro_finalizeRecovery", serde_json::to_value(params)?).await
    }

    /// Grant a session key to a smart account with scoped permissions.
    pub async fn grant_session_key(
        &self,
        params: GrantSessionKeyParams,
    ) -> SdkResult<GrantSessionKeyResponse> {
        self.rpc.call("tenzro_grantSessionKey", serde_json::to_value(params)?).await
    }

    /// Revoke the session-key config from a smart account.
    pub async fn revoke_session_key(
        &self,
        params: RevokeSessionKeyParams,
    ) -> SdkResult<RevokeSessionKeyResponse> {
        self.rpc.call("tenzro_revokeSessionKey", serde_json::to_value(params)?).await
    }

    /// Install or update the per-account `SpendingLimitValidator` config.
    pub async fn set_spending_limit(
        &self,
        params: SetSpendingLimitParams,
    ) -> SdkResult<SetSpendingLimitResponse> {
        self.rpc.call("tenzro_setSpendingLimit", serde_json::to_value(params)?).await
    }

    /// Add a hardware-signer validator (Ledger / Trezor / GridPlus /
    /// YubiKey / generic) to a smart account.
    pub async fn add_hardware_signer(
        &self,
        params: AddHardwareSignerParams,
    ) -> SdkResult<AddHardwareSignerResponse> {
        self.rpc.call("tenzro_addHardwareSigner", serde_json::to_value(params)?).await
    }

    /// Fetch a smart account's current config + installed validators.
    pub async fn get_smart_account(
        &self,
        account_address: &str,
    ) -> SdkResult<SmartAccountSummary> {
        self.rpc
            .call(
                "tenzro_getSmartAccount",
                serde_json::json!({ "account_address": account_address }),
            )
            .await
    }

    /// List every smart account known to the node.
    pub async fn list_smart_accounts(&self) -> SdkResult<ListSmartAccountsResponse> {
        self.rpc.call("tenzro_listSmartAccounts", serde_json::json!({})).await
    }

    /// List in-flight social-recovery ceremonies for an account.
    pub async fn list_pending_recoveries(
        &self,
        account_address: &str,
    ) -> SdkResult<ListPendingRecoveriesResponse> {
        self.rpc
            .call(
                "tenzro_listPendingRecoveries",
                serde_json::json!({ "account_address": account_address }),
            )
            .await
    }

    // ---- Browser-launch WebAuthn session flow -------------------------

    /// Open a pending WebAuthn ceremony that a browser tab completes.
    ///
    /// The node returns a `verification_path` (`/auth/passkey?session=<id>`)
    /// that the caller opens; the page runs `navigator.credentials.create()`
    /// (enroll / add) or `navigator.credentials.get()` (sign) and posts the
    /// outcome back. Poll [`Self::get_passkey_session`] until the status is
    /// terminal. `kind` is `"enroll"`, `"add"`, or `"sign"`; the per-kind
    /// prerequisites (`ml_dsa_public_key_hex` for enroll, `account_address`
    /// for add/sign, `op_hash_hex` for sign) are validated up-front by the
    /// node.
    pub async fn create_passkey_session(
        &self,
        params: CreatePasskeySessionParams,
    ) -> SdkResult<CreatePasskeySessionResponse> {
        self.rpc
            .call("tenzro_createPasskeySession", serde_json::to_value(params)?)
            .await
    }

    /// Poll a pending WebAuthn session by id. Status transitions
    /// pending → in_flight → completed | failed | expired.
    pub async fn get_passkey_session(
        &self,
        session_id: &str,
    ) -> SdkResult<GetPasskeySessionResponse> {
        self.rpc
            .call(
                "tenzro_getPasskeySession",
                serde_json::json!({ "session_id": session_id }),
            )
            .await
    }

    // ---- Passkey credential management --------------------------------

    /// Add an additional passkey credential to an existing smart account.
    pub async fn add_passkey(
        &self,
        params: AddPasskeyParams,
    ) -> SdkResult<AddPasskeyResponse> {
        self.rpc.call("tenzro_addPasskey", serde_json::to_value(params)?).await
    }

    /// List the credential ids enrolled on an account.
    pub async fn list_passkeys(
        &self,
        account_address: &str,
    ) -> SdkResult<ListPasskeysResponse> {
        self.rpc
            .call(
                "tenzro_listPasskeys",
                serde_json::json!({ "account_address": account_address }),
            )
            .await
    }

    /// Remove an enrolled passkey credential from an account.
    pub async fn remove_passkey(
        &self,
        params: RemovePasskeyParams,
    ) -> SdkResult<RemovePasskeyResponse> {
        self.rpc.call("tenzro_removePasskey", serde_json::to_value(params)?).await
    }

    // ---- Threshold-key DKG (bridge signer establishment) --------------

    /// Start a DKLS23 secp256k1 distributed key generation session. Every
    /// participant's operator calls this with identical parameters (the node
    /// sorts `participant_dids` canonically) so the derived `instance_id` is
    /// byte-identical across parties. Admin-token-gated.
    pub async fn mpc_keygen(
        &self,
        params: MpcKeygenParams,
    ) -> SdkResult<MpcKeygenSession> {
        self.rpc.call("tenzro_mpcKeygen", serde_json::to_value(params)?).await
    }

    /// Fetch one DKG session by `instance_id`.
    pub async fn mpc_keygen_status(
        &self,
        instance_id: &str,
    ) -> SdkResult<MpcKeygenSession> {
        self.rpc
            .call(
                "tenzro_mpcKeygenStatus",
                serde_json::json!({ "instance_id": instance_id }),
            )
            .await
    }

    /// List every DKG session known to the node (ordered by start time).
    pub async fn list_mpc_keygen_sessions(&self) -> SdkResult<ListMpcKeygenSessionsResponse> {
        self.rpc.call("tenzro_mpcKeygenStatus", serde_json::json!({})).await
    }

    // ---- Identity / credential lifecycle ------------------------------

    /// Revoke an entire identity by DID across both the auth act-chain and
    /// the TDIP registry, broadcasting the revocation to peers.
    /// Admin-token-gated.
    pub async fn revoke_identity(
        &self,
        params: RevokeIdentityParams,
    ) -> SdkResult<RevokeIdentityResponse> {
        self.rpc
            .call("tenzro_revokeIdentity", serde_json::to_value(params)?)
            .await
    }

    /// Revoke a single JWT by its `jti`, cascading to the act-chain
    /// transitive closure. Admin-token-gated.
    pub async fn revoke_jwt(
        &self,
        params: RevokeJwtParams,
    ) -> SdkResult<RevokeJwtResponse> {
        self.rpc.call("tenzro_revokeJwt", serde_json::to_value(params)?).await
    }

    /// Register a trusted credential/claim issuer for the given topics.
    /// Admin-token-gated.
    pub async fn add_trusted_issuer(
        &self,
        params: AddTrustedIssuerParams,
    ) -> SdkResult<AddTrustedIssuerResponse> {
        self.rpc
            .call("tenzro_addTrustedIssuer", serde_json::to_value(params)?)
            .await
    }
}

// =============================================================================
// DTOs
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollPasskeyParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub passkey_public_key_hex: String,
    pub credential_id_hex: String,
    pub ml_dsa_public_key_hex: String,
    #[serde(default)]
    pub salt: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollPasskeyResponse {
    pub did: String,
    pub smart_account_address: String,
    pub credential_id_hex: String,
    pub webauthn_validator_address: String,
    pub installed_validators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignWithPasskeyParams {
    pub account_address: String,
    pub op_hash_hex: String,
    pub assertion: serde_json::Value,
    /// Hex credential id identifying which enrolled passkey produced the
    /// assertion.
    pub credential_id_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ml_dsa_signature_hex: Option<String>,
    /// Second-credential leg — required when the account's second-factor
    /// policy is `two_credentials`. All three `second_*` fields must be
    /// supplied together and must address a different enrolled credential
    /// than the primary leg.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_assertion: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_credential_id_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_ml_dsa_signature_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignWithPasskeyResponse {
    pub verified: bool,
    pub validator: String,
    pub op_hash_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPasskeyPolicyParams {
    pub account_address: String,
    /// `"single_credential"` or `"two_credentials"`.
    pub second_factor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyPolicyResponse {
    pub account_address: String,
    pub second_factor: String,
    pub required_signatures: usize,
    pub credentials_enrolled: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddGuardianParams {
    pub account_address: String,
    pub guardian_ed25519_pubkey_hex: String,
    pub guardian_ml_dsa_pubkey_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddGuardianResponse {
    pub account_address: String,
    pub guardian_count: u32,
    pub threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiateRecoveryParams {
    pub account_address: String,
    pub new_passkey_public_key_hex: String,
    pub new_credential_id_hex: String,
    pub new_ml_dsa_public_key_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiateRecoveryResponse {
    pub recovery_id: String,
    pub account_address: String,
    pub recovery_op_hash_hex: String,
    pub expires_at_ms: u64,
    pub guardians_required: u32,
    pub guardians_total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitRecoverySignatureParams {
    pub recovery_id: String,
    pub guardian_index: u32,
    pub composite_signature_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitRecoverySignatureResponse {
    pub recovery_id: String,
    pub guardian_signatures_collected: u32,
    pub guardians_required: u32,
    pub quorum_reached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizeRecoveryParams {
    pub recovery_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizeRecoveryResponse {
    pub recovery_id: String,
    pub account_address: String,
    pub new_credential_id_hex: String,
    pub installed_validators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantSessionKeyParams {
    pub account_address: String,
    pub session_pubkey_hex: String,
    pub allowed_selectors_hex: Vec<String>,
    #[serde(default)]
    pub allowed_targets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value_per_call_wei: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_value_wei: Option<String>,
    pub valid_after_unix: u64,
    pub valid_until_unix: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantSessionKeyResponse {
    pub account_address: String,
    pub session_pubkey_hex: String,
    pub valid_after_unix: u64,
    pub valid_until_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeSessionKeyParams {
    pub account_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeSessionKeyResponse {
    pub account_address: String,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetSpendingLimitParams {
    pub account_address: String,
    pub per_tx_cap_wei: String,
    pub daily_cap_wei: String,
    pub authenticator_pubkey_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetSpendingLimitResponse {
    pub account_address: String,
    pub per_tx_cap_wei: String,
    pub daily_cap_wei: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddHardwareSignerParams {
    pub account_address: String,
    pub device_kind: String,
    pub public_key_hex: String,
    #[serde(default)]
    pub required_always: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_above_wei: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddHardwareSignerResponse {
    pub account_address: String,
    pub device_kind: String,
    pub validator_module_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartAccountSummary {
    pub address: String,
    pub owner_hex: String,
    pub nonce: u64,
    pub is_deployed: bool,
    pub installed_validators: Vec<InstalledValidatorSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledValidatorSummary {
    pub module_address: String,
    pub type_id: u64,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSmartAccountsResponse {
    pub count: usize,
    pub smart_accounts: Vec<SmartAccountSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRecoverySummary {
    pub recovery_id: String,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub guardian_signatures_collected: usize,
    pub finalized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPendingRecoveriesResponse {
    pub account_address: String,
    pub count: usize,
    pub pending_recoveries: Vec<PendingRecoverySummary>,
}

// ---- Session flow ---------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePasskeySessionParams {
    /// `"enroll"`, `"add"`, or `"sign"`.
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Required for `enroll`: the ML-DSA-65 verifying key. Add sessions omit
    /// this — the node mints the new credential's post-quantum leg itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ml_dsa_public_key_hex: Option<String>,
    #[serde(default)]
    pub salt: u64,
    /// Required for `add` and `sign`: the target smart-account address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Required for `sign`: the 32-byte op hash (hex) the assertion attests to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op_hash_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ml_dsa_signature_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePasskeySessionResponse {
    pub session_id: String,
    /// Path on the node's web server to open in a browser
    /// (`/auth/passkey?session=<id>`).
    pub verification_path: String,
    /// `"pending" | "in_flight" | "completed" | "failed" | "expired"`.
    pub status: String,
    pub challenge_b64: String,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPasskeySessionResponse {
    pub session_id: String,
    /// `"enroll" | "add" | "sign"`.
    pub kind: String,
    /// `"pending" | "in_flight" | "completed" | "failed" | "expired"`.
    pub status: String,
    /// Present once the ceremony completes; the shape mirrors the underlying
    /// enroll / add / sign handler response.
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    pub expires_at_ms: u64,
}

// ---- Passkey credential management ----------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPasskeyParams {
    pub account_address: String,
    pub new_passkey_public_key_hex: String,
    pub new_credential_id_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPasskeyResponse {
    pub account_address: String,
    pub credential_id_hex: String,
    pub credentials_total: usize,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPasskeysResponse {
    pub account_address: String,
    pub count: usize,
    pub credential_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovePasskeyParams {
    pub account_address: String,
    pub credential_id_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovePasskeyResponse {
    pub account_address: String,
    pub credential_id_hex: String,
    pub removed: bool,
    pub credentials_remaining: usize,
}

// ---- Threshold-key DKG ----------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpcKeygenParams {
    /// This operator's DID; must appear in `participant_dids`.
    pub local_did: String,
    /// Every participant DID (the node sorts + dedups canonically).
    pub participant_dids: Vec<String>,
    pub threshold: u8,
    /// 64-hex finalized block hash — must agree across all parties.
    pub finalized_block_hash: String,
    /// 64-hex session nonce — must agree across all parties.
    pub session_nonce: String,
    /// DID → libp2p PeerId, one entry per remote participant (fail-closed).
    pub peer_bindings: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpcKeygenSession {
    pub instance_id: String,
    /// `"running" | "completed" | "failed"`.
    pub status: String,
    pub local_did: String,
    pub participant_dids: Vec<String>,
    pub threshold: u8,
    pub total_parties: u8,
    pub local_party_index: u8,
    pub started_at_ms: u64,
    #[serde(default)]
    pub finished_at_ms: Option<u64>,
    #[serde(default)]
    pub group_id: Option<String>,
    /// SEC1-compressed group public key.
    #[serde(default)]
    pub group_public_key: Option<String>,
    #[serde(default)]
    pub epoch: Option<u64>,
    /// EIP-55 checksummed address derived from the group key.
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMpcKeygenSessionsResponse {
    pub sessions: Vec<MpcKeygenSession>,
}

// ---- Identity / credential lifecycle --------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeIdentityParams {
    pub did: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeIdentityResponse {
    pub did: String,
    pub status: String,
    pub affected_jti_count: u64,
    /// `"revoked" | "not_registered" | "skipped_no_signer" | "skipped_no_registry"`.
    pub identity_registry: String,
    pub cascade: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeJwtParams {
    pub jti: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeJwtResponse {
    pub jti: String,
    pub status: String,
    pub cascade: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTrustedIssuerParams {
    pub issuer_did: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub topics: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTrustedIssuerResponse {
    pub issuer_did: String,
    pub name: String,
    pub topics: Vec<u64>,
    pub status: String,
}
