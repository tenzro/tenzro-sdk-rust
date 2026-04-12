//! Token SDK for Tenzro Network
//!
//! This module provides token management functionality including creating tokens,
//! querying token info and balances, wrapping TNZO, and cross-VM transfers.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The deterministic CREATE2 address of the wTNZO ERC-20 pointer contract on EVM.
/// All VMs (EVM, SVM, DAML) share the same underlying native TNZO balance via
/// the Sei V2 pointer model -- this address is the EVM representation.
pub const WTNZO_EVM_ADDRESS: &str = "0x7a4bcb13a6b2b384c284b5caa6e5ef3126527f93";

/// Token client for TNZO token registry and cross-VM operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let token = client.token();
///
/// // List all tokens
/// let result = token.list_tokens(None, None).await?;
/// println!("Found {} tokens", result.count);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct TokenClient {
    rpc: Arc<RpcClient>,
}

impl TokenClient {
    /// Creates a new token client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Creates a new ERC-20 token via the unified token registry
    ///
    /// The token is registered across all VMs (EVM, SVM, DAML) using the
    /// Sei V2 pointer model for zero-bridge-risk cross-VM access.
    ///
    /// # Arguments
    ///
    /// * `name` - Token name (e.g., "My Token")
    /// * `symbol` - Token symbol (e.g., "MTK")
    /// * `creator` - Creator address (hex, with or without 0x prefix)
    /// * `initial_supply` - Initial token supply as a decimal string
    /// * `decimals` - Token decimals (default 18 if None)
    /// * `mintable` - Whether additional tokens can be minted
    /// * `burnable` - Whether tokens can be burned
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let token = client.token();
    /// let info = token.create_token(
    ///     "My Token",
    ///     "MTK",
    ///     "0x1234567890abcdef1234567890abcdef12345678",
    ///     "1000000000000000000000000", // 1M tokens with 18 decimals
    ///     Some(18),
    ///     true,
    ///     true,
    /// ).await?;
    /// println!("Created token: {} ({})", info.name, info.symbol);
    /// println!("EVM address: {}", info.evm_address);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_token(
        &self,
        name: &str,
        symbol: &str,
        creator: &str,
        initial_supply: &str,
        decimals: Option<u8>,
        mintable: bool,
        burnable: bool,
    ) -> SdkResult<TokenInfo> {
        let mut params = serde_json::json!({
            "name": name,
            "symbol": symbol,
            "creator": creator,
            "initial_supply": initial_supply,
            "mintable": mintable,
            "burnable": burnable,
        });

        if let Some(d) = decimals {
            params["decimals"] = serde_json::json!(d);
        }

        self.rpc
            .call("tenzro_createToken", serde_json::json!([params]))
            .await
    }

    /// Gets token information by symbol, EVM address, or token ID
    ///
    /// At least one of `symbol`, `evm_address`, or `token_id` must be provided.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Token symbol (e.g., "TNZO")
    /// * `evm_address` - EVM contract address (hex)
    /// * `token_id` - Token registry ID (hex)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let token = client.token();
    /// let info = token.get_token_info(Some("TNZO"), None, None).await?;
    /// println!("Total supply: {}", info.total_supply);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_token_info(
        &self,
        symbol: Option<&str>,
        evm_address: Option<&str>,
        token_id: Option<&str>,
    ) -> SdkResult<TokenInfo> {
        let mut params = serde_json::Map::new();

        if let Some(s) = symbol {
            params.insert("symbol".to_string(), serde_json::json!(s));
        }
        if let Some(a) = evm_address {
            params.insert("evm_address".to_string(), serde_json::json!(a));
        }
        if let Some(id) = token_id {
            params.insert("token_id".to_string(), serde_json::json!(id));
        }

        self.rpc
            .call(
                "tenzro_getToken",
                serde_json::json!([serde_json::Value::Object(params)]),
            )
            .await
    }

    /// Lists registered tokens with optional VM type filter
    ///
    /// # Arguments
    ///
    /// * `vm_type` - Filter by VM type: "evm", "svm", "daml", or "native"
    /// * `limit` - Maximum number of tokens to return (default 50, max 100)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let token = client.token();
    ///
    /// // List all EVM tokens
    /// let result = token.list_tokens(Some("evm"), Some(20)).await?;
    /// for t in &result.tokens {
    ///     println!("{}: {} (supply: {})", t.symbol, t.name, t.total_supply);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_tokens(
        &self,
        vm_type: Option<&str>,
        limit: Option<u64>,
    ) -> SdkResult<TokenList> {
        let mut params = serde_json::Map::new();

        if let Some(vt) = vm_type {
            params.insert("vm_type".to_string(), serde_json::json!(vt));
        }
        if let Some(l) = limit {
            params.insert("limit".to_string(), serde_json::json!(l));
        }

        self.rpc
            .call(
                "tenzro_listTokens",
                serde_json::json!([serde_json::Value::Object(params)]),
            )
            .await
    }

    /// Gets the TNZO token balance for an address across all VMs
    ///
    /// Returns native TNZO balance along with EVM (wTNZO ERC-20),
    /// SVM (wTNZO SPL with 9 decimals), and DAML (CIP-56 holding)
    /// representations. All share the same underlying balance via the
    /// pointer model.
    ///
    /// # Arguments
    ///
    /// * `address` - Account address (hex, with or without 0x prefix)
    /// * `token` - Optional token symbol (defaults to TNZO if None)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let token = client.token();
    /// let balance = token.get_token_balance(
    ///     "0x1234567890abcdef1234567890abcdef12345678",
    ///     None,
    /// ).await?;
    /// println!("Native: {}", balance.native.display);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_token_balance(
        &self,
        address: &str,
        token: Option<&str>,
    ) -> SdkResult<TokenBalance> {
        let mut params = serde_json::json!({
            "address": address,
        });

        if let Some(t) = token {
            params["token"] = serde_json::json!(t);
        }

        self.rpc
            .call("tenzro_getTokenBalance", serde_json::json!([params]))
            .await
    }

    /// Wraps native TNZO to a VM-specific representation
    ///
    /// In the pointer model, wrapping is a no-op -- native TNZO and VM
    /// representations share the same underlying balance. This call
    /// verifies the balance and returns the VM representation details.
    ///
    /// # Arguments
    ///
    /// * `address` - Account address (hex)
    /// * `amount` - Amount to wrap as a decimal string
    /// * `to_vm` - Target VM: "evm", "svm", or "daml"
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let token = client.token();
    /// let result = token.wrap_tnzo(
    ///     "0x1234567890abcdef1234567890abcdef12345678",
    ///     "1000000000000000000", // 1 TNZO
    ///     "evm",
    /// ).await?;
    /// println!("Representation: {}", result.representation);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wrap_tnzo(
        &self,
        address: &str,
        amount: &str,
        to_vm: &str,
    ) -> SdkResult<WrapResult> {
        self.rpc
            .call(
                "tenzro_wrapTnzo",
                serde_json::json!([{
                    "address": address,
                    "amount": amount,
                    "to_vm": to_vm,
                }]),
            )
            .await
    }

    /// Performs an atomic cross-VM token transfer
    ///
    /// Transfers tokens between VMs (EVM, SVM, DAML, Native) using the
    /// Sei V2 pointer model. For TNZO, all VM representations share
    /// the same native balance, so no bridge risk exists.
    ///
    /// # Arguments
    ///
    /// * `token` - Token symbol (e.g., "TNZO")
    /// * `amount` - Amount to transfer as a decimal string
    /// * `from_vm` - Source VM: "evm", "svm", "daml", or "native"
    /// * `to_vm` - Target VM: "evm", "svm", "daml", or "native"
    /// * `from_address` - Source address (hex)
    /// * `to_address` - Destination address (hex)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let token = client.token();
    /// let result = token.cross_vm_transfer(
    ///     "TNZO",
    ///     "1000000000000000000", // 1 TNZO
    ///     "native",
    ///     "evm",
    ///     "0xabc...",
    ///     "0xdef...",
    /// ).await?;
    /// println!("Status: {}", result.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cross_vm_transfer(
        &self,
        token: &str,
        amount: &str,
        from_vm: &str,
        to_vm: &str,
        from_address: &str,
        to_address: &str,
    ) -> SdkResult<TransferResult> {
        self.rpc
            .call(
                "tenzro_crossVmTransfer",
                serde_json::json!([{
                    "token": token,
                    "amount": amount,
                    "from_vm": from_vm,
                    "to_vm": to_vm,
                    "from_address": from_address,
                    "to_address": to_address,
                }]),
            )
            .await
    }
}

/// Token information returned from `create_token` and `get_token_info`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Token registry ID (hex)
    #[serde(default)]
    pub token_id: String,
    /// Token name
    #[serde(default)]
    pub name: String,
    /// Token symbol
    #[serde(default)]
    pub symbol: String,
    /// Number of decimals
    #[serde(default)]
    pub decimals: u8,
    /// Total supply (decimal string)
    #[serde(default)]
    pub total_supply: String,
    /// Initial supply (decimal string, present on create)
    #[serde(default)]
    pub initial_supply: String,
    /// Token type (e.g., "Erc20")
    #[serde(default)]
    pub token_type: String,
    /// EVM contract address (hex)
    #[serde(default)]
    pub evm_address: String,
    /// SVM mint address (hex)
    #[serde(default)]
    pub svm_mint: String,
    /// Creator address (hex)
    #[serde(default)]
    pub creator: String,
    /// Operation status (e.g., "created")
    #[serde(default)]
    pub status: String,
}

/// Result from `list_tokens`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenList {
    /// Number of tokens returned
    #[serde(default)]
    pub count: usize,
    /// Token entries
    #[serde(default)]
    pub tokens: Vec<TokenListEntry>,
}

/// A single token entry in a list result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenListEntry {
    /// Token registry ID (hex)
    #[serde(default)]
    pub token_id: String,
    /// Token name
    #[serde(default)]
    pub name: String,
    /// Token symbol
    #[serde(default)]
    pub symbol: String,
    /// Number of decimals
    #[serde(default)]
    pub decimals: u8,
    /// Total supply (decimal string)
    #[serde(default)]
    pub total_supply: String,
    /// EVM contract address (hex)
    #[serde(default)]
    pub evm_address: String,
}

/// Token balance across all VMs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    /// Account address
    #[serde(default)]
    pub address: String,
    /// Native TNZO balance
    #[serde(default)]
    pub native: NativeBalance,
    /// EVM wTNZO ERC-20 balance
    #[serde(default)]
    pub evm_wtnzo: VmBalance,
    /// SVM wTNZO SPL balance (9 decimals)
    #[serde(default)]
    pub svm_wtnzo: VmBalance,
    /// DAML CIP-56 holding
    #[serde(default)]
    pub daml_holding: DamlBalance,
}

/// Native TNZO balance details
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NativeBalance {
    /// Balance in wei (decimal string)
    #[serde(default)]
    pub balance: String,
    /// Number of decimals (18)
    #[serde(default)]
    pub decimals: u8,
    /// Human-readable display (e.g., "100.000000 TNZO")
    #[serde(default)]
    pub display: String,
}

/// VM-specific token balance
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VmBalance {
    /// Balance in smallest unit (decimal string)
    #[serde(default)]
    pub balance: String,
    /// Number of decimals
    #[serde(default)]
    pub decimals: u8,
}

/// DAML CIP-56 holding balance
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DamlBalance {
    /// Holding amount as a DAML Decimal string
    #[serde(default)]
    pub amount: String,
}

/// Result from `wrap_tnzo`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrapResult {
    /// Account address
    #[serde(default)]
    pub address: String,
    /// Amount wrapped (decimal string)
    #[serde(default)]
    pub amount: String,
    /// Target VM
    #[serde(default)]
    pub target_vm: String,
    /// VM representation description
    #[serde(default)]
    pub representation: String,
    /// Native balance after wrap (decimal string)
    #[serde(default)]
    pub native_balance: String,
    /// Operation status
    #[serde(default)]
    pub status: String,
    /// Additional note about pointer model
    #[serde(default)]
    pub note: String,
}

/// Result from `cross_vm_transfer`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    /// Token symbol
    #[serde(default)]
    pub token: String,
    /// Amount transferred (decimal string)
    #[serde(default)]
    pub amount: String,
    /// Source VM
    #[serde(default)]
    pub from_vm: String,
    /// Target VM
    #[serde(default)]
    pub to_vm: String,
    /// Operation status
    #[serde(default)]
    pub status: String,
}
