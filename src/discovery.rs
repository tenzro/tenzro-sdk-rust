//! Discovery + helper RPCs for the IBC-Eureka, NEAR Chain Signatures,
//! BitVM2, Hyperbridge, Stargate V2 Hydra, Universal Resolver, SIWT,
//! KERI, MPC pre-sign / PKR, global supply, and Institution-identity
//! modules.
//!
//! State-bearing dispatch (party allocation, threshold sign, mint apply)
//! lives on the individual adapter clients; the surface here gives wallets
//! and SDK consumers read access to the new modules.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct DiscoveryClient {
    rpc: Arc<RpcClient>,
}

impl DiscoveryClient {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// IBC-Eureka commitment domain tag (the on-EVM `IBC_VERIFY`
    /// precompile at `0x1020` prepends this when hashing outcomes).
    pub async fn ibc_eureka_commitment_tag(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_ibcEurekaCommitmentTag", serde_json::json!([])).await
    }

    /// NEAR Chain Signatures `epsilon` derivation for a
    /// `(predecessor, path)` pair.
    pub async fn near_chain_sig_epsilon(
        &self,
        predecessor: &str,
        path: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_nearChainSigEpsilon",
                serde_json::json!({
                    "predecessor": predecessor,
                    "path": path,
                }),
            )
            .await
    }

    /// Supported BitVM2 / Clementine verifier kinds.
    pub async fn bitvm2_verifier_kinds(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_bitvm2VerifierKinds", serde_json::json!([])).await
    }

    /// Default Hyperbridge mint-control policy (post-2026-04-13 hardening).
    pub async fn hyperbridge_mint_controls_default(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_hyperbridgeMintControlsDefault", serde_json::json!([]))
            .await
    }

    /// Verified Stargate V2 Hydra pools.
    pub async fn stargate_v2_known_pools(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_stargateV2KnownPools", serde_json::json!([])).await
    }

    /// Methods this Universal Resolver instance can resolve.
    pub async fn universal_resolver_methods(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_universalResolverMethods", serde_json::json!([]))
            .await
    }

    /// Build a SIWT canonical-form message from a JSON payload.
    pub async fn siwt_build_message(
        &self,
        message: SiwtBuildPayload,
    ) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_siwtBuildMessage", serde_json::json!([message])).await
    }

    /// Parse a SIWT canonical-form message.
    pub async fn siwt_parse_message(&self, message: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_siwtParseMessage",
                serde_json::json!({ "message": message }),
            )
            .await
    }

    /// Build a KERI inception event from hex-encoded signing-key bytes
    /// and SHA-256 digests of the next signing keys.
    pub async fn keri_build_inception(
        &self,
        signing_keys_hex: Vec<String>,
        next_key_digests_hex: Vec<String>,
        signing_threshold: Option<u8>,
        next_threshold: Option<u8>,
    ) -> SdkResult<serde_json::Value> {
        let mut params = serde_json::json!({
            "signing_keys_hex": signing_keys_hex,
            "next_key_digests_hex": next_key_digests_hex,
        });
        if let Some(t) = signing_threshold {
            params["signing_threshold"] = serde_json::Value::from(t);
        }
        if let Some(t) = next_threshold {
            params["next_threshold"] = serde_json::Value::from(t);
        }
        self.rpc.call("tenzro_keriBuildInception", params).await
    }

    /// MPC pre-signing pool stats (one entry per active group).
    pub async fn mpc_presign_stats(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_mpcPresignStats", serde_json::json!([])).await
    }

    /// MPC PKR scheduler snapshots (one entry per active group).
    pub async fn mpc_pkr_status(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_mpcPkrStatus", serde_json::json!([])).await
    }

    /// Read the global-supply policy for an asset.
    pub async fn global_supply_policy(&self, asset_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_globalSupplyPolicy",
                serde_json::json!({ "asset_id": asset_id }),
            )
            .await
    }

    /// Read the global-supply circulating amount for an asset.
    pub async fn global_supply_circulating(&self, asset_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_globalSupplyCirculating",
                serde_json::json!({ "asset_id": asset_id }),
            )
            .await
    }

    /// Validate a 20-character ISO 17442 LEI via Mod 97-10.
    pub async fn validate_lei(&self, lei: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_validateLei", serde_json::json!({ "lei": lei }))
            .await
    }

    /// Decentralized MoE shard map for `model_id`: distinct providers
    /// holding each `(layer, expert)` for the model, per-expert
    /// replication factor, under-replicated experts, hot experts, and
    /// role counts (ExpertHolder / Router / Prefill / Decode / Replica).
    pub async fn moe_shard_map(&self, model_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_moeShardMap",
                serde_json::json!({ "model_id": model_id }),
            )
            .await
    }

    /// Build a dispatch plan for `model_id` given per-token top-k routing
    /// decisions. `routings` is `[{token_index, experts: [{layer,
    /// expert}, ...]}, ...]`. When `allow_cold` is false (default) the
    /// planner only picks warm holders.
    pub async fn moe_plan_dispatch(
        &self,
        model_id: &str,
        routings: serde_json::Value,
        allow_cold: bool,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_moePlanDispatch",
                serde_json::json!({
                    "model_id": model_id,
                    "routings": routings,
                    "allow_cold": allow_cold,
                }),
            )
            .await
    }

    /// Current governance-tuned replication policy.
    pub async fn moe_replication_policy(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_moeReplicationPolicy", serde_json::json!([]))
            .await
    }

    /// Catalog-side MoE topology (`num_experts`, `experts_per_token`,
    /// `shared_experts`, `params_per_expert_x10`) for `model_id`.
    pub async fn moe_catalog_shape(&self, model_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_moeCatalogShape",
                serde_json::json!({ "model_id": model_id }),
            )
            .await
    }

    /// Resident experts and gates on this node's expert runtime, each with
    /// its residency tier (`Warm` in memory / `Cold` on disk) and byte
    /// footprint, plus the memory budget and whether GPU compute is active.
    pub async fn moe_expert_status(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_moeExpertStatus", serde_json::json!([]))
            .await
    }

    /// Slice a catalog checkpoint into per-expert blobs, optionally
    /// block-quantizing each projection, and publish them for holders to
    /// load. `quant` is a preset (`q4_k_m` / `q8_0` / `q4_k` / `q6_k`) or a
    /// per-projection object like `{"gate":"q4_k","up":"q4_k","down":"q6_k"}`;
    /// pass `serde_json::Value::Null` to publish dense f32 blobs. `experts`
    /// empty prepares every expert in the layer.
    pub async fn moe_prepare_experts(
        &self,
        model_id: &str,
        layer: u32,
        experts: Vec<u32>,
        include_gate: bool,
        quant: serde_json::Value,
    ) -> SdkResult<serde_json::Value> {
        let mut params = serde_json::json!({
            "model_id": model_id,
            "layer": layer,
            "include_gate": include_gate,
        });
        if !experts.is_empty() {
            params["experts"] = serde_json::json!(experts);
        }
        if !quant.is_null() {
            params["quant"] = quant;
        }
        self.rpc.call("tenzro_moePrepareExperts", params).await
    }

    /// Run one distributed MoE layer forward on the node's expert runtime:
    /// gate `hidden` locally, fan the selected experts out to holders over
    /// the `tenzro/moe` transport, and combine the gate-weighted outputs.
    /// `hidden` is the row-major `[num_tokens, d_model]` activation.
    pub async fn moe_forward(
        &self,
        model_id: &str,
        layer: u32,
        d_model: u32,
        hidden: Vec<f32>,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_moeForward",
                serde_json::json!({
                    "model_id": model_id,
                    "layer": layer,
                    "d_model": d_model,
                    "hidden": hidden,
                }),
            )
            .await
    }

    /// Peer IDs currently discovered on this node's local segment via mDNS.
    /// Returns `{ local_peers: [..], count, available }`; `available` is
    /// false when local discovery is not running.
    pub async fn local_peers(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_localPeers", serde_json::json!([])).await
    }

    /// This node's sustained connectivity tier (`direct` / `relay_only` /
    /// `unreachable`). Returns `{ tier, available }`.
    pub async fn node_reachability(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_nodeReachability", serde_json::json!([])).await
    }

    /// This node's hardware self-profile from the ggml device API: build
    /// commit, CPU arch, OS, devices, and the derived serving VRAM / backend /
    /// capability key.
    pub async fn node_profile(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_nodeProfile", serde_json::json!([])).await
    }

    /// Deterministic cluster placement for a model across candidate members.
    /// `model` is `{ layers, hidden_dim, total_vram_gb }`; `members` is the
    /// candidate `ClusterMember` array. When `force` is true a cluster is
    /// requested even if one member fits the whole model. Returns the fit
    /// decision and, when a cluster forms, the ordered per-member layer stages.
    pub async fn cluster_plan(
        &self,
        model: serde_json::Value,
        members: serde_json::Value,
        force: bool,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_clusterPlan",
                serde_json::json!({
                    "model": model,
                    "members": members,
                    "user_forced": force,
                }),
            )
            .await
    }

    /// Preview how a downloaded model would be placed using the node's live
    /// view: derives the model shape from the GGUF header and discovers LAN
    /// members from gossip — no manual dimensions or member list required.
    /// `force` requests a cluster even when the model fits one member;
    /// `force_single` previews single-host placement. Returns the fit
    /// decision, discovered members, any rejected members (with reasons), and
    /// the proposed per-member layer stages. Call this before
    /// [`ProviderClient::serve_model`](crate::provider::ProviderClient::serve_model)
    /// to show the operator what serving will do.
    pub async fn cluster_preview(
        &self,
        model_id: &str,
        force: bool,
        force_single: bool,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_clusterPreview",
                serde_json::json!({
                    "model_id": model_id,
                    "user_forced": force,
                    "force_single": force_single,
                }),
            )
            .await
    }
}

/// JSON payload for `siwt_build_message`. Mirrors the
/// `SiwtMessage` struct in `tenzro-node::web::siwt`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiwtBuildPayload {
    pub domain: String,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement: Option<String>,
    pub uri: String,
    pub version: String,
    pub chain_id: u64,
    pub nonce: String,
    pub issued_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub resources: Vec<String>,
}
