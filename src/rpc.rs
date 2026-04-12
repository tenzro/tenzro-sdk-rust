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

/// JSON-RPC client for communicating with a Tenzro node
#[derive(Clone)]
pub struct RpcClient {
    http: reqwest::Client,
    endpoint: String,
    request_id: Arc<AtomicU64>,
}

impl RpcClient {
    /// Creates a new RPC client
    pub fn new(endpoint: &str, timeout: Duration) -> SdkResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| SdkError::ConnectionError(e.to_string()))?;

        Ok(Self {
            http,
            endpoint: endpoint.to_string(),
            request_id: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Calls a JSON-RPC method with the given parameters
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

        let response = self
            .http
            .post(&self.endpoint)
            .json(&request)
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

        let rpc_response: RpcResponse<T> = response.json().await.map_err(|e| {
            SdkError::RpcError(format!("Failed to parse response: {}", e))
        })?;

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

    /// Makes an HTTP GET request to the Web API (non-RPC)
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> SdkResult<T> {
        let base = self.endpoint.trim_end_matches('/');
        // Derive web API base from RPC endpoint
        // RPC is on :8545, Web API is on :8080
        // For remote endpoints, use api.tenzro.network instead of rpc.tenzro.network
        let url = if base.contains("rpc.tenzro.network") {
            format!(
                "{}{}",
                base.replace("rpc.tenzro.network", "api.tenzro.network"),
                path
            )
        } else if base.contains("localhost:8545") || base.contains("127.0.0.1:8545") {
            format!("{}{}", base.replace("8545", "8080"), path)
        } else {
            format!("{}{}", base, path)
        };

        let response = self.http.get(&url).send().await.map_err(|e| {
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

    /// Makes an HTTP POST request to the Web API (non-RPC)
    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> SdkResult<T> {
        let base = self.endpoint.trim_end_matches('/');
        let url = if base.contains("rpc.tenzro.network") {
            format!(
                "{}{}",
                base.replace("rpc.tenzro.network", "api.tenzro.network"),
                path
            )
        } else if base.contains("localhost:8545") || base.contains("127.0.0.1:8545") {
            format!("{}{}", base.replace("8545", "8080"), path)
        } else {
            format!("{}{}", base, path)
        };

        let response = self
            .http
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

    /// Returns the RPC endpoint URL
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}
