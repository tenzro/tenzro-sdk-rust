//! Capital Intent client — regulated capital allocation over tokenized assets.
//!
//! `CapitalIntent` is the capital-markets analog of an AP2 Intent Mandate:
//! a signed, expiring authorization that says "I want to acquire / exit /
//! rebalance / hedge / yield this basket, subject to these regulatory
//! constraints, KYA, and ceilings." Solvers bid; the principal (or an
//! authorized assigner) picks one; the intent runs through Execute → Settle
//! with optional Verify and Compensate steps. The full lifecycle is
//! mediated by `tenzro_capitalIntent*` RPCs.
//!
//! Companion read paths:
//! - `tenzro_getCapitalIntent` for the current intent state.
//! - `tenzro_getReserve` and `tenzro_submitReserveAttestation` for the
//!   1:1-backed reserve attestations that underpin attested mints
//!   (`tenzro_attestedMint`).
//!
//! # Example
//!
//! ```no_run
//! # use tenzro_sdk::TenzroClient;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = TenzroClient::new("https://rpc.tenzro.xyz").await?;
//! let capital = client.capital();
//!
//! // Submit a signed intent (already authorized + signed by the principal).
//! let opened = capital.open(serde_json::json!({ /* CapitalIntent payload */ })).await?;
//! println!("intent opened: {opened}");
//!
//! // Solver bids; assigner picks the best one (auto-rank by reputation + price + eta).
//! let assigned = capital.assign("intent_id", None, true, None, None).await?;
//! println!("assigned: {assigned}");
//!
//! // Execute, settle.
//! capital.execute("intent_id", serde_json::json!({ /* leg */ })).await?;
//! capital.settle("intent_id", None).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde_json::Value;
use std::sync::Arc;

/// Capital Intent + Reserve client.
#[derive(Clone)]
pub struct CapitalClient {
    rpc: Arc<RpcClient>,
}

impl CapitalClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Open a new Capital Intent. `intent` is the signed CapitalIntent payload
    /// (objective + constraints + compliance + authorization + settlement_req).
    pub async fn open(&self, intent: Value) -> SdkResult<Value> {
        self.rpc
            .call("tenzro_capitalIntentOpen", serde_json::json!({ "intent": intent }))
            .await
    }

    /// Submit a solver bid against an opened intent.
    pub async fn quote(
        &self,
        intent_id: &str,
        solver_did: &str,
        plan: &str,
        price: u64,
        eta_secs: u64,
    ) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_capitalIntentQuote",
                serde_json::json!({
                    "intent_id": intent_id,
                    "solver_did": solver_did,
                    "plan": plan,
                    "price": price,
                    "eta_secs": eta_secs,
                }),
            )
            .await
    }

    /// Assign the intent to a solver. Pass `solver_did=None, auto=true` to
    /// auto-rank by ERC-8004 reputation, then price, then eta.
    pub async fn assign(
        &self,
        intent_id: &str,
        solver_did: Option<&str>,
        auto: bool,
        payer: Option<&str>,
        payee: Option<&str>,
    ) -> SdkResult<Value> {
        let mut params = serde_json::json!({ "intent_id": intent_id });
        let obj = params.as_object_mut().unwrap();
        if let Some(s) = solver_did {
            obj.insert("solver_did".into(), serde_json::json!(s));
        }
        if auto {
            obj.insert("auto".into(), serde_json::json!(true));
        }
        if let Some(p) = payer {
            obj.insert("payer".into(), serde_json::json!(p));
        }
        if let Some(p) = payee {
            obj.insert("payee".into(), serde_json::json!(p));
        }
        self.rpc.call("tenzro_capitalIntentAssign", params).await
    }

    /// Execute a single leg of an assigned intent.
    pub async fn execute(&self, intent_id: &str, leg: Value) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_capitalIntentExecute",
                serde_json::json!({ "intent_id": intent_id, "leg": leg }),
            )
            .await
    }

    /// Verify a step (e.g. that a leg's settlement proof is anchored on-chain).
    pub async fn verify(&self, intent_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_capitalIntentVerify",
                serde_json::json!({ "intent_id": intent_id }),
            )
            .await
    }

    /// Compensate (roll back) a step that failed verification.
    pub async fn compensate(&self, intent_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_capitalIntentCompensate",
                serde_json::json!({ "intent_id": intent_id }),
            )
            .await
    }

    /// Settle the intent — release escrow to the payee.
    pub async fn settle(&self, intent_id: &str, payee: Option<&str>) -> SdkResult<Value> {
        let mut params = serde_json::json!({ "intent_id": intent_id });
        if let Some(p) = payee {
            params
                .as_object_mut()
                .unwrap()
                .insert("payee".into(), serde_json::json!(p));
        }
        self.rpc.call("tenzro_capitalIntentSettle", params).await
    }

    /// Read the current state of a capital intent.
    pub async fn get(&self, intent_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_getCapitalIntent",
                serde_json::json!({ "intent_id": intent_id }),
            )
            .await
    }

    /// Submit a 1:1-backed reserve attestation for a tokenized asset.
    pub async fn submit_reserve_attestation(&self, attestation: Value) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_submitReserveAttestation",
                serde_json::json!({ "attestation": attestation }),
            )
            .await
    }

    /// Read the latest reserve attestation for a tokenized asset.
    pub async fn get_reserve(&self, asset_id: &str) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_getReserve",
                serde_json::json!({ "asset_id": asset_id }),
            )
            .await
    }

    /// Attested 1:1 mint — token issuance gated by a fresh reserve attestation.
    pub async fn attested_mint(
        &self,
        token_id: &str,
        to: &str,
        amount: &str,
        caller: &str,
    ) -> SdkResult<Value> {
        self.rpc
            .call(
                "tenzro_attestedMint",
                serde_json::json!({
                    "token_id": token_id,
                    "to": to,
                    "amount": amount,
                    "caller": caller,
                }),
            )
            .await
    }
}
