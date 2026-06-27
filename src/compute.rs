//! Compute-rental client for Tenzro Network.
//!
//! A node started with the `ai` role spawns a compute-rental runtime alongside
//! its inference: it rents out CPU/GPU capacity for fixed terms and bills
//! renters per epoch through a streaming deal. Each epoch settles only when an
//! availability proof passes — a provider that cannot serve the reserved
//! capacity earns nothing that epoch, and repeated misses terminate the rental
//! and make the renter whole from the provider's stake.
//!
//! These calls target the provider node directly; they succeed only when that
//! node is serving the AI role.

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// Compute-rental client.
pub struct ComputeClient {
    rpc: Arc<RpcClient>,
}

impl ComputeClient {
    /// Creates a new compute client.
    pub fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Books a fixed-term compute rental against this provider. The renter
    /// pre-funds from their deposit; per-epoch price is the provider's effective
    /// rate. The locked deposit streams to the provider as epochs settle.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let compute = client.compute();
    /// let rental = compute.book_rental("0xrenter", 24).await?;
    /// println!("booked rental {}", rental.rental_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn book_rental(&self, renter: &str, total_epochs: u64) -> SdkResult<ComputeRental> {
        let result = self
            .rpc
            .call(
                "tenzro_computeBookRental",
                json!([{
                    "renter": renter,
                    "total_epochs": total_epochs,
                }]),
            )
            .await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse rental: {}", e)))
    }

    /// Settles one epoch of an active rental, gated on the provider's
    /// availability proof. A valid proof streams the epoch's slice to the
    /// provider; an invalid/missing proof makes the renter whole from stake.
    pub async fn settle_epoch(&self, rental_id: &str, proof_valid: bool) -> SdkResult<EpochOutcome> {
        let result = self
            .rpc
            .call(
                "tenzro_computeSettleEpoch",
                json!([{ "rental_id": rental_id, "proof_valid": proof_valid }]),
            )
            .await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse epoch outcome: {}", e)))
    }

    /// Looks up a compute rental by id.
    pub async fn get_rental(&self, rental_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_computeGetRental", json!([{ "rental_id": rental_id }]))
            .await
    }

    /// Switches the provider to network-dynamic per-epoch pricing seeded from
    /// the current rate. `capacity` is the provider's epoch-slot capacity (the
    /// utilization target is 50% of it); `min_rate`/`max_rate` bound the rate.
    pub async fn set_dynamic_pricing(
        &self,
        capacity: u128,
        min_rate: u128,
        max_rate: Option<u128>,
    ) -> SdkResult<serde_json::Value> {
        let mut params = json!({
            "mode": "dynamic",
            "capacity": capacity.to_string(),
            "min_rate": min_rate.to_string(),
        });
        if let Some(max) = max_rate {
            params["max_rate"] = json!(max.to_string());
        }
        self.rpc
            .call("tenzro_computeSetPricing", json!([params]))
            .await
    }

    /// Returns this node's compute-provider status (effective rate, active rentals).
    pub async fn status(&self) -> SdkResult<ComputeStatus> {
        let result = self.rpc.call("tenzro_computeStatus", json!([])).await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse compute status: {}", e)))
    }
}

/// A booked compute rental (mirrors the node's `RentalAgreement`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeRental {
    /// Unique rental id.
    pub rental_id: String,
    /// Per-epoch price in wei.
    pub price_per_epoch: u128,
    /// Total epochs in the term.
    pub total_epochs: u64,
    /// Epochs delivered and paid so far.
    pub epochs_settled: u64,
}

/// Outcome of one settle epoch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochOutcome {
    /// Rental id settled.
    pub rental_id: String,
    /// Epoch status: `settled`, `missed`, `closed`.
    pub status: String,
    /// Whether the provider was paid this epoch.
    pub settled: bool,
    /// Amount moved this epoch in wei (string-encoded): the slice paid to the
    /// provider on `settled`, or the make-whole credited to the renter on `missed`.
    pub amount_wei: String,
}

/// Compute-provider status snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeStatus {
    /// Whether this node is serving the AI/compute role.
    #[serde(default)]
    pub is_compute_provider: bool,
    /// Effective per-epoch rate in wei (string-encoded).
    #[serde(default)]
    pub effective_rate_wei: String,
    /// Number of active rentals on this provider.
    #[serde(default)]
    pub active_rentals: u64,
}
