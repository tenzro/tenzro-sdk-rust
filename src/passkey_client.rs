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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ml_dsa_signature_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignWithPasskeyResponse {
    pub verified: bool,
    pub validator: String,
    pub op_hash_hex: String,
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
