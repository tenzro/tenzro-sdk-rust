//! Contract SDK for Tenzro Network
//!
//! This module provides smart contract deployment functionality across
//! the multi-VM runtime (EVM, SVM, DAML).

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Contract client for deploying smart contracts
///
/// Supports deployment to EVM, SVM, and DAML VMs via the MultiVmRuntime.
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let contract = client.contract();
///
/// // Deploy an EVM contract
/// let result = contract.deploy(
///     "evm",
///     "0x608060405234801561001057600080fd5b50...",
///     "0x1234567890abcdef1234567890abcdef12345678",
///     None,
///     None,
/// ).await?;
/// println!("Deployed at: {}", result.address);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct ContractClient {
    rpc: Arc<RpcClient>,
}

impl ContractClient {
    /// Creates a new contract client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Deploys a smart contract to the specified VM
    ///
    /// # Arguments
    ///
    /// * `vm_type` - Target VM: "evm", "svm", or "daml"
    /// * `bytecode` - Contract bytecode (hex, with or without 0x prefix)
    /// * `deployer` - Deployer address (hex, with or without 0x prefix)
    /// * `constructor_args` - Optional constructor arguments (hex-encoded ABI)
    /// * `gas_limit` - Optional gas limit (default 3,000,000)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let contract = client.contract();
    ///
    /// // Deploy with constructor args and custom gas limit
    /// let result = contract.deploy(
    ///     "evm",
    ///     "0x608060405234801561001057600080fd5b50...",
    ///     "0x1234567890abcdef1234567890abcdef12345678",
    ///     Some("0x000000000000000000000000000000000000000000000000000000000000002a"),
    ///     Some(5_000_000),
    /// ).await?;
    /// println!("Contract address: {}", result.address);
    /// println!("Gas used: {}", result.gas_used);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn deploy(
        &self,
        vm_type: &str,
        bytecode: &str,
        deployer: &str,
        constructor_args: Option<&str>,
        gas_limit: Option<u64>,
    ) -> SdkResult<DeployResult> {
        let mut params = serde_json::json!({
            "vm_type": vm_type,
            "bytecode": bytecode,
            "deployer": deployer,
        });

        if let Some(args) = constructor_args {
            params["constructor_args"] = serde_json::json!(args);
        }
        if let Some(gl) = gas_limit {
            params["gas_limit"] = serde_json::json!(gl);
        }

        self.rpc
            .call("tenzro_deployContract", serde_json::json!([params]))
            .await
    }

    /// Performs a read-only contract call (eth_call)
    ///
    /// Executes a contract call without creating a transaction on-chain.
    /// Useful for reading state from smart contracts.
    ///
    /// # Arguments
    ///
    /// * `to` - Contract address (hex)
    /// * `data` - ABI-encoded call data (hex)
    /// * `block` - Optional block number or "latest" (default: "latest")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let contract = client.contract();
    /// let result = contract.call_contract(
    ///     "0x1234...",
    ///     "0x70a08231000000000000000000000000abcdef...",
    ///     None,
    /// ).await?;
    /// println!("Return data: {}", result.data);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn call_contract(
        &self,
        to: &str,
        data: &str,
        block: Option<&str>,
    ) -> SdkResult<CallResult> {
        let block_param = block.unwrap_or("latest");
        let result: String = self
            .rpc
            .call(
                "eth_call",
                serde_json::json!([{ "to": to, "data": data }, block_param]),
            )
            .await?;
        Ok(CallResult { data: result })
    }

    /// ABI-encodes a function call
    ///
    /// Encodes a function signature and arguments into call data suitable
    /// for use with `call_contract` or `send_transaction`.
    ///
    /// # Arguments
    ///
    /// * `function_sig` - Function signature (e.g., "transfer(address,uint256)")
    /// * `args` - Function arguments as JSON values
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let contract = client.contract();
    /// let encoded = contract.encode_function(
    ///     "transfer(address,uint256)",
    ///     vec![
    ///         serde_json::json!("0xrecipient..."),
    ///         serde_json::json!("1000000000000000000"),
    ///     ],
    /// ).await?;
    /// println!("Encoded: {}", encoded);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn encode_function(
        &self,
        function_sig: &str,
        args: Vec<serde_json::Value>,
    ) -> SdkResult<String> {
        self.rpc
            .call(
                "tenzro_encodeFunction",
                serde_json::json!([{
                    "function_sig": function_sig,
                    "args": args,
                }]),
            )
            .await
    }

    /// Decodes ABI-encoded return data
    ///
    /// # Arguments
    ///
    /// * `data` - Hex-encoded return data from a contract call
    /// * `output_types` - Expected output types (e.g., ["uint256", "address"])
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let contract = client.contract();
    /// let decoded = contract.decode_result(
    ///     "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000",
    ///     vec!["uint256"],
    /// ).await?;
    /// println!("Decoded: {:?}", decoded);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn decode_result(
        &self,
        data: &str,
        output_types: Vec<&str>,
    ) -> SdkResult<Vec<serde_json::Value>> {
        self.rpc
            .call(
                "tenzro_decodeResult",
                serde_json::json!([{
                    "data": data,
                    "output_types": output_types,
                }]),
            )
            .await
    }
}

/// Result from a read-only contract call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallResult {
    /// Hex-encoded return data
    pub data: String,
}

/// Result from a contract deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployResult {
    /// Deployed contract address (hex)
    #[serde(default)]
    pub address: String,
    /// Gas consumed by the deployment
    #[serde(default)]
    pub gas_used: u64,
    /// VM type the contract was deployed to
    #[serde(default)]
    pub vm_type: String,
    /// Operation status (e.g., "deployed")
    #[serde(default)]
    pub status: String,
}
