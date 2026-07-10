//! Decentralized storage client for Tenzro Network.
//!
//! A node started with the `storage` role spawns a storage-provider runtime:
//! it erasure-codes objects, publishes their shards over the content-addressed
//! transport, and bills renters per epoch through a streaming deal. Each epoch
//! is charged only when a retrievability challenge passes — a provider that
//! cannot prove it still holds the data earns nothing that epoch, and repeated
//! misses terminate the deal.
//!
//! These calls target the provider node directly; they succeed only when that
//! node is serving the storage role.

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// Storage-provider client.
pub struct StorageClient {
    rpc: Arc<RpcClient>,
}

impl StorageClient {
    /// Creates a new storage client.
    pub fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Stores an object: erasure-codes `data` under `data_shards`+`parity_shards`
    /// and publishes the shards over the transport. Returns the stored size and
    /// shard layout.
    ///
    /// `owner_did` gates who may retrieve the object. When `Some`, the object is
    /// owner-only to that DID (the same access model the database tier uses);
    /// when `None`, it defaults to owner-only for the `owner` address.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let storage = client.storage();
    /// let stored = storage
    ///     .store_object("photo-1", "0xowner", b"...bytes...", 4, 2, Some("did:tenzro:human:alice"))
    ///     .await?;
    /// println!("stored {} bytes", stored.size_bytes);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn store_object(
        &self,
        object_id: &str,
        owner: &str,
        data: &[u8],
        data_shards: u32,
        parity_shards: u32,
        owner_did: Option<&str>,
    ) -> SdkResult<StoredObject> {
        let data_b64 = base64::engine::general_purpose::STANDARD.encode(data);
        let mut params = json!({
            "object_id": object_id,
            "owner": owner,
            "data": data_b64,
            "data_shards": data_shards,
            "parity_shards": parity_shards,
        });
        if let Some(did) = owner_did {
            params["owner_did"] = json!(did);
        }
        let result = self
            .rpc
            .call("tenzro_storageStoreObject", json!([params]))
            .await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse stored object: {}", e)))
    }

    /// Opens a streaming storage deal for an already-stored object. The renter
    /// pre-funds from their deposit; per-epoch price is `size_bytes × rate`.
    pub async fn open_deal(
        &self,
        object_id: &str,
        renter: &str,
        size_bytes: u64,
        total_epochs: u64,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_storageOpenDeal",
                json!([{
                    "object_id": object_id,
                    "renter": renter,
                    "size_bytes": size_bytes,
                    "total_epochs": total_epochs,
                }]),
            )
            .await
    }

    /// Runs one proof-of-retrievability-gated charge epoch for a deal. The
    /// renter is billed only when the challenge passes.
    pub async fn charge_epoch(&self, deal_id: &str) -> SdkResult<ChargeOutcome> {
        let result = self
            .rpc
            .call(
                "tenzro_storageChargeEpoch",
                json!([{ "deal_id": deal_id }]),
            )
            .await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse charge outcome: {}", e)))
    }

    /// Looks up a storage deal by id.
    pub async fn get_deal(&self, deal_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_storageGetDeal", json!([{ "deal_id": deal_id }]))
            .await
    }

    /// Switches the provider to network-dynamic byte-epoch pricing seeded from
    /// the current rate. `capacity` is the provider's byte-epoch capacity (the
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
            .call("tenzro_storageSetPricing", json!([params]))
            .await
    }

    /// Returns this node's storage-provider status (effective rate, object count).
    pub async fn status(&self) -> SdkResult<StorageStatus> {
        let result = self.rpc.call("tenzro_storageStatus", json!([])).await?;
        serde_json::from_value(result)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse storage status: {}", e)))
    }
}

/// Result of [`StorageClient::store_object`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredObject {
    /// Logical object id.
    pub object_id: String,
    /// Stored payload size in bytes.
    pub size_bytes: usize,
    /// Number of data shards.
    pub data_shards: usize,
    /// Number of parity shards.
    pub parity_shards: usize,
}

/// Outcome of one charge epoch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargeOutcome {
    /// Deal id charged.
    pub deal_id: String,
    /// Epoch status: `charged`, `missed`, `closed_completed`, `closed_terminated`.
    pub status: String,
    /// Whether the renter was billed this epoch.
    pub charged: bool,
    /// Amount streamed to the provider this epoch, in wei (string-encoded).
    pub slice_wei: String,
}

/// Storage-provider status snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStatus {
    /// Whether this node is serving the storage role.
    #[serde(default)]
    pub is_storage_provider: bool,
    /// Effective byte-epoch rate in wei (string-encoded).
    #[serde(default)]
    pub effective_rate_wei: String,
    /// Number of objects this provider holds.
    #[serde(default)]
    pub object_count: u64,
}
