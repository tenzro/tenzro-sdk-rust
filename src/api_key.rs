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

        // Delegation bundle — emit fields the node parses verbatim.
        // Empty arrays are dropped so we don't ship noise on wire.
        if let Some(d) = params.delegation {
            let push_str_array = |body: &mut serde_json::Map<String, serde_json::Value>, key: &str, v: Vec<String>| {
                if !v.is_empty() {
                    body.insert(
                        key.to_string(),
                        serde_json::Value::Array(v.into_iter().map(serde_json::Value::String).collect()),
                    );
                }
            };
            push_str_array(&mut body, "can_act_as_parties", d.can_act_as_parties);
            push_str_array(&mut body, "can_read_as_parties", d.can_read_as_parties);
            push_str_array(&mut body, "allowed_templates", d.allowed_templates);
            push_str_array(&mut body, "allowed_commands", d.allowed_commands);
            push_str_array(&mut body, "requires_mandate_for", d.requires_mandate_for);
            push_str_array(&mut body, "allowed_tools", d.allowed_tools);
            push_str_array(&mut body, "allowed_skills", d.allowed_skills);
            push_str_array(&mut body, "allowed_knowledge", d.allowed_knowledge);
            push_str_array(
                &mut body,
                "allowed_workflow_templates",
                d.allowed_workflow_templates,
            );
            push_str_array(
                &mut body,
                "allowed_agent_templates",
                d.allowed_agent_templates,
            );
            push_str_array(&mut body, "allowed_models", d.allowed_models);
            if let Some(v) = d.max_per_command_amulet {
                body.insert(
                    "max_per_command_amulet".to_string(),
                    serde_json::Value::String(v.to_string()),
                );
            }
            if let Some(v) = d.max_per_day_amulet {
                body.insert(
                    "max_per_day_amulet".to_string(),
                    serde_json::Value::String(v.to_string()),
                );
            }
            if let Some(v) = d.valid_until {
                body.insert(
                    "valid_until".to_string(),
                    serde_json::Value::Number(v.into()),
                );
            }
            if !d.max_per_resource_tnzo.is_empty() {
                let mut obj = serde_json::Map::new();
                for (rid, v) in d.max_per_resource_tnzo {
                    obj.insert(rid, serde_json::Value::String(v.to_string()));
                }
                body.insert(
                    "max_per_resource_tnzo".to_string(),
                    serde_json::Value::Object(obj),
                );
            }
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
#[derive(Debug, Clone, Default)]
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
    /// Optional agent delegation bundle. Activates per-party,
    /// per-template, per-command, per-resource authorization on
    /// every RPC presenting this key. `None` = legacy unrestricted
    /// behaviour.
    pub delegation: Option<AgentDelegationParams>,
}

/// Per-resource-class allow-lists and ceilings for a tenant key.
/// All fields default-empty; an empty field is "no restriction in
/// that class". A non-empty field activates the gate at the matching
/// invocation handler.
#[derive(Debug, Clone, Default)]
pub struct AgentDelegationParams {
    /// Canton FQ party ids the key is allowed to act for.
    pub can_act_as_parties: Vec<String>,
    /// Canton FQ party ids the key is allowed to observe.
    pub can_read_as_parties: Vec<String>,
    /// Canton DAML template ids the key may query / submit against.
    pub allowed_templates: Vec<String>,
    /// Canton DAML command shapes the key may exercise.
    pub allowed_commands: Vec<String>,
    /// Canton command shapes that require an AP2 cart mandate.
    pub requires_mandate_for: Vec<String>,
    /// Per-command Canton value ceiling in amulet smallest units.
    pub max_per_command_amulet: Option<u128>,
    /// Rolling-day Canton cumulative value ceiling.
    pub max_per_day_amulet: Option<u128>,
    /// Unix timestamp (seconds) after which the key is rejected.
    pub valid_until: Option<i64>,
    /// Tool / MCP resource_ids the key may invoke.
    pub allowed_tools: Vec<String>,
    /// Skill ids the key may invoke.
    pub allowed_skills: Vec<String>,
    /// Knowledge resource_ids the key may query.
    pub allowed_knowledge: Vec<String>,
    /// Workflow template_ids the key may instantiate.
    pub allowed_workflow_templates: Vec<String>,
    /// Agent template_ids the key may instantiate.
    pub allowed_agent_templates: Vec<String>,
    /// Model_ids the key may call for inference.
    pub allowed_models: Vec<String>,
    /// Per-resource TNZO ceiling override
    /// (`resource_id → atto-TNZO`).
    pub max_per_resource_tnzo: std::collections::BTreeMap<String, u128>,
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
            delegation: None,
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
            delegation: None,
        }
    }

    /// Attach a delegation bundle. Activates per-resource-class
    /// authorization at every gated invocation handler.
    pub fn with_delegation(mut self, delegation: AgentDelegationParams) -> Self {
        self.delegation = Some(delegation);
        self
    }
}

impl Default for KeyClass {
    fn default() -> Self {
        Self::Subject
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
    /// Stage 2.b: non-secret metadata about the per-tenant OAuth2
    /// client minted upstream. The credentials stay on the node,
    /// which mints and forwards the tenant's Canton JWT internally
    /// on every canton-scoped call — the `tnz_...` API key is the
    /// tenant's only credential.
    #[serde(default)]
    pub tenant_oauth_client: Option<TenantOAuthClient>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Stage 2.b: non-secret per-tenant OAuth2 client metadata minted
/// upstream by the Tenzro node at API-key issuance time.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TenantOAuthClient {
    pub client_id: String,
    pub issuer_url: String,
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
