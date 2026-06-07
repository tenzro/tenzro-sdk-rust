//! Chain-agnostic discovery client — CAIP-2 / CAIP-10 / CAIP-19.
//!
//! Implements the submitted `tenzro` CASA namespace
//! (`ChainAgnostic/namespaces#184`).
//!
//! - **CAIP-2** chain id: `tenzro:<lowercase hex of first 16 bytes of
//!   the genesis block hash>`. An EVM-compatible `evm_chain_id`
//!   sidecar is returned for tooling that needs the 64-bit EIP-155
//!   chain id.
//! - **CAIP-10** account id: accepts hex or base58btc input,
//!   normalises to canonical 64-hex Tenzro address form.
//! - **CAIP-19** asset id: `slip44` (SLIP-44 coin index `1414421071`),
//!   `token` (Tenzro token registry id, 32-byte hex), or `nft`
//!   (collection id + token id suffix).

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct CaipClient {
    rpc: Arc<RpcClient>,
}

impl CaipClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// CAIP-2 chain identifier for the connected node.
    pub async fn caip2(&self) -> SdkResult<Caip2Info> {
        self.rpc.call("tenzro_caip2", serde_json::json!([])).await
    }

    /// CAIP-10 account identifier for a Tenzro address.
    pub async fn caip10(&self, address: &str) -> SdkResult<Caip10Info> {
        self.rpc
            .call("tenzro_caip10", serde_json::json!([{ "address": address }]))
            .await
    }

    /// CAIP-19 asset identifier for a Tenzro asset. `kind` is one of
    /// `"slip44"`, `"token"`, or `"nft"`; the additional parameters
    /// depend on `kind` (see the `tenzro` CAIP namespace spec).
    pub async fn caip19(&self, params: Caip19Request) -> SdkResult<Caip19Info> {
        self.rpc
            .call("tenzro_caip19", serde_json::json!([params]))
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Caip2Info {
    #[serde(default)]
    pub chain_id: String,
    #[serde(default)]
    pub namespace: String,
    #[serde(default)]
    pub reference: String,
    #[serde(default)]
    pub evm_chain_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Caip10Info {
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub chain_id: String,
    #[serde(default)]
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Caip19Request {
    /// One of `"slip44"`, `"token"`, `"nft"`.
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nft_token_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Caip19Info {
    #[serde(default)]
    pub asset_id: String,
    #[serde(default)]
    pub chain_id: String,
    #[serde(default)]
    pub asset_namespace: String,
    #[serde(default)]
    pub asset_reference: String,
    #[serde(default)]
    pub token_id: Option<String>,
}
