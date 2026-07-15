//! App-hosting SDK for Tenzro Network
//!
//! Wraps the site / function / machine / lease JSON-RPC surface exposed by
//! `tenzro-node`. Deploy a static site, a `wasi:http` function, or a Firecracker
//! microVM and serve it under a public hostname (`myapp.apps.tenzro.xyz`) routed
//! from the operator's ingress edge to any serving node over the `tenzro/http`
//! ALPN.
//!
//! Content is uploaded as content-addressed blobs first (via
//! [`crate::iroh::IrohClient::publish_blob`]); the deploy call references each
//! blob by hash. Mutations (publish / remove / set-alias / claim-domain …) are
//! DID-owner-authenticated: pass the signed `did_envelope` header value for the
//! owner DID via [`HostingClient::with_did_envelope`].
//!
//! # Example
//!
//! ```no_run
//! # use tenzro_sdk::TenzroClient;
//! # use std::collections::BTreeMap;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = TenzroClient::new("https://rpc.tenzro.xyz").await?;
//! let iroh = client.iroh();
//! let hosting = client.hosting().with_did_envelope("<signed-envelope>");
//!
//! // Upload each asset, collect its blob hash into a route map.
//! let index = iroh.publish_blob(b"<!doctype html>...".to_vec()).await?;
//! let mut routes = Vec::new();
//! routes.push(tenzro_sdk::hosting::SiteRoute {
//!     path: "/index.html".into(),
//!     blob_hash: index.blake3_hex,
//!     content_type: "text/html".into(),
//!     size: index.size_bytes,
//! });
//!
//! let site = hosting
//!     .publish_site("my-app", "did:tenzro:human:...", routes)
//!     .spa(true)
//!     .call()
//!     .await?;
//! let site_id = site["site_id"].as_str().unwrap().to_string();
//!
//! // Attach a hostname so the site serves by name.
//! hosting
//!     .set_alias("my-app.apps.tenzro.xyz", &site_id, "did:tenzro:human:...")
//!     .await?;
//! # Ok(())
//! # }
//! ```

use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::sync::Arc;

/// App-hosting client (sites, functions, machines, leases).
#[derive(Clone)]
pub struct HostingClient {
    rpc: Arc<RpcClient>,
    did_envelope: Option<String>,
}

/// A single static-site route: a request path mapped to a content-addressed blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteRoute {
    /// Request path, e.g. `/index.html` or `/assets/app.js`.
    pub path: String,
    /// BLAKE3 blob hash of the asset (from `iroh.publish_blob`).
    pub blob_hash: String,
    /// MIME type served in the `Content-Type` header.
    pub content_type: String,
    /// Byte length of the asset.
    #[serde(default)]
    pub size: u64,
}

impl HostingClient {
    /// Creates a new hosting client.
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self {
            rpc,
            did_envelope: None,
        }
    }

    /// Attach the signed DID envelope forwarded on every mutating call so the
    /// node can verify the caller controls the owner DID. Returns a new client;
    /// the original is unchanged.
    pub fn with_did_envelope(&self, envelope: impl Into<String>) -> Self {
        Self {
            rpc: self.rpc.clone(),
            did_envelope: Some(envelope.into()),
        }
    }

    fn envelope_into(&self, params: &mut Map<String, Value>) {
        if let Some(env) = &self.did_envelope {
            params.insert("did_envelope".to_string(), Value::String(env.clone()));
        }
    }

    // ---- Static sites -----------------------------------------------------

    /// Publish a static site from a route map. Returns a builder so the optional
    /// index / not-found / SPA / pricing / placement fields can be set before
    /// `.call()`.
    pub fn publish_site(
        &self,
        name: impl Into<String>,
        owner_did: impl Into<String>,
        routes: Vec<SiteRoute>,
    ) -> PublishSiteBuilder<'_> {
        PublishSiteBuilder {
            client: self,
            name: name.into(),
            owner_did: owner_did.into(),
            routes,
            index_path: None,
            not_found_path: None,
            spa: false,
            price_per_request: None,
            replicas: None,
            region_hint: None,
            max_price_per_hour: None,
        }
    }

    /// Fetch a site manifest by `site_id`.
    pub async fn get_site(&self, site_id: &str) -> crate::error::SdkResult<Value> {
        self.rpc
            .call("tenzro_siteGet", json!({ "site_id": site_id }))
            .await
    }

    /// List sites, optionally filtered by owner DID.
    pub async fn list_sites(&self, owner_did: Option<&str>) -> crate::error::SdkResult<Value> {
        let params = match owner_did {
            Some(o) => json!({ "owner_did": o }),
            None => json!({}),
        };
        self.rpc.call("tenzro_listSites", params).await
    }

    /// Remove a site. Owner-authenticated.
    pub async fn remove_site(
        &self,
        site_id: &str,
        owner_did: &str,
    ) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("site_id".into(), json!(site_id));
        params.insert("owner_did".into(), json!(owner_did));
        self.envelope_into(&mut params);
        self.rpc.call("tenzro_siteRemove", Value::Object(params)).await
    }

    // ---- Hostname aliases -------------------------------------------------

    /// Point a public hostname at a site so it serves by name. Owner-authenticated.
    pub async fn set_alias(
        &self,
        hostname: &str,
        site_id: &str,
        owner_did: &str,
    ) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("hostname".into(), json!(hostname));
        params.insert("site_id".into(), json!(site_id));
        params.insert("owner_did".into(), json!(owner_did));
        self.envelope_into(&mut params);
        self.rpc.call("tenzro_siteSetAlias", Value::Object(params)).await
    }

    /// Resolve a hostname to its alias record.
    pub async fn get_alias(&self, hostname: &str) -> crate::error::SdkResult<Value> {
        self.rpc
            .call("tenzro_siteGetAlias", json!({ "hostname": hostname }))
            .await
    }

    /// List hostname aliases, optionally filtered by owner DID.
    pub async fn list_aliases(&self, owner_did: Option<&str>) -> crate::error::SdkResult<Value> {
        let params = match owner_did {
            Some(o) => json!({ "owner_did": o }),
            None => json!({}),
        };
        self.rpc.call("tenzro_listSiteAliases", params).await
    }

    /// Remove a hostname alias. Owner-authenticated.
    pub async fn remove_alias(
        &self,
        hostname: &str,
        owner_did: &str,
    ) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("hostname".into(), json!(hostname));
        params.insert("owner_did".into(), json!(owner_did));
        self.envelope_into(&mut params);
        self.rpc.call("tenzro_siteRemoveAlias", Value::Object(params)).await
    }

    // ---- Ingress placement (which serving nodes hold a site) --------------

    /// Set the serving nodes (iroh `EndpointId` strings) that answer
    /// `tenzro/http` forwards for a site. An empty list serves locally. The
    /// site owner is authenticated from the manifest. Owner-authenticated.
    pub async fn set_placement(
        &self,
        site_id: &str,
        serving_nodes: Vec<String>,
    ) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("site_id".into(), json!(site_id));
        params.insert("serving_nodes".into(), json!(serving_nodes));
        self.envelope_into(&mut params);
        self.rpc.call("tenzro_siteSetPlacement", Value::Object(params)).await
    }

    /// Get the ingress placement record for a site.
    pub async fn get_placement(&self, site_id: &str) -> crate::error::SdkResult<Value> {
        self.rpc
            .call("tenzro_siteGetPlacement", json!({ "site_id": site_id }))
            .await
    }

    /// List all ingress placement records.
    pub async fn list_placements(&self) -> crate::error::SdkResult<Value> {
        self.rpc.call("tenzro_listSitePlacements", json!({})).await
    }

    /// Remove a site's ingress placement (reverts to local serving).
    /// Owner-authenticated.
    pub async fn remove_placement(&self, site_id: &str) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("site_id".into(), json!(site_id));
        self.envelope_into(&mut params);
        self.rpc.call("tenzro_siteRemovePlacement", Value::Object(params)).await
    }

    // ---- Custom domains ---------------------------------------------------

    /// Claim a custom domain for a site. Returns the DNS records the owner must
    /// publish (`dns_records` + `ownership_txt_name`). Owner-authenticated.
    pub async fn claim_domain(
        &self,
        hostname: &str,
        site_id: &str,
        owner_did: &str,
    ) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("hostname".into(), json!(hostname));
        params.insert("site_id".into(), json!(site_id));
        params.insert("owner_did".into(), json!(owner_did));
        self.envelope_into(&mut params);
        self.rpc.call("tenzro_siteClaimDomain", Value::Object(params)).await
    }

    /// Verify a claimed domain against its published DNS TXT proof.
    /// Owner-authenticated.
    pub async fn verify_domain(
        &self,
        hostname: &str,
        owner_did: &str,
    ) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("hostname".into(), json!(hostname));
        params.insert("owner_did".into(), json!(owner_did));
        self.envelope_into(&mut params);
        self.rpc.call("tenzro_siteVerifyDomain", Value::Object(params)).await
    }

    /// Get a custom-domain record (status + DNS records).
    pub async fn get_domain(&self, hostname: &str) -> crate::error::SdkResult<Value> {
        self.rpc
            .call("tenzro_siteGetDomain", json!({ "hostname": hostname }))
            .await
    }

    /// List custom domains, optionally filtered by owner DID.
    pub async fn list_domains(&self, owner_did: Option<&str>) -> crate::error::SdkResult<Value> {
        let params = match owner_did {
            Some(o) => json!({ "owner_did": o }),
            None => json!({}),
        };
        self.rpc.call("tenzro_listSiteDomains", params).await
    }

    /// Remove a custom domain. Owner-authenticated.
    pub async fn remove_domain(
        &self,
        hostname: &str,
        owner_did: &str,
    ) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("hostname".into(), json!(hostname));
        params.insert("owner_did".into(), json!(owner_did));
        self.envelope_into(&mut params);
        self.rpc.call("tenzro_siteRemoveDomain", Value::Object(params)).await
    }

    // ---- Functions (wasi:http components) ---------------------------------

    /// Deploy a `wasi:http` function from a component blob hash. Returns a
    /// builder for the optional capabilities / fuel / deadline / pricing /
    /// placement fields.
    pub fn deploy_function(
        &self,
        name: impl Into<String>,
        owner_did: impl Into<String>,
        wasm_blob_hash: impl Into<String>,
    ) -> DeployFunctionBuilder<'_> {
        DeployFunctionBuilder {
            client: self,
            name: name.into(),
            owner_did: owner_did.into(),
            wasm_blob_hash: wasm_blob_hash.into(),
            capabilities: None,
            fuel_limit: None,
            deadline_ms: None,
            price_per_request: None,
            replicas: None,
            region_hint: None,
            max_price_per_hour: None,
        }
    }

    /// Fetch a function deployment by id.
    pub async fn get_function(&self, id: &str) -> crate::error::SdkResult<Value> {
        self.rpc
            .call("tenzro_functionGet", json!({ "id": id }))
            .await
    }

    /// List function deployments, optionally filtered by owner DID.
    pub async fn list_functions(&self, owner_did: Option<&str>) -> crate::error::SdkResult<Value> {
        let params = match owner_did {
            Some(o) => json!({ "owner_did": o }),
            None => json!({}),
        };
        self.rpc.call("tenzro_listFunctions", params).await
    }

    /// Remove a function deployment. Owner-authenticated.
    pub async fn remove_function(&self, id: &str, owner_did: &str) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("id".into(), json!(id));
        params.insert("owner_did".into(), json!(owner_did));
        self.envelope_into(&mut params);
        self.rpc.call("tenzro_functionRemove", Value::Object(params)).await
    }

    // ---- Machines (Firecracker microVMs) ----------------------------------

    /// Deploy a microVM from an image artifact CAID and the loopback port the
    /// guest server listens on. Returns a builder for resources / sealed env /
    /// TEE / pricing / placement.
    pub fn deploy_machine(
        &self,
        name: impl Into<String>,
        owner_did: impl Into<String>,
        artifact_caid: impl Into<String>,
        internal_port: u16,
    ) -> DeployMachineBuilder<'_> {
        DeployMachineBuilder {
            client: self,
            name: name.into(),
            owner_did: owner_did.into(),
            artifact_caid: artifact_caid.into(),
            internal_port,
            resources: None,
            sealed_env: None,
            tee_required: false,
            price_per_request: None,
            replicas: None,
            region_hint: None,
            max_price_per_hour: None,
        }
    }

    /// Fetch a machine deployment by id.
    pub async fn get_machine(&self, id: &str) -> crate::error::SdkResult<Value> {
        self.rpc
            .call("tenzro_machineGet", json!({ "id": id }))
            .await
    }

    /// List machine deployments, optionally filtered by owner DID.
    pub async fn list_machines(&self, owner_did: Option<&str>) -> crate::error::SdkResult<Value> {
        let params = match owner_did {
            Some(o) => json!({ "owner_did": o }),
            None => json!({}),
        };
        self.rpc.call("tenzro_listMachines", params).await
    }

    /// Remove a machine deployment (stops any running microVM first).
    /// Owner-authenticated.
    pub async fn remove_machine(&self, id: &str, owner_did: &str) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("id".into(), json!(id));
        params.insert("owner_did".into(), json!(owner_did));
        self.envelope_into(&mut params);
        self.rpc.call("tenzro_machineRemove", Value::Object(params)).await
    }

    /// Live run-state of a machine deployment.
    pub async fn machine_status(&self, id: &str) -> crate::error::SdkResult<Value> {
        self.rpc
            .call("tenzro_machineStatus", json!({ "id": id }))
            .await
    }

    /// This node's X25519 machine-sealing public key. Wrap each environment
    /// secret to this key before `deploy_machine` so plaintext never leaves the
    /// deployer.
    pub async fn machine_sealing_key(&self) -> crate::error::SdkResult<Value> {
        self.rpc.call("tenzro_machineSealingKey", json!({})).await
    }

    // ---- Placement leases -------------------------------------------------

    /// All placement leases across every hosted app on this node's scheduler.
    pub async fn list_leases(&self) -> crate::error::SdkResult<Value> {
        self.rpc.call("tenzro_listLeases", json!({})).await
    }

    /// Placement leases for a single app (site / function / machine) id.
    pub async fn leases_for_app(&self, app_id: &str) -> crate::error::SdkResult<Value> {
        self.rpc
            .call("tenzro_getLeasesForApp", json!({ "app_id": app_id }))
            .await
    }
}

/// Builder for [`HostingClient::publish_site`].
pub struct PublishSiteBuilder<'a> {
    client: &'a HostingClient,
    name: String,
    owner_did: String,
    routes: Vec<SiteRoute>,
    index_path: Option<String>,
    not_found_path: Option<String>,
    spa: bool,
    price_per_request: Option<u128>,
    replicas: Option<usize>,
    region_hint: Option<String>,
    max_price_per_hour: Option<u128>,
}

impl PublishSiteBuilder<'_> {
    /// Override the default index path (`/index.html`).
    pub fn index_path(mut self, path: impl Into<String>) -> Self {
        self.index_path = Some(path.into());
        self
    }
    /// Route served for unmatched paths (custom 404 page).
    pub fn not_found_path(mut self, path: impl Into<String>) -> Self {
        self.not_found_path = Some(path.into());
        self
    }
    /// Serve the index for unmatched non-asset paths (single-page-app routing).
    pub fn spa(mut self, spa: bool) -> Self {
        self.spa = spa;
        self
    }
    /// x402 price per request in base TNZO units.
    pub fn price_per_request(mut self, price: u128) -> Self {
        self.price_per_request = Some(price);
        self
    }
    /// Number of serving replicas to place.
    pub fn replicas(mut self, replicas: usize) -> Self {
        self.replicas = Some(replicas);
        self
    }
    /// Preferred region for placement.
    pub fn region_hint(mut self, region: impl Into<String>) -> Self {
        self.region_hint = Some(region.into());
        self
    }
    /// Maximum price per hour a serving node may charge.
    pub fn max_price_per_hour(mut self, price: u128) -> Self {
        self.max_price_per_hour = Some(price);
        self
    }

    /// Submit the publish request.
    pub async fn call(self) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("name".into(), json!(self.name));
        params.insert("owner_did".into(), json!(self.owner_did));
        params.insert("routes".into(), serde_json::to_value(&self.routes).unwrap_or(Value::Null));
        if let Some(i) = self.index_path {
            params.insert("index_path".into(), json!(i));
        }
        if let Some(n) = self.not_found_path {
            params.insert("not_found_path".into(), json!(n));
        }
        params.insert("spa".into(), json!(self.spa));
        if let Some(p) = self.price_per_request {
            params.insert("price_per_request".into(), json!(p.to_string()));
        }
        if let Some(r) = self.replicas {
            params.insert("replicas".into(), json!(r));
        }
        if let Some(r) = self.region_hint {
            params.insert("region_hint".into(), json!(r));
        }
        if let Some(m) = self.max_price_per_hour {
            params.insert("max_price_per_hour".into(), json!(m.to_string()));
        }
        self.client.envelope_into(&mut params);
        self.client
            .rpc
            .call("tenzro_sitePublish", Value::Object(params))
            .await
    }
}

/// Builder for [`HostingClient::deploy_function`].
pub struct DeployFunctionBuilder<'a> {
    client: &'a HostingClient,
    name: String,
    owner_did: String,
    wasm_blob_hash: String,
    capabilities: Option<Value>,
    fuel_limit: Option<u64>,
    deadline_ms: Option<u64>,
    price_per_request: Option<u128>,
    replicas: Option<usize>,
    region_hint: Option<String>,
    max_price_per_hour: Option<u128>,
}

impl DeployFunctionBuilder<'_> {
    /// Capability manifest (host imports the component is granted).
    pub fn capabilities(mut self, capabilities: Value) -> Self {
        self.capabilities = Some(capabilities);
        self
    }
    /// wasmtime fuel limit per request.
    pub fn fuel_limit(mut self, fuel: u64) -> Self {
        self.fuel_limit = Some(fuel);
        self
    }
    /// Wall-clock deadline per request, in milliseconds.
    pub fn deadline_ms(mut self, deadline: u64) -> Self {
        self.deadline_ms = Some(deadline);
        self
    }
    /// x402 price per request in base TNZO units.
    pub fn price_per_request(mut self, price: u128) -> Self {
        self.price_per_request = Some(price);
        self
    }
    /// Number of serving replicas to place.
    pub fn replicas(mut self, replicas: usize) -> Self {
        self.replicas = Some(replicas);
        self
    }
    /// Preferred region for placement.
    pub fn region_hint(mut self, region: impl Into<String>) -> Self {
        self.region_hint = Some(region.into());
        self
    }
    /// Maximum price per hour a serving node may charge.
    pub fn max_price_per_hour(mut self, price: u128) -> Self {
        self.max_price_per_hour = Some(price);
        self
    }

    /// Submit the deploy request.
    pub async fn call(self) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("name".into(), json!(self.name));
        params.insert("owner_did".into(), json!(self.owner_did));
        params.insert("wasm_blob_hash".into(), json!(self.wasm_blob_hash));
        if let Some(c) = self.capabilities {
            params.insert("capabilities".into(), c);
        }
        if let Some(f) = self.fuel_limit {
            params.insert("fuel_limit".into(), json!(f));
        }
        if let Some(d) = self.deadline_ms {
            params.insert("deadline_ms".into(), json!(d));
        }
        if let Some(p) = self.price_per_request {
            params.insert("price_per_request".into(), json!(p.to_string()));
        }
        if let Some(r) = self.replicas {
            params.insert("replicas".into(), json!(r));
        }
        if let Some(r) = self.region_hint {
            params.insert("region_hint".into(), json!(r));
        }
        if let Some(m) = self.max_price_per_hour {
            params.insert("max_price_per_hour".into(), json!(m.to_string()));
        }
        self.client.envelope_into(&mut params);
        self.client
            .rpc
            .call("tenzro_functionDeploy", Value::Object(params))
            .await
    }
}

/// Builder for [`HostingClient::deploy_machine`].
pub struct DeployMachineBuilder<'a> {
    client: &'a HostingClient,
    name: String,
    owner_did: String,
    artifact_caid: String,
    internal_port: u16,
    resources: Option<Value>,
    sealed_env: Option<Value>,
    tee_required: bool,
    price_per_request: Option<u128>,
    replicas: Option<usize>,
    region_hint: Option<String>,
    max_price_per_hour: Option<u128>,
}

impl DeployMachineBuilder<'_> {
    /// Resource request (`vcpus`, `mem_mib`, `disk_mib`).
    pub fn resources(mut self, resources: Value) -> Self {
        self.resources = Some(resources);
        self
    }
    /// Sealed environment variables (each ciphertext wrapped to the node's
    /// sealing key — see [`HostingClient::machine_sealing_key`]).
    pub fn sealed_env(mut self, sealed_env: Value) -> Self {
        self.sealed_env = Some(sealed_env);
        self
    }
    /// Require a TEE-capable serving node.
    pub fn tee_required(mut self, required: bool) -> Self {
        self.tee_required = required;
        self
    }
    /// x402 price per request in base TNZO units.
    pub fn price_per_request(mut self, price: u128) -> Self {
        self.price_per_request = Some(price);
        self
    }
    /// Number of serving replicas to place.
    pub fn replicas(mut self, replicas: usize) -> Self {
        self.replicas = Some(replicas);
        self
    }
    /// Preferred region for placement.
    pub fn region_hint(mut self, region: impl Into<String>) -> Self {
        self.region_hint = Some(region.into());
        self
    }
    /// Maximum price per hour a serving node may charge.
    pub fn max_price_per_hour(mut self, price: u128) -> Self {
        self.max_price_per_hour = Some(price);
        self
    }

    /// Submit the deploy request.
    pub async fn call(self) -> crate::error::SdkResult<Value> {
        let mut params = Map::new();
        params.insert("name".into(), json!(self.name));
        params.insert("owner_did".into(), json!(self.owner_did));
        params.insert("artifact_caid".into(), json!(self.artifact_caid));
        params.insert("internal_port".into(), json!(self.internal_port));
        if let Some(r) = self.resources {
            params.insert("resources".into(), r);
        }
        if let Some(s) = self.sealed_env {
            params.insert("sealed_env".into(), s);
        }
        params.insert("tee_required".into(), json!(self.tee_required));
        if let Some(p) = self.price_per_request {
            params.insert("price_per_request".into(), json!(p.to_string()));
        }
        if let Some(r) = self.replicas {
            params.insert("replicas".into(), json!(r));
        }
        if let Some(r) = self.region_hint {
            params.insert("region_hint".into(), json!(r));
        }
        if let Some(m) = self.max_price_per_hour {
            params.insert("max_price_per_hour".into(), json!(m.to_string()));
        }
        self.client.envelope_into(&mut params);
        self.client
            .rpc
            .call("tenzro_machineDeploy", Value::Object(params))
            .await
    }
}
