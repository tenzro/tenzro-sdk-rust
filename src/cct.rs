//! TNZO CCT (Chainlink Cross-Chain Token) SDK for Tenzro Network
//!
//! Client for the canonical TNZO CCT pool registry: Ethereum uses a
//! LockRelease pool; Base, Arbitrum, Optimism, and Solana use BurnMint pools.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// TNZO CCT (Chainlink Cross-Chain Token) pool inspection client.
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::TenzroClient;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = TenzroClient::new("https://rpc.tenzro.network").await?;
/// let cct = client.cct();
/// let pools = cct.list_pools().await?;
/// println!("{} TNZO CCT pools registered", pools.count);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct CctClient {
    rpc: Arc<RpcClient>,
}

impl CctClient {
    /// Creates a new CCT client.
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// List all registered TNZO CCT pools.
    pub async fn list_pools(&self) -> SdkResult<CctPoolList> {
        self.rpc
            .call("tenzro_cctListPools", serde_json::json!({}))
            .await
    }

    /// Get a single TNZO CCT pool by chain name
    /// (e.g. `ethereum`, `base`, `arbitrum`, `optimism`, `solana`).
    pub async fn get_pool(&self, chain: &str) -> SdkResult<CctPool> {
        self.rpc
            .call(
                "tenzro_cctGetPool",
                serde_json::json!({ "chain": chain }),
            )
            .await
    }
}

/// List of TNZO CCT pools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CctPoolList {
    /// Number of registered pools.
    #[serde(default)]
    pub count: u64,
    /// Per-chain pool entries.
    #[serde(default)]
    pub pools: Vec<CctPool>,
}

/// Metadata for a single TNZO CCT pool on one chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CctPool {
    /// CAIP-2 or Chainlink chain id.
    #[serde(default)]
    pub chain_id: String,
    /// Chainlink CCIP chain selector.
    #[serde(default)]
    pub chain_selector: String,
    /// Deployed pool contract address.
    #[serde(default)]
    pub pool_address: String,
    /// Underlying TNZO token contract address on the chain.
    #[serde(default)]
    pub token_address: String,
    /// Pool type ("LockRelease" on Ethereum; "BurnMint" elsewhere).
    #[serde(default)]
    pub pool_type: String,
    /// Contract name (e.g. "LockReleaseTokenPool").
    #[serde(default)]
    pub contract_name: String,
    /// Outbound rate-limiter capacity (decimal string).
    #[serde(default)]
    pub outbound_capacity: String,
    /// Inbound rate-limiter capacity (decimal string).
    #[serde(default)]
    pub inbound_capacity: String,
    /// Rate-limiter refill rate (decimal string).
    #[serde(default)]
    pub refill_rate: String,
}
