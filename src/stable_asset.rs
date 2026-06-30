//! Stable-asset issuance client.
//!
//! Issuer-agnostic stable-unit policies layered on the Secure-Mint reserve
//! floor. An issuer registers a unit, then mints/redeems against it; mints are
//! hard-gated so `circulating + amount ≤ reserve` always holds. Registration
//! requires an API key carrying the `issuer` scope.
//!
//! Wraps the node-side RPCs:
//! - `tenzro_registerStableAsset`, `tenzro_getStableAsset`
//! - `tenzro_mintStableAsset`, `tenzro_redeemStableAsset`

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct StableAssetClient {
    rpc: Arc<RpcClient>,
}

impl StableAssetClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Register or replace an issuer's stable-asset policy (needs `issuer` scope).
    pub async fn register(&self, params: RegisterStableAsset) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_registerStableAsset", serde_json::json!([params]))
            .await
    }

    /// Read an issuer's stable-asset policy.
    pub async fn get(&self, issuer: &str, unit_token: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getStableAsset",
                serde_json::json!([{ "issuer": issuer, "unit_token": unit_token }]),
            )
            .await
    }

    /// Mint units, gated by the Secure-Mint reserve floor.
    pub async fn mint(
        &self,
        issuer: &str,
        unit_token: &str,
        amount: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_mintStableAsset",
                serde_json::json!([{ "issuer": issuer, "unit_token": unit_token, "amount": amount }]),
            )
            .await
    }

    /// Redeem (burn) units, decrementing circulating supply.
    pub async fn redeem(
        &self,
        issuer: &str,
        unit_token: &str,
        amount: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_redeemStableAsset",
                serde_json::json!([{ "issuer": issuer, "unit_token": unit_token, "amount": amount }]),
            )
            .await
    }
}

/// Reserve backing source for a stable unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReserveSource {
    /// Off-chain custodial reserve, attested by a DID.
    Custodial {
        attester_did: String,
        asset_caip19: String,
    },
    /// On-chain vault holding the backing asset.
    OnChainVault {
        vault: String,
        asset_caip19: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterStableAsset {
    pub issuer: String,
    pub unit_token: String,
    pub symbol: String,
    pub reserve_source: ReserveSource,
    pub por_feed_id: String,
    pub allowed_rails: Vec<String>,
    pub settlement_dst: String,
}
