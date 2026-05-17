//! SeedAgent treasury earmark and protocol-owned bootstrap agent registry
//! SDK for Tenzro Network (Spec 10).
//!
//! The SeedAgent earmark is a genesis-funded TNZO allocation governed by
//! a decay schedule and `Charter`s that enumerate which `OperationKind`s
//! a seed agent may exercise (inference, task marketplace, bridge, etc.)
//! with per-charter spend caps, target throughput, and counterparty
//! filters.
//!
//! All endpoints here are read-only — provisioning, monthly decay, and
//! sunset wind-down land in a later wave with the off-chain provisioning
//! daemon and governance-executor mutation paths.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde_json::Value;
use std::sync::Arc;

/// SeedAgent registry client.
#[derive(Clone)]
pub struct SeedAgentClient {
    rpc: Arc<RpcClient>,
}

impl SeedAgentClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Returns the singleton `TreasuryEarmark` — genesis allocation,
    /// decay schedule, master enabled flag, and surplus-burn disposition.
    pub async fn get_treasury_earmark(&self) -> SdkResult<Value> {
        self.rpc
            .call("tenzro_getTreasuryEarmark", serde_json::json!({}))
            .await
    }

    /// Fetch a single `Charter` by its hex-encoded identifier.
    pub async fn get_seed_agent_charter(&self, charter_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_getSeedAgentCharter",
                serde_json::json!({ "charter_id": charter_id }),
            )
            .await
    }

    /// List every registered `Charter` (active and sunset).
    pub async fn list_seed_agent_charters(&self) -> SdkResult<Value> {
        self.rpc
            .call("tenzro_listSeedAgentCharters", serde_json::json!({}))
            .await
    }

    /// List provisioned `SeedAgentRecord`s, optionally filtered by charter id.
    pub async fn list_seed_agents(&self, charter_id: Option<&str>) -> SdkResult<Value> {
        let params = match charter_id {
            Some(cid) => serde_json::json!({ "charter_id": cid }),
            None => serde_json::json!({}),
        };
        self.rpc.call("tenzro_listSeedAgents", params).await
    }

    /// Returns network activity metrics over the requested window. Used by
    /// the SeedAgent counterparty filter and the organic-activity dashboards
    /// to exclude protocol-owned bootstrap traffic during the 12-month
    /// earmark window.
    pub async fn get_network_activity(&self, window_blocks: Option<u64>) -> SdkResult<Value> {
        let params = match window_blocks {
            Some(w) => serde_json::json!({ "window_blocks": w }),
            None => serde_json::json!({}),
        };
        self.rpc.call("tenzro_getNetworkActivity", params).await
    }
}
