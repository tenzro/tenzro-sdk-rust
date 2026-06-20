//! JSON-RPC 2.0 client for Tenzro Network
//!
//! Provides the low-level transport for all SDK calls to a Tenzro node.

use crate::error::{SdkError, SdkResult};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    params: serde_json::Value,
    id: u64,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<T>,
    error: Option<RpcError>,
    #[allow(dead_code)]
    id: u64,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

/// JSON-RPC client for communicating with a Tenzro node.
///
/// Defaults to the HTTP backend (`reqwest` against an `http(s)://...`
/// endpoint). When the `embedded` cargo feature is enabled, the client
/// can also be constructed with an `Arc<TenzroNode>` and dispatches
/// calls in-process via `tenzro_node::dispatch_embedded` — same gates,
/// same handler chain, no network hop.
#[derive(Clone)]
pub struct RpcClient {
    backend: RpcBackend,
    request_id: Arc<AtomicU64>,
}

/// Transport backend for [`RpcClient`].
#[derive(Clone)]
enum RpcBackend {
    /// HTTP JSON-RPC against a remote node. The default.
    Http {
        http: reqwest::Client,
        endpoint: String,
    },
    /// In-process JSON-RPC against an embedded node. Available behind
    /// the `embedded` cargo feature.
    #[cfg(feature = "embedded")]
    Embedded {
        node: Arc<tenzro_node::TenzroNode>,
    },
}

impl RpcClient {
    /// Creates a new HTTP RPC client.
    pub fn new(endpoint: &str, timeout: Duration) -> SdkResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| SdkError::ConnectionError(e.to_string()))?;

        Ok(Self {
            backend: RpcBackend::Http {
                http,
                endpoint: endpoint.to_string(),
            },
            request_id: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Creates a new in-process RPC client that dispatches directly
    /// against the given embedded [`tenzro_node::TenzroNode`]. The
    /// HTTP-equivalent gate + dispatch + analytics pipeline runs as a
    /// plain function call — no localhost port, no IPC overhead, the
    /// same authorization semantics the HTTP endpoint enforces.
    #[cfg(feature = "embedded")]
    pub fn embedded(node: Arc<tenzro_node::TenzroNode>) -> Self {
        Self {
            backend: RpcBackend::Embedded { node },
            request_id: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Calls a JSON-RPC method with the given parameters. Dispatches
    /// to whichever backend the client was constructed with — HTTP by
    /// default, or in-process via `tenzro_node::dispatch_embedded` when
    /// the `embedded` cargo feature is enabled and the client was
    /// constructed with [`Self::embedded`].
    pub async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> SdkResult<T> {
        let id = self.request_id.fetch_add(1, Ordering::Relaxed);

        let request = RpcRequest {
            jsonrpc: "2.0",
            method,
            params,
            id,
        };

        tracing::debug!("RPC call: {} (id={})", method, id);

        let response_value: serde_json::Value = match &self.backend {
            RpcBackend::Http { http, endpoint } => {
                dispatch_http(http, endpoint, &request).await?
            }
            #[cfg(feature = "embedded")]
            RpcBackend::Embedded { node } => dispatch_embedded(node, &request).await,
        };

        let rpc_response: RpcResponse<T> = serde_json::from_value(response_value)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse response: {}", e)))?;

        if let Some(err) = rpc_response.error {
            return Err(SdkError::RpcError(format!(
                "[{}] {}",
                err.code, err.message
            )));
        }

        rpc_response
            .result
            .ok_or_else(|| SdkError::RpcError("Response missing result field".to_string()))
    }

    /// Makes an HTTP GET request to the Web API (non-RPC).
    ///
    /// HTTP-only — the embedded backend has no Web API surface (callers
    /// in-process would dispatch the JSON-RPC equivalent via
    /// [`Self::call`], or reach into the node's web server directly).
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> SdkResult<T> {
        let (http, endpoint) = self.require_http()?;
        let url = web_api_url(endpoint, path);
        let response = http.get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                SdkError::Timeout
            } else {
                SdkError::ConnectionError(e.to_string())
            }
        })?;

        if !response.status().is_success() {
            return Err(SdkError::RpcError(format!(
                "HTTP {}: {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        response
            .json()
            .await
            .map_err(|e| SdkError::RpcError(format!("Failed to parse response: {}", e)))
    }

    /// Makes an HTTP POST request to the Web API (non-RPC). HTTP-only;
    /// see [`Self::get`].
    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> SdkResult<T> {
        let (http, endpoint) = self.require_http()?;
        let url = web_api_url(endpoint, path);
        let response = http
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    SdkError::Timeout
                } else {
                    SdkError::ConnectionError(e.to_string())
                }
            })?;

        if !response.status().is_success() {
            return Err(SdkError::RpcError(format!(
                "HTTP {}: {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        response
            .json()
            .await
            .map_err(|e| SdkError::RpcError(format!("Failed to parse response: {}", e)))
    }

    /// Makes an authenticated HTTP POST to the Web API with caller-supplied
    /// DPoP-bound JWT and DPoP proof headers (RFC 9449).
    ///
    /// Unlike the JSON-RPC `call()` path, web-API endpoints in the
    /// `/wallet/*` family require a fresh DPoP proof per request that
    /// signs over the exact `(method, htu)` pair. Because the caller
    /// (wallet kernel) is the only party who can produce that proof,
    /// the SDK cannot infer it from environment ambient state — it
    /// must be passed in. HTTP-only.
    pub async fn post_with_auth<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        bearer_jwt: &str,
        dpop_proof: &str,
    ) -> SdkResult<T> {
        let (http, endpoint) = self.require_http()?;
        let url = web_api_url(endpoint, path);
        let response = http
            .post(&url)
            .header("Authorization", format!("DPoP {}", bearer_jwt))
            .header("DPoP", dpop_proof)
            .json(body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    SdkError::Timeout
                } else {
                    SdkError::ConnectionError(e.to_string())
                }
            })?;

        if !response.status().is_success() {
            return Err(SdkError::RpcError(format!(
                "HTTP {}: {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }
        response
            .json()
            .await
            .map_err(|e| SdkError::RpcError(format!("Failed to parse response: {}", e)))
    }

    /// Authenticated counterpart to [`Self::get`] — see
    /// [`Self::post_with_auth`] for the rationale. HTTP-only.
    pub async fn get_with_auth<T: DeserializeOwned>(
        &self,
        path: &str,
        bearer_jwt: &str,
        dpop_proof: &str,
    ) -> SdkResult<T> {
        let (http, endpoint) = self.require_http()?;
        let url = web_api_url(endpoint, path);
        let response = http
            .get(&url)
            .header("Authorization", format!("DPoP {}", bearer_jwt))
            .header("DPoP", dpop_proof)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    SdkError::Timeout
                } else {
                    SdkError::ConnectionError(e.to_string())
                }
            })?;

        if !response.status().is_success() {
            return Err(SdkError::RpcError(format!(
                "HTTP {}: {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }
        response
            .json()
            .await
            .map_err(|e| SdkError::RpcError(format!("Failed to parse response: {}", e)))
    }

    /// Build the Web API base URL by mapping the configured RPC
    /// endpoint to its sibling Web API host. HTTP-only.
    pub fn web_api_url(&self, path: &str) -> SdkResult<String> {
        let (_http, endpoint) = self.require_http()?;
        Ok(web_api_url(endpoint, path))
    }

    /// Returns the RPC endpoint URL for the HTTP backend, or `None`
    /// when this client was constructed with the embedded backend.
    pub fn endpoint(&self) -> Option<&str> {
        match &self.backend {
            RpcBackend::Http { endpoint, .. } => Some(endpoint),
            #[cfg(feature = "embedded")]
            RpcBackend::Embedded { .. } => None,
        }
    }

    /// Extract the HTTP backend pieces or return an error explaining the
    /// embedded mode doesn't support this surface.
    fn require_http(&self) -> SdkResult<(&reqwest::Client, &str)> {
        match &self.backend {
            RpcBackend::Http { http, endpoint } => Ok((http, endpoint.as_str())),
            #[cfg(feature = "embedded")]
            RpcBackend::Embedded { .. } => Err(SdkError::ConnectionError(
                "Web API surface is HTTP-only; not available on the embedded backend"
                    .to_string(),
            )),
        }
    }
}

/// Sibling-subdomain mapping for the Web API base URL. The production
/// deploy serves `rpc.tenzro.network` and `api.tenzro.network` from the
/// same node — `:8545` is JSON-RPC, `:8080` is the Web API surface.
fn web_api_url(endpoint: &str, path: &str) -> String {
    let base = endpoint.trim_end_matches('/');
    if base.contains("rpc.tenzro.network") {
        format!(
            "{}{}",
            base.replace("rpc.tenzro.network", "api.tenzro.network"),
            path
        )
    } else if base.contains("localhost:8545") || base.contains("127.0.0.1:8545") {
        format!("{}{}", base.replace("8545", "8080"), path)
    } else {
        format!("{}{}", base, path)
    }
}

/// HTTP backend JSON-RPC dispatch. Pulls ambient auth headers from
/// `TENZRO_BEARER_JWT` / `TENZRO_DPOP_PROOF` / `TENZRO_API_KEY` /
/// `TENZRO_ADMIN_TOKEN` env vars the same way the CLI does
/// (`crates/tenzro-cli/src/rpc.rs`).
async fn dispatch_http(
    http: &reqwest::Client,
    endpoint: &str,
    request: &RpcRequest<'_>,
) -> SdkResult<serde_json::Value> {
    let mut req = http.post(endpoint).json(request);
    if let Ok(bearer) = std::env::var("TENZRO_BEARER_JWT")
        && !bearer.is_empty()
    {
        req = req.header("Authorization", format!("DPoP {}", bearer));
    }
    if let Ok(dpop) = std::env::var("TENZRO_DPOP_PROOF")
        && !dpop.is_empty()
    {
        req = req.header("DPoP", dpop);
    }
    if let Ok(key) = std::env::var("TENZRO_API_KEY")
        && !key.is_empty()
    {
        req = req.header("X-Tenzro-Api-Key", key);
    }
    if let Ok(token) = std::env::var("TENZRO_ADMIN_TOKEN")
        && !token.is_empty()
    {
        req = req.header("X-Tenzro-Admin-Token", token);
    }

    let response = req
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                SdkError::Timeout
            } else {
                SdkError::ConnectionError(e.to_string())
            }
        })?;

    if !response.status().is_success() {
        return Err(SdkError::RpcError(format!(
            "HTTP {}: {}",
            response.status(),
            response.status().canonical_reason().unwrap_or("Unknown")
        )));
    }

    response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| SdkError::RpcError(format!("Failed to parse response: {}", e)))
}

/// Embedded backend dispatch via [`tenzro_node::dispatch_embedded`].
/// Ambient auth env vars are forwarded into [`tenzro_node::EmbeddedAuth`]
/// so embedded callers get the same authorization semantics as the HTTP
/// path — no shortcut around admin-token / API-key / DPoP gates.
#[cfg(feature = "embedded")]
async fn dispatch_embedded(
    node: &Arc<tenzro_node::TenzroNode>,
    request: &RpcRequest<'_>,
) -> serde_json::Value {
    let auth = tenzro_node::EmbeddedAuth {
        authorization: std::env::var("TENZRO_BEARER_JWT")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|s| format!("DPoP {}", s)),
        dpop: std::env::var("TENZRO_DPOP_PROOF").ok().filter(|s| !s.is_empty()),
        admin_token: std::env::var("TENZRO_ADMIN_TOKEN").ok().filter(|s| !s.is_empty()),
        api_key: std::env::var("TENZRO_API_KEY").ok().filter(|s| !s.is_empty()),
        ..Default::default()
    };
    let payload = serde_json::to_value(request).unwrap_or(serde_json::Value::Null);
    tenzro_node::dispatch_embedded(node, payload, auth).await
}
