//! LI.FI Cross-Chain Aggregation SDK for Tenzro Network
//!
//! This module provides direct access to the LI.FI REST API (`https://li.quest/v1/`)
//! for cross-chain token swaps, bridging, and route discovery across 30+ chains and
//! 20+ bridge/DEX aggregators.
//!
//! LI.FI is an external service -- these methods call the LI.FI API directly rather
//! than going through the Tenzro RPC node.
//!
//! # Example
//!
//! ```no_run
//! use tenzro_sdk::lifi::LifiClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = LifiClient::new(None);
//!
//!     // Get supported chains
//!     let chains = client.get_chains().await?;
//!     println!("Supported chains: {}", chains.len());
//!
//!     // Get a quote for swapping ETH to USDC on Arbitrum
//!     let quote = client.get_quote(
//!         "ETH", "USDC", "1", "42161", "42161",
//!         "0xYourAddress...", "0xYourAddress...",
//!     ).await?;
//!     println!("Estimated output: {}", quote.estimate.to_amount);
//!     Ok(())
//! }
//! ```

use crate::error::{SdkError, SdkResult};
use serde::{Deserialize, Serialize};

/// Default LI.FI API base URL
const LIFI_API_BASE: &str = "https://li.quest/v1";

/// LI.FI cross-chain aggregation client
///
/// Calls the LI.FI REST API directly for cross-chain swap quotes,
/// route discovery, token information, and transaction building.
#[derive(Clone)]
pub struct LifiClient {
    http: reqwest::Client,
    base_url: String,
}

impl LifiClient {
    /// Creates a new LI.FI client
    ///
    /// # Arguments
    ///
    /// * `api_key` - Optional LI.FI API key for higher rate limits
    pub fn new(api_key: Option<&str>) -> Self {
        let mut builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30));

        if let Some(key) = api_key {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "x-lifi-api-key",
                reqwest::header::HeaderValue::from_str(key).unwrap_or_else(|_| {
                    reqwest::header::HeaderValue::from_static("")
                }),
            );
            builder = builder.default_headers(headers);
        }

        Self {
            http: builder.build().unwrap_or_else(|_| reqwest::Client::new()),
            base_url: LIFI_API_BASE.to_string(),
        }
    }

    /// Creates a LI.FI client with a custom base URL (for testing)
    pub fn with_base_url(base_url: &str, api_key: Option<&str>) -> Self {
        let mut client = Self::new(api_key);
        client.base_url = base_url.trim_end_matches('/').to_string();
        client
    }

    /// Gets all supported chains
    ///
    /// Returns the list of chains supported by LI.FI including chain IDs,
    /// names, and native token information.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::lifi::LifiClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LifiClient::new(None);
    /// let chains = client.get_chains().await?;
    /// for chain in &chains {
    ///     println!("{}: {} (id: {})", chain.name, chain.native_token.symbol, chain.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_chains(&self) -> SdkResult<Vec<LifiChain>> {
        let url = format!("{}/chains", self.base_url);
        let response: LifiChainsResponse = self.get_json(&url).await?;
        Ok(response.chains)
    }

    /// Gets supported tokens, optionally filtered by chain
    ///
    /// # Arguments
    ///
    /// * `chains` - Optional comma-separated list of chain IDs to filter by
    pub async fn get_tokens(&self, chains: Option<&str>) -> SdkResult<serde_json::Value> {
        let mut url = format!("{}/tokens", self.base_url);
        if let Some(c) = chains {
            url = format!("{}?chains={}", url, c);
        }
        self.get_json(&url).await
    }

    /// Gets a quote for a cross-chain or same-chain swap
    ///
    /// Returns the best route with estimated output amount, fees, and
    /// execution steps.
    ///
    /// # Arguments
    ///
    /// * `from_token` - Source token address or symbol
    /// * `to_token` - Destination token address or symbol
    /// * `from_amount` - Amount to swap (in smallest unit)
    /// * `from_chain` - Source chain ID
    /// * `to_chain` - Destination chain ID
    /// * `from_address` - Sender address
    /// * `to_address` - Recipient address
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::lifi::LifiClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = LifiClient::new(None);
    /// let quote = client.get_quote(
    ///     "ETH", "USDC",
    ///     "1000000000000000000", // 1 ETH
    ///     "1",    // Ethereum
    ///     "42161", // Arbitrum
    ///     "0xsender...", "0xrecipient...",
    /// ).await?;
    /// println!("Output: {} {}", quote.estimate.to_amount, quote.action.to_token.symbol);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_quote(
        &self,
        from_token: &str,
        to_token: &str,
        from_amount: &str,
        from_chain: &str,
        to_chain: &str,
        from_address: &str,
        to_address: &str,
    ) -> SdkResult<LifiQuote> {
        let url = format!(
            "{}/quote?fromToken={}&toToken={}&fromAmount={}&fromChain={}&toChain={}&fromAddress={}&toAddress={}",
            self.base_url, from_token, to_token, from_amount, from_chain, to_chain, from_address, to_address
        );
        self.get_json(&url).await
    }

    /// Gets all available routes for a swap
    ///
    /// Returns multiple route options ranked by output amount, speed, and fees.
    ///
    /// # Arguments
    ///
    /// * `request` - Route request parameters
    pub async fn get_routes(&self, request: &LifiRouteRequest) -> SdkResult<LifiRoutesResponse> {
        let url = format!("{}/advanced/routes", self.base_url);
        self.post_json(&url, request).await
    }

    /// Gets the status of a cross-chain transaction
    ///
    /// # Arguments
    ///
    /// * `tx_hash` - Transaction hash on the source chain
    /// * `bridge` - Bridge used (e.g., "stargate", "hop")
    /// * `from_chain` - Source chain ID
    /// * `to_chain` - Destination chain ID
    pub async fn get_status(
        &self,
        tx_hash: &str,
        bridge: &str,
        from_chain: &str,
        to_chain: &str,
    ) -> SdkResult<LifiStatus> {
        let url = format!(
            "{}/status?txHash={}&bridge={}&fromChain={}&toChain={}",
            self.base_url, tx_hash, bridge, from_chain, to_chain
        );
        self.get_json(&url).await
    }

    /// Gets token information on a specific chain
    ///
    /// # Arguments
    ///
    /// * `chain` - Chain ID
    /// * `token` - Token address or symbol
    pub async fn get_token(
        &self,
        chain: &str,
        token: &str,
    ) -> SdkResult<LifiToken> {
        let url = format!("{}/token?chain={}&token={}", self.base_url, chain, token);
        self.get_json(&url).await
    }

    /// Gets all available connections (supported chain-to-chain routes)
    ///
    /// # Arguments
    ///
    /// * `from_chain` - Optional source chain ID filter
    /// * `to_chain` - Optional destination chain ID filter
    pub async fn get_connections(
        &self,
        from_chain: Option<&str>,
        to_chain: Option<&str>,
    ) -> SdkResult<LifiConnectionsResponse> {
        let mut url = format!("{}/connections", self.base_url);
        let mut params = Vec::new();
        if let Some(fc) = from_chain {
            params.push(format!("fromChain={}", fc));
        }
        if let Some(tc) = to_chain {
            params.push(format!("toChain={}", tc));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }
        self.get_json(&url).await
    }

    /// Gets all available bridge/DEX tools
    ///
    /// Returns the list of bridge and DEX aggregators available through LI.FI.
    pub async fn get_tools(&self) -> SdkResult<LifiToolsResponse> {
        let url = format!("{}/tools", self.base_url);
        self.get_json(&url).await
    }

    /// Builds a transaction for a specific route step
    ///
    /// Takes a step from a route response and returns the transaction data
    /// ready to be signed and submitted.
    ///
    /// # Arguments
    ///
    /// * `step` - A route step from `get_routes()` or `get_quote()`
    pub async fn get_step_transaction(
        &self,
        step: &serde_json::Value,
    ) -> SdkResult<LifiTransactionRequest> {
        let url = format!("{}/advanced/stepTransaction", self.base_url);
        self.post_json(&url, step).await
    }

    // -----------------------------------------------------------------------
    // Internal HTTP helpers
    // -----------------------------------------------------------------------

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> SdkResult<T> {
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| SdkError::ConnectionError(format!("LI.FI request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SdkError::RpcError(format!(
                "LI.FI API error (HTTP {}): {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| SdkError::RpcError(format!("Failed to parse LI.FI response: {}", e)))
    }

    async fn post_json<B: Serialize, T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: &B,
    ) -> SdkResult<T> {
        let response = self
            .http
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| SdkError::ConnectionError(format!("LI.FI request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(SdkError::RpcError(format!(
                "LI.FI API error (HTTP {}): {}",
                status, body_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| SdkError::RpcError(format!("Failed to parse LI.FI response: {}", e)))
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// LI.FI chains response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LifiChainsResponse {
    #[serde(default)]
    chains: Vec<LifiChain>,
}

/// A chain supported by LI.FI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifiChain {
    /// Chain ID
    #[serde(default)]
    pub id: u64,
    /// Chain name
    #[serde(default)]
    pub name: String,
    /// Chain key (e.g., "eth", "arb", "pol")
    #[serde(default)]
    pub key: String,
    /// Chain type (e.g., "EVM", "SVM")
    #[serde(default, rename = "chainType")]
    pub chain_type: String,
    /// Native token of the chain
    #[serde(default)]
    pub native_token: LifiToken,
}

/// A token in the LI.FI ecosystem
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifiToken {
    /// Token contract address (or native address)
    #[serde(default)]
    pub address: String,
    /// Token symbol
    #[serde(default)]
    pub symbol: String,
    /// Token name
    #[serde(default)]
    pub name: String,
    /// Number of decimals
    #[serde(default)]
    pub decimals: u8,
    /// Chain ID the token is on
    #[serde(default, rename = "chainId")]
    pub chain_id: u64,
    /// Logo URI
    #[serde(default, rename = "logoURI")]
    pub logo_uri: String,
    /// Price in USD
    #[serde(default, rename = "priceUSD")]
    pub price_usd: String,
}

/// A swap/bridge quote from LI.FI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifiQuote {
    /// Quote ID
    #[serde(default)]
    pub id: String,
    /// The action being performed
    #[serde(default)]
    pub action: LifiAction,
    /// Estimate details
    #[serde(default)]
    pub estimate: LifiEstimate,
    /// Transaction request (ready to sign)
    #[serde(default, rename = "transactionRequest")]
    pub transaction_request: Option<LifiTransactionRequest>,
}

/// Action details in a LI.FI quote
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifiAction {
    /// Source chain ID
    #[serde(default, rename = "fromChainId")]
    pub from_chain_id: u64,
    /// Destination chain ID
    #[serde(default, rename = "toChainId")]
    pub to_chain_id: u64,
    /// Source token
    #[serde(default, rename = "fromToken")]
    pub from_token: LifiToken,
    /// Destination token
    #[serde(default, rename = "toToken")]
    pub to_token: LifiToken,
    /// Amount being sent (smallest unit)
    #[serde(default, rename = "fromAmount")]
    pub from_amount: String,
}

/// Estimate details in a LI.FI quote
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifiEstimate {
    /// Estimated output amount (smallest unit)
    #[serde(default, rename = "toAmount")]
    pub to_amount: String,
    /// Minimum output amount (smallest unit)
    #[serde(default, rename = "toAmountMin")]
    pub to_amount_min: String,
    /// Estimated execution time in seconds
    #[serde(default, rename = "executionDuration")]
    pub execution_duration: u64,
    /// Estimated gas costs
    #[serde(default, rename = "gasCosts")]
    pub gas_costs: Vec<LifiGasCost>,
}

/// Gas cost breakdown
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifiGasCost {
    /// Gas cost type
    #[serde(default, rename = "type")]
    pub cost_type: String,
    /// Estimated gas amount
    #[serde(default)]
    pub estimate: String,
    /// Gas token
    #[serde(default)]
    pub token: LifiToken,
    /// Amount in USD
    #[serde(default, rename = "amountUSD")]
    pub amount_usd: String,
}

/// Transaction data ready to be signed
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifiTransactionRequest {
    /// Target contract address
    #[serde(default)]
    pub to: String,
    /// Transaction data (hex)
    #[serde(default)]
    pub data: String,
    /// Value to send (hex)
    #[serde(default)]
    pub value: String,
    /// Gas limit (hex)
    #[serde(default, rename = "gasLimit")]
    pub gas_limit: String,
    /// Gas price (hex)
    #[serde(default, rename = "gasPrice")]
    pub gas_price: String,
    /// Chain ID
    #[serde(default, rename = "chainId")]
    pub chain_id: u64,
    /// Sender address
    #[serde(default)]
    pub from: String,
}

/// Route request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifiRouteRequest {
    /// Source chain ID
    #[serde(rename = "fromChainId")]
    pub from_chain_id: u64,
    /// Destination chain ID
    #[serde(rename = "toChainId")]
    pub to_chain_id: u64,
    /// Source token address
    #[serde(rename = "fromTokenAddress")]
    pub from_token_address: String,
    /// Destination token address
    #[serde(rename = "toTokenAddress")]
    pub to_token_address: String,
    /// Amount to swap (smallest unit)
    #[serde(rename = "fromAmount")]
    pub from_amount: String,
    /// Sender address
    #[serde(rename = "fromAddress")]
    pub from_address: String,
    /// Recipient address (optional, defaults to fromAddress)
    #[serde(rename = "toAddress", skip_serializing_if = "Option::is_none")]
    pub to_address: Option<String>,
}

/// Routes response from LI.FI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifiRoutesResponse {
    /// Available routes sorted by best output
    #[serde(default)]
    pub routes: Vec<LifiRoute>,
}

/// A single route option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifiRoute {
    /// Route ID
    #[serde(default)]
    pub id: String,
    /// Source token amount (smallest unit)
    #[serde(default, rename = "fromAmount")]
    pub from_amount: String,
    /// Estimated output amount (smallest unit)
    #[serde(default, rename = "toAmount")]
    pub to_amount: String,
    /// Minimum output amount (smallest unit)
    #[serde(default, rename = "toAmountMin")]
    pub to_amount_min: String,
    /// Route steps (bridge/swap operations)
    #[serde(default)]
    pub steps: Vec<serde_json::Value>,
    /// Tags (e.g., "CHEAPEST", "FASTEST")
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Cross-chain transaction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifiStatus {
    /// Transaction status (e.g., "DONE", "PENDING", "FAILED", "NOT_FOUND")
    #[serde(default)]
    pub status: String,
    /// Sub-status for more detail
    #[serde(default, rename = "substatus")]
    pub sub_status: String,
    /// Sending chain transaction info
    #[serde(default)]
    pub sending: Option<LifiTxInfo>,
    /// Receiving chain transaction info
    #[serde(default)]
    pub receiving: Option<LifiTxInfo>,
}

/// Transaction info within a status response
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifiTxInfo {
    /// Transaction hash
    #[serde(default, rename = "txHash")]
    pub tx_hash: String,
    /// Chain ID
    #[serde(default, rename = "chainId")]
    pub chain_id: u64,
    /// Amount (smallest unit)
    #[serde(default)]
    pub amount: String,
    /// Token info
    #[serde(default)]
    pub token: Option<LifiToken>,
}

/// Connections response (supported routes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifiConnectionsResponse {
    /// Available connections
    #[serde(default)]
    pub connections: Vec<serde_json::Value>,
}

/// Available bridge/DEX tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifiToolsResponse {
    /// Bridge tools
    #[serde(default)]
    pub bridges: Vec<LifiTool>,
    /// DEX tools
    #[serde(default)]
    pub exchanges: Vec<LifiTool>,
}

/// A bridge or DEX tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifiTool {
    /// Tool key (e.g., "stargate", "1inch")
    #[serde(default)]
    pub key: String,
    /// Tool name
    #[serde(default)]
    pub name: String,
    /// Logo URI
    #[serde(default, rename = "logoURI")]
    pub logo_uri: String,
    /// Supported chains
    #[serde(default, rename = "supportedChains")]
    pub supported_chains: Vec<serde_json::Value>,
}
