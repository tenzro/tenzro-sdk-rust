//! API-key management SDK for Tenzro Network.
//!
//! Two control planes:
//!
//! 1. **Operator** (`X-Tenzro-Admin-Token`): mint / list / revoke any key
//!    on the operator's own node. Sourced from the `TENZRO_ADMIN_TOKEN`
//!    env var by the underlying `RpcClient`.
//! 2. **Subject** (`X-Tenzro-Api-Key`): list / revoke keys belonging to
//!    the caller's own subject. Sourced from `TENZRO_API_KEY`.
//!
//! Every Tenzro node operator holds their own admin token for *their
//! own* node. There is no global "Tenzro Labs token," and admin
//! capabilities do not extend to network-wide state (validator set,
//! treasury, fee schedule, system contracts — those flow through
//! on-chain governance via `tenzro-token`). See `docs/api-keys.md`.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// API-key management client.
#[derive(Clone)]
pub struct ApiKeyClient {
    rpc: Arc<RpcClient>,
}

impl ApiKeyClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    // ── Operator surface (admin-token-gated) ─────────────────────────

    /// Mint a new API key on this node. Requires `TENZRO_ADMIN_TOKEN`
    /// in the environment.
    ///
    /// `class` controls revocability:
    /// - [`KeyClass::Subject`] (default): subject can self-revoke,
    ///   admin can revoke.
    /// - [`KeyClass::OperatorInternal`]: admin-only revoke.
    /// - [`KeyClass::OperatorProtected`]: not revokable via RPC —
    ///   rotate by updating the operator secret and restarting the
    ///   node. Requires the `confirm_operator_protected` interlock
    ///   server-side.
    pub async fn create(&self, params: CreateApiKeyParams) -> SdkResult<CreatedApiKey> {
        let mut body = serde_json::Map::new();
        body.insert("label".to_string(), serde_json::Value::String(params.label));
        if let Some(subject) = params.subject {
            body.insert("subject".to_string(), serde_json::Value::String(subject));
        }
        body.insert(
            "scopes".to_string(),
            serde_json::Value::Array(
                params
                    .scopes
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
        body.insert(
            "class".to_string(),
            serde_json::Value::String(params.class.as_str().to_string()),
        );
        if matches!(params.class, KeyClass::OperatorProtected) {
            body.insert(
                "confirm_operator_protected".to_string(),
                serde_json::Value::Bool(true),
            );
        }
        if let Some(uid) = params.canton_user_id {
            body.insert("canton_user_id".to_string(), serde_json::Value::String(uid));
        }
        if let Some(b) = params.auto_provision_canton {
            body.insert(
                "auto_provision_canton".to_string(),
                serde_json::Value::Bool(b),
            );
        }
        self.rpc
            .call("tenzro_createApiKey", serde_json::Value::Object(body))
            .await
    }

    /// List every API key the node has issued — active and revoked.
    /// Admin-token-gated.
    pub async fn list(&self) -> SdkResult<ApiKeyList> {
        self.rpc
            .call("tenzro_listApiKeys", serde_json::json!({}))
            .await
    }

    /// Revoke an API key by its non-secret `key_id`. Admin-token-gated.
    ///
    /// Fails with `-32004` if the target is an `operator_protected`
    /// key (those cannot be revoked via RPC, by anyone, including an
    /// admin). Rotate that class by updating the operator secret +
    /// restart.
    pub async fn revoke(&self, key_id: &str) -> SdkResult<RevokeApiKeyResult> {
        self.rpc
            .call(
                "tenzro_revokeApiKey",
                serde_json::json!({ "key_id": key_id }),
            )
            .await
    }

    // ── Subject surface (X-Tenzro-Api-Key authenticated) ─────────────

    /// List every API key belonging to the caller's own subject.
    /// Requires `TENZRO_API_KEY` in the environment.
    pub async fn list_mine(&self) -> SdkResult<MyApiKeyList> {
        self.rpc
            .call("tenzro_listMyApiKeys", serde_json::json!({}))
            .await
    }

    /// Revoke an API key belonging to the caller's own subject.
    /// Requires `TENZRO_API_KEY` in the environment.
    ///
    /// Only `subject`-class keys are eligible. The error for
    /// "no such key" and "not your key" is intentionally the same so
    /// ownership cannot be probed.
    pub async fn revoke_mine(&self, key_id: &str) -> SdkResult<RevokeApiKeyResult> {
        self.rpc
            .call(
                "tenzro_revokeMyApiKey",
                serde_json::json!({ "key_id": key_id }),
            )
            .await
    }
}

/// Parameters for [`ApiKeyClient::create`].
#[derive(Debug, Clone)]
pub struct CreateApiKeyParams {
    /// Free-form label shown in `list`.
    pub label: String,
    /// Optional subject identifier — typically a Tenzro DID. Required
    /// if the operator wants the holder to self-revoke later.
    pub subject: Option<String>,
    /// Scopes to grant. Defaults to `["canton"]` if empty.
    pub scopes: Vec<String>,
    /// Revocability class.
    pub class: KeyClass,
    /// Optional Canton User Management Service user id (e.g.
    /// `tenzro-labs@clients`). When set together with the `canton`
    /// scope, the node will automatically allocate a tenant party,
    /// create the Canton user with that primary party, and grant
    /// CanActAs — fully provisioned tenant in one operator call.
    pub canton_user_id: Option<String>,
    /// Whether to auto-provision the Canton user when
    /// `canton_user_id` is set. Defaults to `true` server-side;
    /// pass `Some(false)` to opt out.
    pub auto_provision_canton: Option<bool>,
}

impl CreateApiKeyParams {
    /// Convenience constructor for a default `Subject`-class key with
    /// `canton` scope.
    pub fn subject(label: impl Into<String>, subject_did: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            subject: Some(subject_did.into()),
            scopes: vec!["canton".to_string()],
            class: KeyClass::Subject,
            canton_user_id: None,
            auto_provision_canton: None,
        }
    }

    /// Convenience constructor for a tenant key bound to a Canton
    /// user id. Auto-provisions the Canton user + party + rights when
    /// the operator submits this on a canton-enabled node.
    pub fn tenant_canton(
        label: impl Into<String>,
        subject_did: impl Into<String>,
        canton_user_id: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            subject: Some(subject_did.into()),
            scopes: vec!["canton".to_string()],
            class: KeyClass::Subject,
            canton_user_id: Some(canton_user_id.into()),
            auto_provision_canton: None,
        }
    }
}

/// Key class — controls who can revoke the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyClass {
    /// Default. Subject can self-revoke via `revoke_mine`; admin can
    /// revoke via `revoke`.
    Subject,
    /// Operator-only ops key. Admin can revoke; subject path does not
    /// apply.
    OperatorInternal,
    /// Operator-only locked-down key. Not revokable via RPC by anyone
    /// (including admin). Rotate by updating the operator secret +
    /// restarting the node.
    OperatorProtected,
}

impl KeyClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Subject => "subject",
            Self::OperatorInternal => "operator_internal",
            Self::OperatorProtected => "operator_protected",
        }
    }
}

/// Response from [`ApiKeyClient::create`]. The `key` field is the
/// plaintext `tnz_...` token and is shown exactly once — persist it
/// immediately.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreatedApiKey {
    pub key: String,
    pub key_id: String,
    pub label: String,
    #[serde(default)]
    pub subject: Option<String>,
    pub scopes: Vec<String>,
    #[serde(default)]
    pub class: Option<String>,
    pub created_at: i64,
    /// Bound Canton User Management Service user id, if any.
    #[serde(default)]
    pub canton_user_id: Option<String>,
    /// FQ party id (`<hint>::<participant-hash>`) auto-provisioned for
    /// this user.
    #[serde(default)]
    pub canton_primary_party: Option<String>,
    /// Stage 2.b: Canton IdentityProviderConfig id auto-registered
    /// for this tenant when the node is configured with a
    /// tenant-IdP provisioner. The party + user live under this IDP.
    #[serde(default)]
    pub canton_identity_provider_id: Option<String>,
    /// Summary of the Canton provision step.
    #[serde(default)]
    pub canton_provisioning: Option<CantonProvisioningSummary>,
    /// Stage 2.b: per-tenant OAuth2 client minted upstream, returned
    /// exactly once. The `client_secret` is the tenant's responsibility
    /// to persist; the Tenzro node does not store it.
    #[serde(default)]
    pub tenant_oauth_client: Option<TenantOAuthClient>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Stage 2.b: per-tenant OAuth2 client minted upstream by the
/// Tenzro node at API-key issuance time. Returned once.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TenantOAuthClient {
    pub client_id: String,
    pub client_secret: String,
    pub token_url: String,
    pub issuer_url: String,
    pub jwks_url: String,
    pub audience: String,
}

/// Summary of the Canton auto-provision step when `canton_user_id`
/// is bound on `create`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CantonProvisioningSummary {
    /// `provisioned` for a fresh provision, `already_exists` for
    /// idempotent reissue.
    pub status: String,
    pub user_id: String,
    #[serde(default)]
    pub primary_party: Option<String>,
    #[serde(default)]
    pub party_hint: Option<String>,
    #[serde(default)]
    pub rights_granted: Option<Vec<String>>,
}

/// One row of the keyring as returned by `list` / `list_mine`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiKeyRecord {
    pub key_id: String,
    #[serde(default)]
    pub subject: Option<String>,
    pub label: String,
    pub scopes: Vec<String>,
    #[serde(default)]
    pub class: Option<String>,
    pub created_at: i64,
    #[serde(default)]
    pub revoked_at: Option<i64>,
    pub active: bool,
}

/// Response from [`ApiKeyClient::list`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiKeyList {
    pub keys: Vec<ApiKeyRecord>,
}

/// Response from [`ApiKeyClient::list_mine`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MyApiKeyList {
    pub keys: Vec<ApiKeyRecord>,
    pub subject: String,
}

/// Response from [`ApiKeyClient::revoke`] / [`ApiKeyClient::revoke_mine`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevokeApiKeyResult {
    pub key_id: String,
    pub revoked: bool,
}
