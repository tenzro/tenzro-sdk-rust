//! MCP plugin host SDK — operator-only credential vault management.
//!
//! These RPCs are admin-token-gated. Tenants never see them. The
//! operator uses them to populate the sealed credential vault that
//! the plugin host references at MCP invocation time.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct McpHostClient {
    rpc: Arc<RpcClient>,
}

impl McpHostClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Store an upstream secret in the sealed credential vault.
    /// `sealed_secret_ref` is an opaque label the operator picks;
    /// it's referenced by MCP tool registrations via
    /// `upstream_auth.sealed_secret_ref`.
    pub async fn store_secret(&self, sealed_secret_ref: &str, plaintext: &str) -> SdkResult<StoreSecretResponse> {
        self.rpc
            .call(
                "tenzro_storeMcpSecret",
                serde_json::json!({
                    "sealed_secret_ref": sealed_secret_ref,
                    "plaintext": plaintext,
                }),
            )
            .await
    }

    /// Remove a secret from the vault. Idempotent.
    pub async fn forget_secret(&self, sealed_secret_ref: &str) -> SdkResult<ForgetSecretResponse> {
        self.rpc
            .call(
                "tenzro_forgetMcpSecret",
                serde_json::json!({"sealed_secret_ref": sealed_secret_ref}),
            )
            .await
    }

    /// Evict a persistent stdio MCP subprocess. The next invocation
    /// will respawn it. Use after rotating an upstream credential or
    /// when an operator needs to force a clean restart.
    pub async fn evict_subprocess(&self, tool_id: &str) -> SdkResult<EvictSubprocessResponse> {
        self.rpc
            .call(
                "tenzro_evictMcpSubprocess",
                serde_json::json!({"tool_id": tool_id}),
            )
            .await
    }
}

/// Upstream auth descriptor for MCP tool registration. Mirrors the
/// node-side `UpstreamAuth` enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpstreamAuth {
    /// `Authorization: Bearer <secret>` on outbound HTTP.
    Bearer { sealed_secret_ref: String },
    /// `<header_name>: <secret>` on outbound HTTP.
    Header {
        header_name: String,
        sealed_secret_ref: String,
    },
    /// `<env_var_name> = <secret>` in subprocess environment.
    EnvVar {
        env_var_name: String,
        sealed_secret_ref: String,
    },
    /// `<param_name>=<secret>` as query parameter on the endpoint URL.
    QueryParam {
        param_name: String,
        sealed_secret_ref: String,
    },
}

/// Spawn spec for stdio MCP subprocesses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StdioSpawnSpec {
    pub command: String,
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(default = "default_persistent")]
    pub persistent: bool,
}

fn default_persistent() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSecretResponse {
    pub sealed_secret_ref: String,
    pub stored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgetSecretResponse {
    pub sealed_secret_ref: String,
    pub forgotten: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvictSubprocessResponse {
    pub tool_id: String,
    pub evicted: bool,
}
