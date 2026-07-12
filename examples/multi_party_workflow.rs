//! Multi-party saga workflow end-to-end example.
//!
//! Opens a two-step DvP workflow (buyer escrows USDC, seller delivers
//! T-bills), drives each step through Execute → Verify, finalizes, and
//! mirrors the receipt to Canton DAML for the regulated counterparty's
//! reconciliation.
//!
//! ```bash
//! cargo run -p tenzro-sdk --example multi_party_workflow
//! ```

use serde_json::json;
use tenzro_sdk::TenzroClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TenzroClient::new("https://rpc.tenzro.xyz").await?;
    let workflow = client.workflow();

    // 1. Declare the workflow.
    let payload = json!({
        "creator_did": "did:tenzro:human:alice",
        "participants": [
            "did:tenzro:human:alice",
            "did:tenzro:human:bob"
        ],
        "steps": [
            { "step_id": "escrow-funds", "status": "pending" },
            { "step_id": "deliver-tbills", "status": "pending" }
        ],
        "metadata": {
            "purpose": "DvP — delivery-versus-payment, T-bill primary purchase"
        }
    });

    let opened = workflow.open(payload).await?;
    let workflow_id = opened
        .get("workflow_id")
        .and_then(|v| v.as_str())
        .unwrap_or("workflow-id-here");
    println!("workflow opened: {workflow_id}");

    // 2. Step 1: buyer escrows USDC (per-step escrow ceiling).
    workflow
        .step_execute(workflow_id, "escrow-funds", Some(1_000_000_000))
        .await?;
    workflow.step_verify(workflow_id, "escrow-funds").await?;

    // 3. Step 2: seller delivers tokenized T-bills.
    workflow.step_execute(workflow_id, "deliver-tbills", None).await?;
    match workflow.step_verify(workflow_id, "deliver-tbills").await {
        Ok(_) => {
            workflow.finalize(workflow_id).await?;
            workflow.mirror_to_canton(workflow_id).await?;
            let receipt = workflow.get_receipt(workflow_id).await?;
            let metrics = workflow.get_operational_metrics(workflow_id).await?;
            println!("receipt: {receipt}");
            println!("metrics: {metrics}");
        }
        Err(err) => {
            println!("delivery verify failed — compensating: {err}");
            workflow.step_compensate(workflow_id, "deliver-tbills").await?;
            workflow.step_compensate(workflow_id, "escrow-funds").await?;
        }
    }
    Ok(())
}
