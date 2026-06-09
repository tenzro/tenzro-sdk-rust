//! TEE-attested clock client.
//!
//! Returns the canonical `AttestedTimestamp` envelope (wall_ms +
//! monotonic_ns + tee_vendor metadata) used by long-running
//! multi-party workflows that cannot trust any single replica's
//! wall-clock.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct AttestedClockClient {
    rpc: Arc<RpcClient>,
}

impl AttestedClockClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Return the current node wall-clock as an `AttestedTimestamp`
    /// envelope. When the node is running inside a TEE the envelope
    /// carries vendor attestation; otherwise `tee_vendor` is `null`
    /// and the relying party MUST reject the envelope for production
    /// mandate / deadline use.
    pub async fn now(&self) -> SdkResult<AttestedTimestampEnvelope> {
        self.rpc
            .call("tenzro_attestedClockNow", serde_json::Value::Null)
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestedTimestampEnvelope {
    pub wall_ms: u64,
    pub monotonic_ns: u64,
    pub tee_vendor: Option<String>,
    pub note: Option<String>,
}
