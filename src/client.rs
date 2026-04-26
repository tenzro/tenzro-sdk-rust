//! Main SDK client for Tenzro Network
//!
//! This module provides the primary entry point for interacting with Tenzro Network.

use crate::config::SdkConfig;
use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use crate::types::Address;
use crate::{
    agent::AgentClient, agent_payments::AgentPaymentClient, ap2::Ap2Client,
    auth::AuthClient,
    bridge::BridgeClient, canton::CantonClient, circuit_breaker::CircuitBreakerClient,
    compliance::ComplianceClient, contract::ContractClient, crypto::CryptoClient,
    custody::CustodyClient, debridge::DebridgeClient, erc7802::Erc7802Client,
    events::EventClient, governance::GovernanceClient, identity::IdentityClient,
    inference::InferenceClient, marketplace::MarketplaceClient, nanopayment::NanopaymentClient,
    nft::NftClient, payment::PaymentClient, provider::ProviderClient,
    settlement::SettlementClient, skill::SkillClient, staking::StakingClient,
    streaming::StreamingClient, task::TaskClient, tee::TeeClient, token::TokenClient,
    tool::ToolClient, wallet::WalletClient, zk::ZkClient,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Main Tenzro Network client
///
/// The primary entry point for developers to interact with Tenzro Network.
///
/// # Example
///
/// ```no_run
/// use tenzro_sdk::TenzroClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = TenzroClient::new("https://rpc.tenzro.network").await?;
///
///     let block_number = client.block_number().await?;
///     println!("Current block: {}", block_number);
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct TenzroClient {
    config: Arc<SdkConfig>,
    pub(crate) rpc: Arc<RpcClient>,
}

impl TenzroClient {
    /// Creates a new client connected to the given RPC endpoint.
    ///
    /// This is the simplest way to get started. Uses default timeout and settings.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tenzro_sdk::TenzroClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = TenzroClient::new("https://rpc.tenzro.network").await?;
    ///     println!("Connected!");
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(endpoint: &str) -> SdkResult<Self> {
        let config = SdkConfig::builder()
            .endpoint(endpoint)
            .build()?;
        Self::connect(config).await
    }

    /// Connects to a Tenzro Network node with full configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tenzro_sdk::{TenzroClient, config::SdkConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = SdkConfig::testnet();
    ///     let client = TenzroClient::connect(config).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn connect(config: SdkConfig) -> SdkResult<Self> {
        tracing::info!("Connecting to Tenzro Network at {}", config.endpoint);

        let timeout = Duration::from_millis(config.timeout_ms);
        let rpc = RpcClient::new(&config.endpoint, timeout)?;

        let client = Self {
            config: Arc::new(config),
            rpc: Arc::new(rpc),
        };

        tracing::info!("Successfully connected to Tenzro Network");
        Ok(client)
    }

    /// Checks if the node is reachable by calling `eth_chainId`
    pub async fn is_connected(&self) -> bool {
        self.get_chain_id().await.is_ok()
    }

    /// Gets the current block number (height)
    pub async fn block_number(&self) -> SdkResult<u64> {
        let hex: String = self
            .rpc
            .call("tenzro_blockNumber", serde_json::json!([]))
            .await?;
        parse_hex_u64(&hex)
    }

    /// Gets a block by its height
    pub async fn get_block(&self, height: u64) -> SdkResult<BlockInfo> {
        let block: BlockInfo = self
            .rpc
            .call(
                "tenzro_getBlock",
                serde_json::json!([{ "block_number": height }]),
            )
            .await?;
        Ok(block)
    }

    /// Gets the latest block
    pub async fn get_latest_block(&self) -> SdkResult<BlockInfo> {
        let block: BlockInfo = self
            .rpc
            .call("tenzro_getBlock", serde_json::json!([{ "height": "latest" }]))
            .await?;
        Ok(block)
    }

    /// Gets the balance of an address (in wei)
    pub async fn get_balance(&self, address: Address) -> SdkResult<u128> {
        let hex: String = self
            .rpc
            .call(
                "tenzro_getBalance",
                serde_json::json!([format!("0x{}", hex::encode(address.as_bytes()))]),
            )
            .await?;
        parse_hex_u128(&hex)
    }

    /// Gets the nonce of an address
    pub async fn get_nonce(&self, address: Address) -> SdkResult<u64> {
        let hex: String = self
            .rpc
            .call(
                "tenzro_getNonce",
                serde_json::json!([format!("0x{}", hex::encode(address.as_bytes()))]),
            )
            .await?;
        parse_hex_u64(&hex)
    }

    /// Gets the chain ID
    pub async fn get_chain_id(&self) -> SdkResult<u64> {
        let hex: String = self
            .rpc
            .call("eth_chainId", serde_json::json!([]))
            .await?;
        parse_hex_u64(&hex)
    }

    /// Submits a raw transaction
    pub async fn send_transaction(
        &self,
        from: Address,
        to: Address,
        value: u64,
        gas_limit: Option<u64>,
        gas_price: Option<u64>,
    ) -> SdkResult<String> {
        let mut tx = serde_json::json!({
            "from": format!("0x{}", hex::encode(from.as_bytes())),
            "to": format!("0x{}", hex::encode(to.as_bytes())),
            "value": format!("0x{:x}", value),
        });

        if let Some(gl) = gas_limit {
            tx["gas_limit"] = serde_json::json!(format!("0x{:x}", gl));
        }
        if let Some(gp) = gas_price {
            tx["gas_price"] = serde_json::json!(format!("0x{:x}", gp));
        }

        let tx_hash: String = self
            .rpc
            .call("eth_sendRawTransaction", serde_json::json!([tx]))
            .await?;
        Ok(tx_hash)
    }

    /// Gets information about the connected node
    pub async fn node_info(&self) -> SdkResult<NodeInfo> {
        let rpc_info: serde_json::Value = self
            .rpc
            .call("tenzro_nodeInfo", serde_json::json!([]))
            .await?;

        Ok(NodeInfo {
            version: rpc_info["version"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            chain_id: self.config.chain_id,
            block_height: rpc_info["block_height"].as_u64().unwrap_or(0),
            peer_count: rpc_info["peer_count"].as_u64().unwrap_or(0) as u32,
            syncing: rpc_info["state"]
                .as_str()
                .map(|s| s == "syncing")
                .unwrap_or(false),
        })
    }

    /// Gets the node status via the Web API
    pub async fn get_status(&self) -> SdkResult<NodeStatus> {
        self.rpc.get("/status").await
    }

    /// Requests testnet TNZO tokens from the faucet
    pub async fn request_faucet(&self, address: Address) -> SdkResult<FaucetResponse> {
        let body = serde_json::json!({
            "address": format!("0x{}", hex::encode(address.as_bytes())),
        });
        self.rpc.post("/faucet", &body).await
    }

    /// Gets the TNZO total supply
    pub async fn total_supply(&self) -> SdkResult<String> {
        self.rpc
            .call("tenzro_totalSupply", serde_json::json!([]))
            .await
    }

    /// Gets the peer count
    pub async fn peer_count(&self) -> SdkResult<u64> {
        let hex: String = self
            .rpc
            .call("tenzro_peerCount", serde_json::json!([]))
            .await?;
        parse_hex_u64(&hex)
    }

    /// Creates a wallet client for wallet operations
    pub fn wallet(&self) -> WalletClient {
        WalletClient::new(self.rpc.clone())
    }

    /// Creates an inference client for AI model inference
    pub fn inference(&self) -> InferenceClient {
        InferenceClient::new(self.rpc.clone())
    }

    /// Creates a settlement client for payment settlement
    pub fn settlement(&self) -> SettlementClient {
        SettlementClient::new(self.rpc.clone())
    }

    /// Creates an agent client for AI agent operations
    pub fn agent(&self) -> AgentClient {
        AgentClient::new(self.rpc.clone())
    }

    /// Creates an identity client for TDIP identity operations
    pub fn identity(&self) -> IdentityClient {
        IdentityClient::new(self.rpc.clone())
    }

    /// Creates a payment client for MPP/x402 payment operations
    pub fn payment(&self) -> PaymentClient {
        PaymentClient::new(self.rpc.clone())
    }

    /// Creates a governance client for governance operations
    pub fn governance(&self) -> GovernanceClient {
        GovernanceClient::new(self.rpc.clone())
    }

    /// Creates a provider client for network participation and model serving
    pub fn provider(&self) -> ProviderClient {
        ProviderClient::new(self.rpc.clone())
    }

    /// Creates a task client for task marketplace operations
    pub fn task(&self) -> TaskClient {
        TaskClient::new(self.rpc.clone())
    }

    /// Creates a marketplace client for agent template marketplace operations
    pub fn marketplace(&self) -> MarketplaceClient {
        MarketplaceClient::new(self.rpc.clone())
    }

    /// Creates a Canton client for Canton/DAML operations
    pub fn canton(&self) -> CantonClient {
        CantonClient::new(self.rpc.clone())
    }

    /// Creates a staking client for TNZO staking operations
    pub fn staking(&self) -> StakingClient {
        StakingClient::new(self.rpc.clone())
    }

    /// Creates a token client for token registry and cross-VM operations
    pub fn token(&self) -> TokenClient {
        TokenClient::new(self.rpc.clone())
    }

    /// Creates a contract client for smart contract deployment
    pub fn contract(&self) -> ContractClient {
        ContractClient::new(self.rpc.clone())
    }

    /// Creates a skill client for Skills Registry operations
    pub fn skill(&self) -> SkillClient {
        SkillClient::new(self.rpc.clone())
    }

    /// Creates a tool client for Tool Registry operations
    pub fn tool(&self) -> ToolClient {
        ToolClient::new(self.rpc.clone())
    }

    /// Creates an AP2 client for agentic payment protocol operations
    pub fn ap2(&self) -> Ap2Client {
        Ap2Client::new(self.rpc.clone())
    }

    /// Creates a bridge client for cross-chain token transfer operations
    pub fn bridge(&self) -> BridgeClient {
        BridgeClient::new(self.rpc.clone())
    }

    /// Creates an agent payment client for spending policies and transactions
    pub fn agent_payments(&self) -> AgentPaymentClient {
        AgentPaymentClient::new(self.rpc.clone())
    }

    /// Creates a circuit breaker client for provider health management
    pub fn circuit_breaker(&self) -> CircuitBreakerClient {
        CircuitBreakerClient::new(self.rpc.clone())
    }

    /// Creates a nanopayment client for micropayment channel operations
    pub fn nanopayment(&self) -> NanopaymentClient {
        NanopaymentClient::new(self.rpc.clone())
    }

    /// Creates an ERC-7802 client for cross-chain token mint/burn operations
    pub fn erc7802(&self) -> Erc7802Client {
        Erc7802Client::new(self.rpc.clone())
    }

    /// Creates an ERC-8004 client for the trustless agents registry
    /// (identity, reputation, validation) on any EVM chain
    pub fn erc8004(&self) -> crate::erc8004::Erc8004Client {
        crate::erc8004::Erc8004Client::new(self.rpc.clone())
    }

    /// Creates a Wormhole client for cross-chain token transfers and
    /// VAA helpers via the node's Wormhole BridgeRouter adapter
    pub fn wormhole(&self) -> crate::wormhole::WormholeClient {
        crate::wormhole::WormholeClient::new(self.rpc.clone())
    }

    /// Creates a CCT (Chainlink Cross-Chain Token) client for inspecting
    /// the canonical TNZO CCT pool registry
    pub fn cct(&self) -> crate::cct::CctClient {
        crate::cct::CctClient::new(self.rpc.clone())
    }

    /// Creates an NFT client for collection and token management
    pub fn nft(&self) -> NftClient {
        NftClient::new(self.rpc.clone())
    }

    /// Creates a compliance client for ERC-3643 regulated token operations
    pub fn compliance(&self) -> ComplianceClient {
        ComplianceClient::new(self.rpc.clone())
    }

    /// Creates an event client for querying and subscribing to on-chain events
    pub fn events(&self) -> EventClient {
        EventClient::new(self.rpc.clone())
    }

    /// Creates a deBridge DLN client for cross-chain swaps
    pub fn debridge(&self) -> DebridgeClient {
        DebridgeClient::new(self.rpc.clone())
    }

    /// Creates a crypto client for cryptographic operations
    pub fn crypto(&self) -> CryptoClient {
        CryptoClient::new(self.rpc.clone())
    }

    /// Creates a TEE client for Trusted Execution Environment operations
    pub fn tee(&self) -> TeeClient {
        TeeClient::new(self.rpc.clone())
    }

    /// Creates a ZK client for zero-knowledge proof operations
    pub fn zk(&self) -> ZkClient {
        ZkClient::new(self.rpc.clone())
    }

    /// Creates a custody client for key custody and wallet security operations
    pub fn custody(&self) -> CustodyClient {
        CustodyClient::new(self.rpc.clone())
    }

    /// Creates a streaming client for real-time inference and event streaming
    pub fn streaming(&self) -> StreamingClient {
        StreamingClient::new(self.rpc.clone())
    }

    /// Creates an auth client for OAuth 2.1 + DPoP onboarding,
    /// JWT/DID revocation, and HITL approval flows.
    pub fn auth(&self) -> AuthClient {
        AuthClient::new(self.rpc.clone())
    }

    /// Returns the SDK configuration
    pub fn config(&self) -> &SdkConfig {
        &self.config
    }

    /// Returns the RPC endpoint URL
    pub fn endpoint(&self) -> &str {
        self.rpc.endpoint()
    }

    /// Gets a transaction by hash
    pub async fn get_transaction(&self, hash: &str) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_getTransaction", serde_json::json!([hash])).await
    }

    /// Checks if the node is syncing
    pub async fn syncing(&self) -> SdkResult<bool> {
        let result: serde_json::Value = self.rpc.call("tenzro_syncing", serde_json::json!([])).await?;
        Ok(result.get("syncing").and_then(|v| v.as_bool()).unwrap_or(false))
    }

    /// Gets the finalized block height
    pub async fn get_finalized_block(&self) -> SdkResult<u64> {
        let hex: String = self.rpc.call("tenzro_getFinalizedBlock", serde_json::json!([])).await?;
        parse_hex_u64(&hex)
    }

    /// Exports the node configuration
    pub async fn export_config(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_exportConfig", serde_json::json!([])).await
    }

    /// Gets transaction history for an address
    pub async fn get_transaction_history(&self, address: &str, limit: Option<u32>) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_getTransactionHistory", serde_json::json!([{"address": address, "limit": limit}])).await
    }

    /// Lists all accounts
    pub async fn list_accounts(&self) -> SdkResult<serde_json::Value> {
        self.rpc.call("tenzro_listAccounts", serde_json::json!([])).await
    }
}

/// Information about a Tenzro Network node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node software version
    pub version: String,
    /// Chain ID the node is connected to
    pub chain_id: u64,
    /// Current block height
    pub block_height: u64,
    /// Number of connected peers
    pub peer_count: u32,
    /// Whether the node is currently syncing
    pub syncing: bool,
}

/// Block information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block height
    pub height: u64,
    /// Block hash
    pub hash: String,
    /// Previous block hash
    #[serde(default)]
    pub prev_hash: String,
    /// State root
    #[serde(default)]
    pub state_root: String,
    /// Transaction root
    #[serde(default)]
    pub tx_root: String,
    /// Block timestamp
    #[serde(default)]
    pub timestamp: String,
    /// Block proposer
    #[serde(default)]
    pub proposer: String,
    /// Transaction count
    #[serde(default)]
    pub tx_count: u64,
    /// Gas used
    #[serde(default)]
    pub gas_used: u64,
    /// Gas limit
    #[serde(default)]
    pub gas_limit: u64,
}

/// Node status from Web API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    /// Node state (running, syncing, etc.)
    #[serde(default)]
    pub node_state: String,
    /// Node role (validator, user, etc.)
    #[serde(default)]
    pub role: String,
    /// Health status
    #[serde(default)]
    pub health: String,
    /// Current block height
    #[serde(default)]
    pub block_height: u64,
    /// Number of connected peers
    #[serde(default)]
    pub peer_count: u64,
    /// Uptime in seconds
    #[serde(default)]
    pub uptime_secs: u64,
}

/// Faucet response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaucetResponse {
    /// Whether the request succeeded
    pub success: bool,
    /// Transaction hash (if successful)
    pub tx_hash: Option<String>,
    /// Amount sent (e.g., "100 TNZO")
    #[serde(default)]
    pub amount: String,
    /// Status message
    #[serde(default)]
    pub message: String,
}

/// Parse a hex string (with or without "0x" prefix) into u64
fn parse_hex_u128(hex_str: &str) -> SdkResult<u128> {
    let stripped = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    u128::from_str_radix(stripped, 16)
        .map_err(|e| SdkError::RpcError(format!("Failed to parse hex '{}': {}", hex_str, e)))
}

fn parse_hex_u64(hex_str: &str) -> SdkResult<u64> {
    let stripped = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    u64::from_str_radix(stripped, 16)
        .map_err(|e| SdkError::RpcError(format!("Failed to parse hex '{}': {}", hex_str, e)))
}
