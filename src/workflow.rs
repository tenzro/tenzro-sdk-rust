//! Multi-agent saga workflow client.
//!
//! A workflow is an ordered sequence of saga steps (`Execute → Verify →
//! Compensate`) with optional per-step escrow, durable lifecycle state, and
//! optional mirroring to Canton DAML for institutional counterparties.
//! The full lifecycle is mediated by `tenzro_workflow*` RPCs.
//!
//! # Example
//!
//! ```no_run
//! # use tenzro_sdk::TenzroClient;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = TenzroClient::new("https://rpc.tenzro.network").await?;
//! let workflow = client.workflow();
//!
//! // Open a workflow with a set of declared steps.
//! let opened = workflow.open(serde_json::json!({ /* workflow payload */ })).await?;
//! let id = opened["workflow_id"].as_str().unwrap();
//!
//! // Drive each step through its lifecycle.
//! workflow.step_execute(id, "step-1", None).await?;
//! workflow.step_verify(id, "step-1").await?;
//! // (or compensate if verify fails)
//! workflow.finalize(id).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde_json::Value;
use std::sync::Arc;

/// Multi-agent saga workflow client.
#[derive(Clone)]
pub struct WorkflowClient {
    rpc: Arc<RpcClient>,
}

impl WorkflowClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Open a multi-agent saga workflow.
    pub async fn open(&self, workflow: Value) -> SdkResult<Value> {
        self.rpc
            .call("tenzro_workflowOpen", serde_json::json!({ "workflow": workflow }))
            .await
    }

    /// Transition a step Pending → Executing, optionally locking a per-step escrow.
    pub async fn step_execute(
        &self,
        workflow_id: &str,
        step_id: &str,
        escrow_amount: Option<u128>,
    ) -> SdkResult<Value> {
        let mut params = serde_json::json!({
            "workflow_id": workflow_id,
            "step_id": step_id,
        });
        if let Some(amt) = escrow_amount {
            params
                .as_object_mut()
                .unwrap()
                .insert("escrow_amount".into(), serde_json::json!(amt.to_string()));
        }
        self.rpc.call("tenzro_workflowStepExecute", params).await
    }

    /// Verify a step's outcome.
    pub async fn step_verify(&self, workflow_id: &str, step_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_workflowStepVerify",
                serde_json::json!({ "workflow_id": workflow_id, "step_id": step_id }),
            )
            .await
    }

    /// Compensate a step (roll back).
    pub async fn step_compensate(&self, workflow_id: &str, step_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_workflowStepCompensate",
                serde_json::json!({ "workflow_id": workflow_id, "step_id": step_id }),
            )
            .await
    }

    /// Finalize the workflow — emits a `WorkflowReceipt` when all steps have
    /// completed successfully (or compensated cleanly).
    pub async fn finalize(&self, workflow_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_workflowFinalize",
                serde_json::json!({ "workflow_id": workflow_id }),
            )
            .await
    }

    /// Read the current state of a workflow.
    pub async fn get(&self, workflow_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_getWorkflow",
                serde_json::json!({ "workflow_id": workflow_id }),
            )
            .await
    }

    /// Read the underlying saga (step-level execution state).
    pub async fn get_saga(&self, workflow_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_getWorkflowSaga",
                serde_json::json!({ "workflow_id": workflow_id }),
            )
            .await
    }

    /// Read the durable lifecycle record (Created / Open / Executing / …).
    pub async fn get_lifecycle(&self, workflow_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_getWorkflowLifecycle",
                serde_json::json!({ "workflow_id": workflow_id }),
            )
            .await
    }

    /// Read the receipt emitted when the workflow finalized.
    pub async fn get_receipt(&self, workflow_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_getWorkflowReceipt",
                serde_json::json!({ "workflow_id": workflow_id }),
            )
            .await
    }

    /// Read operational metrics for a workflow (durations, escrow flows, ...).
    pub async fn get_operational_metrics(&self, workflow_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_getWorkflowOperationalMetrics",
                serde_json::json!({ "workflow_id": workflow_id }),
            )
            .await
    }

    /// List recent workflow receipts.
    pub async fn list_receipts(&self, limit: Option<u32>) -> SdkResult<Value> {
        let mut params = serde_json::json!({});
        if let Some(l) = limit {
            params
                .as_object_mut()
                .unwrap()
                .insert("limit".into(), serde_json::json!(l));
        }
        self.rpc.call("tenzro_listWorkflowReceipts", params).await
    }

    /// List workflows authored by a creator DID.
    pub async fn list_by_creator(&self, creator_did: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_listWorkflowsByCreator",
                serde_json::json!({ "creator_did": creator_did }),
            )
            .await
    }

    /// List workflows where `participant_did` appears as a step actor.
    pub async fn list_by_participant(&self, participant_did: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_listWorkflowsByParticipant",
                serde_json::json!({ "participant_did": participant_did }),
            )
            .await
    }

    /// List workflows currently in a given status.
    pub async fn list_by_status(&self, status: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_listWorkflowsByStatus",
                serde_json::json!({ "status": status }),
            )
            .await
    }

    /// Mirror a workflow's receipt into Canton DAML for institutional
    /// counterparties.
    pub async fn mirror_to_canton(&self, workflow_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_mirrorWorkflowToCanton",
                serde_json::json!({ "workflow_id": workflow_id }),
            )
            .await
    }

    /// Verify a DID-signed envelope (e.g. an off-chain signed step result).
    pub async fn verify_did_envelope(&self, envelope: Value) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_verifyDidEnvelope",
                serde_json::json!({ "envelope": envelope }),
            )
            .await
    }
}
