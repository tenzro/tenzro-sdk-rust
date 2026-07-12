//! Iroh consumer surface SDK for Tenzro Network
//!
//! Wraps the `tenzro_iroh_*` JSON-RPC namespace exposed by `tenzro-node`,
//! which fronts the shared `IrohBackedResolver`. The same QUIC + Pkarr +
//! iroh-blobs substrate backs the storage DA backend, training outer-gradient
//! distribution, confidential sealed-shard distribution, model-weight peer
//! fetch, the agent-memory archive DA path, and A2A JSON-RPC over the
//! `tenzro/a2a` ALPN.
//!
//! # Example
//!
//! ```no_run
//! # use tenzro_sdk::TenzroClient;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = TenzroClient::new("https://rpc.tenzro.xyz").await?;
//! let iroh = client.iroh();
//!
//! let info = iroh.get_info().await?;
//! println!("endpoint_id: {}", info.endpoint_id);
//! println!("alpns:       {:?}", info.alpns);
//!
//! // Publish a local payload, get a tenzro:// URI back
//! let uri = iroh.publish_blob(b"hello world".to_vec()).await?;
//! println!("published: {}", uri.tenzro_uri);
//!
//! // Fetch any tenzro:// URI (blob / model / gradient / shard / receipt)
//! let bytes = iroh.fetch_blob(&uri.tenzro_uri).await?;
//! assert_eq!(bytes, b"hello world");
//! # Ok(())
//! # }
//! ```

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Iroh content-addressed transport client.
#[derive(Clone)]
pub struct IrohClient {
    rpc: Arc<RpcClient>,
}

impl IrohClient {
    /// Creates a new Iroh client.
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Endpoint id, Pkarr relay, and bound ALPNs for this node.
    pub async fn get_info(&self) -> SdkResult<IrohInfo> {
        self.rpc
            .call("tenzro_iroh_getInfo", serde_json::json!({}))
            .await
    }

    /// Just the iroh `EndpointId` (z-base-32 form + 32-byte hex).
    pub async fn get_endpoint_id(&self) -> SdkResult<IrohEndpointId> {
        self.rpc
            .call("tenzro_iroh_getEndpointId", serde_json::json!({}))
            .await
    }

    /// ALPNs registered on the shared iroh router.
    pub async fn list_alpns(&self) -> SdkResult<IrohAlpnList> {
        self.rpc
            .call("tenzro_iroh_listAlpns", serde_json::json!({}))
            .await
    }

    /// Publish raw bytes as a `tenzro://blob/<blake3-hex>` URI.
    pub async fn publish_blob(&self, bytes: Vec<u8>) -> SdkResult<IrohPublishResult> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        self.rpc
            .call(
                "tenzro_iroh_publishBlob",
                serde_json::json!({ "bytes_b64": b64 }),
            )
            .await
    }

    /// Fetch a `tenzro://{blob,model,gradient,shard,receipt}/...` URI to raw
    /// bytes. The dispatcher resolves the variant to the underlying iroh-blobs
    /// hash via the shared `IrohBackedResolver`.
    pub async fn fetch_blob(&self, tenzro_uri: &str) -> SdkResult<Vec<u8>> {
        let resp: IrohFetchResponse = self
            .rpc
            .call(
                "tenzro_iroh_fetchBlob",
                serde_json::json!({ "tenzro_uri": tenzro_uri }),
            )
            .await?;
        base64::engine::general_purpose::STANDARD
            .decode(&resp.bytes_b64)
            .map_err(|e| SdkError::RpcError(format!("iroh bytes_b64 decode: {e}")))
    }

    /// Resolve any `tenzro://...` URI through the iroh resolver — alias for
    /// [`fetch_blob`] with a more descriptive name for code that thinks of
    /// the URI as a name rather than a blob handle.
    pub async fn resolve(&self, tenzro_uri: &str) -> SdkResult<Vec<u8>> {
        self.fetch_blob(tenzro_uri).await
    }
}

/// Result of `tenzro_iroh_getInfo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohInfo {
    /// z-base-32 iroh `EndpointId` (byte-identical to the TDIP Ed25519 key).
    #[serde(default)]
    pub endpoint_id: String,
    /// 32-byte endpoint id, hex-encoded.
    #[serde(default)]
    pub endpoint_id_hex: String,
    /// Pkarr relay this node publishes its endpoint record to (when configured).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pkarr_relay: Option<String>,
    /// Whether iroh-docs collaborative replicas are enabled on this node.
    #[serde(default)]
    pub docs_enabled: bool,
    /// All ALPNs bound on the shared iroh router.
    #[serde(default)]
    pub alpns: Vec<String>,
}

/// Result of `tenzro_iroh_getEndpointId`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohEndpointId {
    /// z-base-32 iroh `EndpointId`.
    #[serde(default)]
    pub endpoint_id: String,
    /// 32-byte endpoint id, hex-encoded.
    #[serde(default)]
    pub endpoint_id_hex: String,
}

/// Result of `tenzro_iroh_listAlpns`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohAlpnList {
    /// Bound ALPNs and short descriptions.
    #[serde(default)]
    pub alpns: Vec<IrohAlpnEntry>,
}

/// A single ALPN binding on the shared iroh router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohAlpnEntry {
    /// ALPN identifier (e.g. `iroh-blobs`, `tenzro/a2a`).
    #[serde(default)]
    pub alpn: String,
    /// Short description of the protocol handler bound to this ALPN.
    #[serde(default)]
    pub description: String,
}

/// Result of `tenzro_iroh_publishBlob`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohPublishResult {
    /// `tenzro://blob/<blake3-hex>` URI for the published payload.
    #[serde(default)]
    pub tenzro_uri: String,
    /// Raw BLAKE3 hash, hex-encoded.
    #[serde(default)]
    pub blake3_hex: String,
    /// Byte length of the published payload.
    #[serde(default)]
    pub size_bytes: u64,
}

/// Raw `tenzro_iroh_fetchBlob` response — the SDK exposes the decoded bytes
/// via [`IrohClient::fetch_blob`] instead of this base64 envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IrohFetchResponse {
    #[serde(default)]
    bytes_b64: String,
}
