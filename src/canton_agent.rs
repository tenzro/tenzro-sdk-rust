//! CantonAgentClient — agentic Canton SDK surface.
//!
//! Built on top of the existing `CantonClient` (which covers the
//! operator-facing Canton ops), this client adds the autonomous-
//! agent path: mandate-bound DAML write via `tenzro_canton_submitWithMandate`,
//! scoped read via `tenzro_canton_watchParty`, and rollup analytics
//! via `tenzro_canton_aggregateAnalytics`.
//!
//! Use this client when the caller is an autonomous agent operating
//! under an API key with `can_act_as_parties` / `can_read_as_parties`
//! delegation scopes — the gate is enforced at the node, so the SDK
//! does not pre-check.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct CantonAgentClient {
    rpc: Arc<RpcClient>,
}

impl CantonAgentClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Submit a DAML command bound to an AP2 mandate pair. Validates
    /// the cart against AP2 invariants + TDIP delegation + runtime
    /// SpendingPolicy + optional escrow / SPT ceilings. Only when all
    /// applicable ceilings pass does the DAML command submit.
    pub async fn submit_with_mandate(
        &self,
        params: SubmitWithMandateParams,
    ) -> SdkResult<MandateBoundReceipt> {
        let v = serde_json::to_value(&params).map_err(|_| {
            crate::error::SdkError::SerializationError
        })?;
        self.rpc.call("tenzro_canton_submitWithMandate", v).await
    }

    /// Get the live active-contracts snapshot for a single party. The
    /// presenting API key must allow reading for this party (via
    /// `can_read_as_parties` on the issued key).
    pub async fn watch_party(
        &self,
        party_fq: &str,
        template_ids: Vec<String>,
    ) -> SdkResult<WatchPartySnapshot> {
        self.rpc
            .call(
                "tenzro_canton_watchParty",
                serde_json::json!({
                    "party": party_fq,
                    "template_ids": template_ids,
                }),
            )
            .await
    }

    /// Operator admin-read of rolled-up per-key Canton call counters.
    /// Admin-token-gated. `group_by = "subject" | "key_id"`.
    pub async fn aggregate_analytics(
        &self,
        group_by: &str,
    ) -> SdkResult<AggregateAnalytics> {
        self.rpc
            .call(
                "tenzro_canton_aggregateAnalytics",
                serde_json::json!({"group_by": group_by}),
            )
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitWithMandateParams {
    /// AP2 checkout VDC.
    pub mandate: Mandate,
    /// `create` or `exercise`.
    pub command_type: String,
    /// Canton template id (e.g. `#Splice.AmuletRules:AmuletRules:Transfer`).
    pub template_id: String,
    /// `create` arguments OR `exercise` arguments (choose by `command_type`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_arguments: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choice_argument: Option<serde_json::Value>,
    /// Optional `actAs` override for the submission.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_as: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mandate {
    pub checkout: serde_json::Value,
    pub payment: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MandateBoundReceipt {
    pub ap2_receipt: serde_json::Value,
    pub canton_receipt: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchPartySnapshot {
    pub party: String,
    pub template_ids: Vec<String>,
    pub active_contracts: serde_json::Value,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateAnalytics {
    pub group_by: String,
    pub buckets: Vec<AnalyticsBucket>,
    pub row_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsBucket {
    pub key: String,
    pub total_calls: u64,
    pub last_called_at: i64,
}
