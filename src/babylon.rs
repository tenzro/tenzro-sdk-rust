//! Babylon Bitcoin staking client.
//!
//! Babylon's finality-providers protocol lets Tenzro validators be
//! economically secured by native BTC. Bitcoin holders delegate to a
//! Tenzro validator (registered as a Babylon finality provider), and
//! the validator must submit Extractable One-Time Signatures (EOTS)
//! over Tenzro block hashes to avoid slashing of the delegated BTC.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct BabylonClient {
    rpc: Arc<RpcClient>,
}

impl BabylonClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    pub async fn register_finality_provider(
        &self,
        req: RegisterFinalityProviderRequest,
    ) -> SdkResult<FinalityProvider> {
        self.rpc
            .call(
                "tenzro_babylonRegisterFinalityProvider",
                serde_json::json!([req]),
            )
            .await
    }

    pub async fn get_finality_provider(
        &self,
        validator: &str,
    ) -> SdkResult<Option<FinalityProvider>> {
        self.rpc
            .call(
                "tenzro_babylonGetFinalityProvider",
                serde_json::json!([{ "validator": validator }]),
            )
            .await
    }

    pub async fn list_finality_providers(&self) -> SdkResult<Vec<FinalityProvider>> {
        self.rpc
            .call("tenzro_babylonListFinalityProviders", serde_json::json!([]))
            .await
    }

    pub async fn total_stake_for_provider(&self, validator: &str) -> SdkResult<BabylonTotalStake> {
        self.rpc
            .call(
                "tenzro_babylonTotalStakeForProvider",
                serde_json::json!([{ "validator": validator }]),
            )
            .await
    }

    pub async fn submit_finality_signature(
        &self,
        req: SubmitFinalitySignatureRequest,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_babylonSubmitFinalitySignature",
                serde_json::json!([req]),
            )
            .await
    }

    pub async fn list_delegations(&self, validator: &str) -> SdkResult<Vec<BtcDelegation>> {
        self.rpc
            .call(
                "tenzro_babylonListDelegations",
                serde_json::json!([{ "validator": validator }]),
            )
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegisterFinalityProviderRequest {
    pub validator: String,
    pub btc_pk: String,
    pub commission_bps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityProvider {
    #[serde(default)]
    pub validator: String,
    #[serde(default)]
    pub btc_pk: String,
    #[serde(default)]
    pub commission_bps: u32,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BabylonTotalStake {
    #[serde(default)]
    pub validator: String,
    #[serde(default)]
    pub total_satoshis: u64,
    #[serde(default)]
    pub delegation_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubmitFinalitySignatureRequest {
    pub validator: String,
    pub block_hash: String,
    pub eots_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtcDelegation {
    #[serde(default)]
    pub delegator_btc_pk: String,
    #[serde(default)]
    pub validator: String,
    #[serde(default)]
    pub satoshis: u64,
    #[serde(default)]
    pub start_height: u64,
    #[serde(default)]
    pub end_height: Option<u64>,
}
