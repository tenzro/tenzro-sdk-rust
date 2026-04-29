//! OAuth 2.1 + DPoP onboarding helpers for Tenzro Network.
//!
//! Onboarding uses OAuth 2.1 (RFC 6749 successor) + DPoP-bound JWTs
//! (RFC 9449) + Rich Authorization Requests (RFC 9396). Participants —
//! humans, delegated agents under a human controller, and fully
//! autonomous agents — onboard via the three RPCs exposed here. Each
//! call provisions a TDIP identity (+ MPC wallet) and returns a JWT
//! bound to a holder-supplied DPoP `jkt` (RFC 7638 thumbprint of the
//! holder's Ed25519 public key).
//!
//! Subsequent privileged calls (sign + send transaction, escrow create,
//! release/refund, etc.) authenticate by sending the JWT in the
//! `Authorization: Bearer <jwt>` header alongside a per-request DPoP proof
//! in the `DPoP` header (when the token is DPoP-bound). The SDK forwards
//! both headers automatically when the `TENZRO_BEARER_JWT` and
//! `TENZRO_DPOP_PROOF` environment variables are set — see
//! [`crate::rpc::RpcClient`] for the transport-level wiring.
//!
//! # Example
//!
//! ```no_run
//! # use tenzro_sdk::{TenzroClient, config::SdkConfig};
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let client = TenzroClient::connect(SdkConfig::testnet()).await?;
//! let auth = client.auth();
//! let session = auth.onboard_human("Alice", None).await?;
//! println!("DID:    {}", session.identity["did"]);
//! println!("Wallet: {}", session.wallet["address"]);
//! println!("Token:  {}…", &session.access_token[..32]);
//! # Ok(()) }
//! ```

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Client for OAuth 2.1 onboarding RPCs.
///
/// Construct via [`crate::TenzroClient::auth`].
#[derive(Clone)]
pub struct AuthClient {
    rpc: Arc<RpcClient>,
}

impl AuthClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Onboard a new **human** participant — provisions a TDIP `did:tenzro:human:*`
    /// identity, a fresh MPC wallet, and returns an OAuth 2.1 access token.
    ///
    /// # Arguments
    ///
    /// * `display_name` — human-readable label surfaced in approver UIs.
    /// * `dpop_jkt` — optional RFC 7638 JWK thumbprint of the holder's Ed25519
    ///   public key. If supplied, the issued JWT is DPoP-bound to that key
    ///   and every subsequent privileged call must accompany the bearer with
    ///   a fresh DPoP proof signed by the same key. Strongly recommended.
    pub async fn onboard_human(
        &self,
        display_name: &str,
        dpop_jkt: Option<&str>,
    ) -> SdkResult<OnboardSession> {
        let mut params = serde_json::json!({ "display_name": display_name });
        if let Some(jkt) = dpop_jkt {
            params["dpop_jkt"] = serde_json::Value::String(jkt.to_string());
        }
        self.rpc.call("tenzro_onboardHuman", params).await
    }

    /// Onboard a **delegated agent** that acts on behalf of an existing
    /// `controller_did` (typically a human). The agent inherits the
    /// controller's act-chain and is bounded by `delegation_scope`.
    ///
    /// Revoking the controller DID via [`Self::revoke_did`] cascades and
    /// invalidates this agent's token automatically.
    pub async fn onboard_delegated_agent(
        &self,
        controller_did: &str,
        capabilities: Vec<String>,
        delegation_scope: serde_json::Value,
        dpop_jkt: Option<&str>,
    ) -> SdkResult<OnboardSession> {
        let mut params = serde_json::json!({
            "controller_did": controller_did,
            "capabilities": capabilities,
            "delegation_scope": delegation_scope,
        });
        if let Some(jkt) = dpop_jkt {
            params["dpop_jkt"] = serde_json::Value::String(jkt.to_string());
        }
        self.rpc
            .call("tenzro_onboardDelegatedAgent", params)
            .await
    }

    /// Onboard a **fully autonomous agent**. Unlike a delegated agent, this
    /// has no human controller — instead the agent must post a TNZO bond
    /// (slashable on misbehaviour) at `bond_funding_address` before
    /// onboarding succeeds.
    pub async fn onboard_autonomous_agent(
        &self,
        bond_funding_address: &str,
        dpop_jkt: Option<&str>,
    ) -> SdkResult<OnboardSession> {
        let mut params = serde_json::json!({
            "bond_funding_address": bond_funding_address,
        });
        if let Some(jkt) = dpop_jkt {
            params["dpop_jkt"] = serde_json::Value::String(jkt.to_string());
        }
        self.rpc
            .call("tenzro_onboardAutonomousAgent", params)
            .await
    }

    /// Exchange a long-lived refresh token for a fresh access token. Mirrors
    /// OAuth 2.1 `grant_type=refresh_token`. Refresh tokens are opaque UUIDs
    /// with a 30-day TTL; access tokens are HS256 JWTs with a 1-hour TTL.
    ///
    /// If `dpop_jkt` is supplied, the new access token is DPoP-bound to that
    /// thumbprint. The refresh token itself is **not** rotated in V1.
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
        dpop_jkt: Option<&str>,
    ) -> SdkResult<RefreshedToken> {
        let mut params = serde_json::json!({ "refresh_token": refresh_token });
        if let Some(jkt) = dpop_jkt {
            params["dpop_jkt"] = serde_json::Value::String(jkt.to_string());
        }
        self.rpc.call("tenzro_refreshToken", params).await
    }

    /// Mint a fresh access + refresh token pair against an existing MPC
    /// wallet. Useful when the holder already provisioned a wallet via
    /// `tenzro_createWallet` and now wants OAuth-style auth credentials
    /// without re-running the full onboarding flow.
    ///
    /// Returns the same shape as the three onboard variants —
    /// `OnboardSession` — so it slots into existing session-management code.
    pub async fn link_wallet_for_auth(
        &self,
        wallet_id: &str,
        dpop_jkt: Option<&str>,
        display_name: Option<&str>,
        ttl_secs: Option<u64>,
    ) -> SdkResult<OnboardSession> {
        let mut params = serde_json::json!({ "wallet_id": wallet_id });
        if let Some(jkt) = dpop_jkt {
            params["dpop_jkt"] = serde_json::Value::String(jkt.to_string());
        }
        if let Some(name) = display_name {
            params["display_name"] = serde_json::Value::String(name.to_string());
        }
        if let Some(ttl) = ttl_secs {
            params["ttl_secs"] = serde_json::Value::Number(ttl.into());
        }
        self.rpc.call("tenzro_linkWalletForAuth", params).await
    }

    /// Revoke a single JWT by its `jti` claim. The token is added to the
    /// engine's revocation set and any subsequent validation fails.
    pub async fn revoke_jwt(&self, jti: &str, reason: Option<&str>) -> SdkResult<RevokeResponse> {
        let params = serde_json::json!({
            "jti": jti,
            "reason": reason.unwrap_or("revoked via SDK"),
        });
        self.rpc.call("tenzro_revokeJwt", params).await
    }

    /// Revoke an entire identity by DID. Every JWT minted under this DID
    /// (and every descendant DID in the act-chain) is invalidated
    /// transitively.
    pub async fn revoke_did(&self, did: &str, reason: Option<&str>) -> SdkResult<RevokeResponse> {
        let params = serde_json::json!({
            "did": did,
            "reason": reason.unwrap_or("revoked via SDK"),
        });
        self.rpc.call("tenzro_revokeDid", params).await
    }

    /// List approvals in `Pending` status for the given approver DID.
    /// Returns the records the approver should review and decide on.
    pub async fn list_pending_approvals(
        &self,
        approver_did: &str,
    ) -> SdkResult<PendingApprovals> {
        let params = serde_json::json!({ "approver_did": approver_did });
        self.rpc.call("tenzro_listPendingApprovals", params).await
    }

    /// Decide a pending approval — either `"approved"` or `"denied"`. Only
    /// the recorded approver DID may decide; mismatched approvers are
    /// rejected with JSON-RPC error code `-32001` (forbidden).
    pub async fn decide_approval(
        &self,
        approval_id: &str,
        decision: &str,
        approver_did: &str,
    ) -> SdkResult<ApprovalDecision> {
        let params = serde_json::json!({
            "approval_id": approval_id,
            "decision": decision,
            "approver_did": approver_did,
        });
        self.rpc.call("tenzro_decideApproval", params).await
    }
}

/// One of the three onboarding RPCs (or `link_wallet_for_auth`) returns
/// this session bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardSession {
    /// Provisioned TDIP identity record.
    pub identity: serde_json::Value,
    /// Provisioned MPC wallet record (id + address).
    pub wallet: serde_json::Value,
    /// OAuth 2.1 access token (HS256 JWT, optionally DPoP-bound). Send as
    /// `Authorization: Bearer <token>` on subsequent privileged calls.
    /// When DPoP-bound, also send a fresh `DPoP: <proof>` header.
    pub access_token: String,
    /// Always `"Bearer"` (RFC 6750 token type, even though DPoP-bound).
    #[serde(default)]
    pub token_type: String,
    /// Access-token lifetime in seconds (default 3600).
    #[serde(default)]
    pub expires_in: u64,
    /// Long-lived refresh token (opaque UUID, 30-day TTL). Exchange via
    /// [`AuthClient::refresh_token`] when the access token expires. Treat
    /// as a secret — leakage allows minting access tokens until revocation.
    #[serde(default)]
    pub refresh_token: String,
    /// Refresh-token lifetime in seconds (default 30 days).
    #[serde(default)]
    pub refresh_token_expires_in: u64,
    /// `true` iff the access token requires a DPoP proof on every call.
    #[serde(default)]
    pub dpop_bound: bool,
    /// RFC 9396 Rich Authorization Request payload echoed back, describing
    /// the act-chain and capabilities the token is authorized for.
    #[serde(default)]
    pub authorization_details: serde_json::Value,
}

/// Result of [`AuthClient::refresh_token`]. The refresh token is **not**
/// rotated in V1 — only the access token changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshedToken {
    /// New access-token JWT.
    pub access_token: String,
    /// Always `"Bearer"`.
    #[serde(default)]
    pub token_type: String,
    /// Access-token lifetime in seconds.
    #[serde(default)]
    pub expires_in: u64,
    /// `true` iff the new access token is DPoP-bound (i.e., the request
    /// supplied `dpop_jkt` and the engine encoded a `cnf.jkt` claim).
    #[serde(default)]
    pub dpop_bound: bool,
}

/// Result of `revoke_jwt` / `revoke_did`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeResponse {
    /// Engine status string — typically `"revoked"`.
    #[serde(default)]
    pub status: String,
    /// Number of JTIs invalidated by this call (>1 indicates cascade).
    #[serde(default)]
    pub affected_jti_count: u64,
}

/// Result of `list_pending_approvals`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApprovals {
    /// Number of pending records returned.
    #[serde(default)]
    pub count: u64,
    /// The records themselves — opaque JSON to keep the SDK decoupled
    /// from `tenzro-auth` storage internals.
    #[serde(default)]
    pub pending: Vec<serde_json::Value>,
}

/// Result of `decide_approval`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecision {
    /// New status — `"Approved"` or `"Denied"`.
    #[serde(default)]
    pub status: String,
    /// Echo of the approval id.
    #[serde(default)]
    pub approval_id: String,
}
