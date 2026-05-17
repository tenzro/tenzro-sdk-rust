//! NFT Management SDK for Tenzro Network
//!
//! This module provides NFT operations including creating collections, minting,
//! transferring, and querying NFT information across all supported VMs.
//!
//! NFT collections support cross-VM pointer registration via the Sei V2 pointer
//! model, allowing the same collection to be accessed from EVM, SVM, and DAML
//! without bridge risk.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// NFT client for collection and token management
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let nft = client.nft();
///
/// // List collections
/// let collections = nft.list_collections(None).await?;
/// println!("Found {} collections", collections.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct NftClient {
    rpc: Arc<RpcClient>,
}

impl NftClient {
    /// Creates a new NFT client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Creates a new NFT collection
    ///
    /// Registers a new NFT collection in the unified token registry. The
    /// collection can later be given cross-VM pointers via `register_pointer`.
    ///
    /// # Arguments
    ///
    /// * `name` - Collection name (e.g., "Tenzro Genesis")
    /// * `symbol` - Collection symbol (e.g., "TGEN")
    /// * `nft_type` - NFT standard: "erc721", "erc1155", or "metaplex"
    /// * `creator` - Creator address (hex, with or without 0x prefix)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let nft = client.nft();
    /// let collection = nft.create_collection(
    ///     "Tenzro Genesis",
    ///     "TGEN",
    ///     "erc721",
    ///     "0x1234567890abcdef1234567890abcdef12345678",
    /// ).await?;
    /// println!("Collection created: {} ({})", collection.name, collection.collection_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_collection(
        &self,
        name: &str,
        symbol: &str,
        nft_type: &str,
        creator: &str,
    ) -> SdkResult<CollectionInfo> {
        self.rpc
            .call(
                "tenzro_createNftCollection",
                serde_json::json!([{
                    "name": name,
                    "symbol": symbol,
                    "nft_type": nft_type,
                    "creator": creator,
                }]),
            )
            .await
    }

    /// Mints a new NFT in an existing collection
    ///
    /// # Arguments
    ///
    /// * `collection_id` - Collection identifier (hex)
    /// * `token_id` - Token ID within the collection (decimal string)
    /// * `recipient` - Recipient address (hex)
    /// * `metadata_uri` - URI pointing to token metadata (e.g., IPFS or HTTPS)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let nft = client.nft();
    /// let result = nft.mint_nft(
    ///     "0xabc123...",
    ///     "1",
    ///     "0xdef456...",
    ///     "ipfs://QmXyz.../metadata.json",
    /// ).await?;
    /// println!("Minted: token {} (tx: {})", result.token_id, result.tx_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn mint_nft(
        &self,
        collection_id: &str,
        token_id: &str,
        recipient: &str,
        metadata_uri: &str,
    ) -> SdkResult<MintResult> {
        self.rpc
            .call(
                "tenzro_mintNft",
                serde_json::json!([{
                    "collection_id": collection_id,
                    "token_id": token_id,
                    "recipient": recipient,
                    "metadata_uri": metadata_uri,
                }]),
            )
            .await
    }

    /// Transfers an NFT between addresses
    ///
    /// # Arguments
    ///
    /// * `collection_id` - Collection identifier (hex)
    /// * `token_id` - Token ID within the collection (decimal string)
    /// * `from` - Sender address (hex)
    /// * `to` - Recipient address (hex)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let nft = client.nft();
    /// let result = nft.transfer_nft(
    ///     "0xabc123...",
    ///     "1",
    ///     "0xsender...",
    ///     "0xrecipient...",
    /// ).await?;
    /// println!("Transfer status: {}", result.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn transfer_nft(
        &self,
        collection_id: &str,
        token_id: &str,
        from: &str,
        to: &str,
    ) -> SdkResult<NftTransferResult> {
        self.rpc
            .call(
                "tenzro_transferNft",
                serde_json::json!([{
                    "collection_id": collection_id,
                    "token_id": token_id,
                    "from": from,
                    "to": to,
                }]),
            )
            .await
    }

    /// Gets NFT information by collection ID and optional token ID
    ///
    /// When `token_id` is `None`, returns collection-level information.
    /// When `token_id` is provided, returns the specific token's metadata.
    ///
    /// # Arguments
    ///
    /// * `collection_id` - Collection identifier (hex)
    /// * `token_id` - Optional token ID within the collection
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let nft = client.nft();
    ///
    /// // Get specific token info
    /// let info = nft.get_nft_info("0xabc123...", Some("1")).await?;
    /// println!("Owner: {}", info.owner);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_nft_info(
        &self,
        collection_id: &str,
        token_id: Option<&str>,
    ) -> SdkResult<NftInfo> {
        let mut params = serde_json::json!({
            "collection_id": collection_id,
        });

        if let Some(tid) = token_id {
            params["token_id"] = serde_json::json!(tid);
        }

        self.rpc
            .call("tenzro_getNftInfo", serde_json::json!([params]))
            .await
    }

    /// Lists NFT collections with an optional creator filter
    ///
    /// # Arguments
    ///
    /// * `creator` - Optional creator address to filter by
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let nft = client.nft();
    /// let collections = nft.list_collections(None).await?;
    /// for c in &collections {
    ///     println!("{}: {} ({} minted)", c.collection_id, c.name, c.total_supply);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_collections(
        &self,
        creator: Option<&str>,
    ) -> SdkResult<Vec<CollectionInfo>> {
        let mut params = serde_json::Map::new();

        if let Some(c) = creator {
            params.insert("creator".to_string(), serde_json::json!(c));
        }

        self.rpc
            .call(
                "tenzro_listNftCollections",
                serde_json::json!([serde_json::Value::Object(params)]),
            )
            .await
    }

    /// Registers a cross-VM pointer for an NFT collection
    ///
    /// Creates a pointer on the target VM that maps to the same underlying
    /// collection data, using the Sei V2 pointer model for zero-bridge-risk
    /// cross-VM access.
    ///
    /// # Arguments
    ///
    /// * `collection_id` - Collection identifier (hex)
    /// * `target_vm` - Target VM: "evm", "svm", or "daml"
    /// * `target_address` - Address on the target VM to register the pointer at
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let nft = client.nft();
    /// let result = nft.register_pointer(
    ///     "0xabc123...",
    ///     "evm",
    ///     "0x7a4bcb13...",
    /// ).await?;
    /// println!("Pointer registered: {}", result.pointer_address);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register_pointer(
        &self,
        collection_id: &str,
        target_vm: &str,
        target_address: &str,
    ) -> SdkResult<PointerResult> {
        self.rpc
            .call(
                "tenzro_registerNftPointer",
                serde_json::json!([{
                    "collection_id": collection_id,
                    "target_vm": target_vm,
                    "target_address": target_address,
                }]),
            )
            .await
    }
}

/// NFT collection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    /// Collection identifier (hex)
    #[serde(default)]
    pub collection_id: String,
    /// Collection name
    #[serde(default)]
    pub name: String,
    /// Collection symbol
    #[serde(default)]
    pub symbol: String,
    /// NFT standard (e.g., "erc721", "erc1155", "metaplex")
    #[serde(default)]
    pub nft_type: String,
    /// Creator address (hex)
    #[serde(default)]
    pub creator: String,
    /// Total number of minted tokens
    #[serde(default)]
    pub total_supply: u64,
    /// Operation status (e.g., "created", "active")
    #[serde(default)]
    pub status: String,
}

/// Result from minting an NFT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintResult {
    /// Collection identifier (hex)
    #[serde(default)]
    pub collection_id: String,
    /// Minted token ID
    #[serde(default)]
    pub token_id: String,
    /// Recipient address (hex)
    #[serde(default)]
    pub recipient: String,
    /// Metadata URI
    #[serde(default)]
    pub metadata_uri: String,
    /// Transaction hash
    #[serde(default)]
    pub tx_hash: String,
    /// Operation status
    #[serde(default)]
    pub status: String,
}

/// Result from transferring an NFT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftTransferResult {
    /// Collection identifier (hex)
    #[serde(default)]
    pub collection_id: String,
    /// Token ID
    #[serde(default)]
    pub token_id: String,
    /// Sender address (hex)
    #[serde(default)]
    pub from: String,
    /// Recipient address (hex)
    #[serde(default)]
    pub to: String,
    /// Transaction hash
    #[serde(default)]
    pub tx_hash: String,
    /// Operation status
    #[serde(default)]
    pub status: String,
}

/// NFT information for a specific token or collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftInfo {
    /// Collection identifier (hex)
    #[serde(default)]
    pub collection_id: String,
    /// Token ID (empty for collection-level queries)
    #[serde(default)]
    pub token_id: String,
    /// Current owner address (hex)
    #[serde(default)]
    pub owner: String,
    /// Metadata URI
    #[serde(default)]
    pub metadata_uri: String,
    /// Collection name
    #[serde(default)]
    pub name: String,
    /// Collection symbol
    #[serde(default)]
    pub symbol: String,
    /// NFT standard
    #[serde(default)]
    pub nft_type: String,
}

/// Result from registering a cross-VM pointer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointerResult {
    /// Collection identifier (hex)
    #[serde(default)]
    pub collection_id: String,
    /// Target VM
    #[serde(default)]
    pub target_vm: String,
    /// Pointer address on the target VM
    #[serde(default)]
    pub pointer_address: String,
    /// Operation status
    #[serde(default)]
    pub status: String,
}
