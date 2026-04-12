//! Identity SDK for Tenzro Network
//!
//! This module provides identity management functionality for
//! TDIP (Tenzro Decentralized Identity Protocol) identities.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Identity client for TDIP identity operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let identity_client = client.identity();
///
/// // Register a new human identity
/// let result = identity_client.register_human("Alice").await?;
/// println!("DID: {}", result.did);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct IdentityClient {
    rpc: Arc<RpcClient>,
}

impl IdentityClient {
    /// Creates a new identity client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Registers a new human identity
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let identity_client = client.identity();
    /// let result = identity_client.register_human("Alice").await?;
    /// println!("Registered: {}", result.did);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register_human(&self, display_name: &str) -> SdkResult<RegisterIdentityResponse> {
        self.rpc
            .call(
                "tenzro_registerIdentity",
                serde_json::json!([{ "display_name": display_name }]),
            )
            .await
    }

    /// Registers a new human identity with a specific public key
    pub async fn register_human_with_key(
        &self,
        display_name: &str,
        public_key: &str,
        key_type: &str,
    ) -> SdkResult<RegisterIdentityResponse> {
        self.rpc
            .call(
                "tenzro_registerIdentity",
                serde_json::json!([{
                    "display_name": display_name,
                    "public_key": public_key,
                    "key_type": key_type,
                }]),
            )
            .await
    }

    /// Resolves a DID to its identity information
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let identity_client = client.identity();
    /// let info = identity_client.resolve("did:tenzro:human:abc123").await?;
    /// println!("Status: {}", info.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn resolve(&self, did: &str) -> SdkResult<IdentityInfo> {
        self.rpc
            .call(
                "tenzro_resolveIdentity",
                serde_json::json!([{ "did": did }]),
            )
            .await
    }

    /// Resolves a DID to its W3C DID Document
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let identity_client = client.identity();
    /// let doc = identity_client.resolve_did_document("did:tenzro:human:abc123").await?;
    /// println!("DID Document: {}", serde_json::to_string_pretty(&doc)?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn resolve_did_document(
        &self,
        did: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_resolveDidDocument",
                serde_json::json!([{ "did": did }]),
            )
            .await
    }

    /// Registers a new machine identity under a controller
    ///
    /// Creates a `did:tenzro:machine:{controller}:{uuid}` identity, or an
    /// autonomous machine identity `did:tenzro:machine:{uuid}` if no
    /// controller DID is provided.
    ///
    /// # Arguments
    ///
    /// * `params` - Machine identity registration parameters
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # use tenzro_sdk::identity::RegisterMachineParams;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let identity = client.identity();
    /// let result = identity.register_machine(RegisterMachineParams {
    ///     controller_did: Some("did:tenzro:human:abc123".to_string()),
    ///     capabilities: vec!["inference".to_string(), "analysis".to_string()],
    ///     delegation_scope: None,
    /// }).await?;
    /// println!("Machine DID: {}", result.did);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register_machine(
        &self,
        params: RegisterMachineParams,
    ) -> SdkResult<RegisterIdentityResponse> {
        self.rpc
            .call(
                "tenzro_registerMachineIdentity",
                serde_json::json!([{
                    "controller_did": params.controller_did,
                    "capabilities": params.capabilities,
                    "delegation_scope": params.delegation_scope,
                }]),
            )
            .await
    }

    /// Lists all registered identities
    pub async fn list_identities(&self) -> SdkResult<serde_json::Value> {
        self.rpc
            .call("tenzro_listIdentities", serde_json::json!([]))
            .await
    }

    /// Adds a verifiable credential to an identity
    pub async fn add_credential(&self, did: &str, credential_type: &str, issuer: Option<&str>, claims: Option<serde_json::Value>) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_addCredential",
                serde_json::json!({"did": did, "type": credential_type, "issuer": issuer, "claims": claims}),
            )
            .await
    }

    /// Adds a service endpoint to an identity
    pub async fn add_service(&self, did: &str, service_type: &str, endpoint: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_addService",
                serde_json::json!({"did": did, "type": service_type, "endpoint": endpoint}),
            )
            .await
    }

    /// Sets a human-readable username for a DID
    ///
    /// Usernames provide a convenient alias for DIDs, making them easier to
    /// share and remember. Each DID can have at most one username.
    ///
    /// # Arguments
    ///
    /// * `did` - The DID to set the username for
    /// * `username` - The desired username
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let identity = client.identity();
    /// identity.set_username("did:tenzro:human:abc123", "alice").await?;
    /// println!("Username set successfully");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_username(&self, did: &str, username: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_setUsername",
                serde_json::json!([{ "did": did, "username": username }]),
            )
            .await
    }

    /// Resolves a username to its associated DID
    ///
    /// # Arguments
    ///
    /// * `username` - The username to resolve
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let identity = client.identity();
    /// let resolution = identity.resolve_username("alice").await?;
    /// println!("DID: {}", resolution.did);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn resolve_username(&self, username: &str) -> SdkResult<UsernameResolution> {
        self.rpc
            .call(
                "tenzro_resolveUsername",
                serde_json::json!([{ "username": username }]),
            )
            .await
    }

    /// Resolves a DID to its identity information via the MCP server
    ///
    /// This is an alias for `resolve()` using the `tenzro_resolveDid` RPC method,
    /// matching the MCP server tool name.
    ///
    /// # Arguments
    ///
    /// * `did` - The DID to resolve
    pub async fn resolve_did(&self, did: &str) -> SdkResult<IdentityInfo> {
        self.rpc
            .call(
                "tenzro_resolveDid",
                serde_json::json!([{ "did": did }]),
            )
            .await
    }

    /// Sets the delegation scope for a machine identity
    ///
    /// Configures spending limits, allowed operations, protocols, and chains
    /// for a machine DID.
    ///
    /// # Arguments
    ///
    /// * `did` - The machine DID to configure
    /// * `scope` - Delegation scope configuration (JSON object)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let identity = client.identity();
    /// let scope = serde_json::json!({
    ///     "max_transaction_value": "1000000000000000000",
    ///     "max_daily_spend": "10000000000000000000",
    ///     "allowed_operations": ["transfer", "inference"],
    /// });
    /// identity.set_delegation_scope("did:tenzro:machine:abc123", scope).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_delegation_scope(
        &self,
        did: &str,
        scope: serde_json::Value,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_setDelegationScope",
                serde_json::json!([{
                    "did": did,
                    "scope": scope,
                }]),
            )
            .await
    }

    /// Imports an existing identity (e.g., from a backup or another node)
    ///
    /// # Arguments
    ///
    /// * `params` - Identity import parameters
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # use tenzro_sdk::identity::ImportIdentityParams;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let identity = client.identity();
    /// let result = identity.import_identity(ImportIdentityParams {
    ///     did: "did:tenzro:human:abc123".to_string(),
    ///     private_key: "0xdeadbeef...".to_string(),
    ///     key_type: "ed25519".to_string(),
    ///     password: "my-secure-password".to_string(),
    /// }).await?;
    /// println!("Imported: {} ({})", result.did, result.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn import_identity(
        &self,
        params: ImportIdentityParams,
    ) -> SdkResult<RegisterIdentityResponse> {
        self.rpc
            .call(
                "tenzro_importIdentity",
                serde_json::json!([{
                    "did": params.did,
                    "private_key": params.private_key,
                    "key_type": params.key_type,
                    "password": params.password,
                }]),
            )
            .await
    }
}

/// Identity type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentityType {
    /// Human identity
    Human,
    /// Machine identity
    Machine,
}

/// Response from identity registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterIdentityResponse {
    /// The assigned DID
    pub did: String,
    /// Registration status
    #[serde(default)]
    pub status: String,
    /// Private key (only returned when auto-generated)
    pub private_key: Option<String>,
}

/// Parameters for registering a machine identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMachineParams {
    /// Controller DID (omit for autonomous machine identity)
    pub controller_did: Option<String>,
    /// Capabilities for the machine (e.g., "inference", "analysis")
    pub capabilities: Vec<String>,
    /// Optional delegation scope constraints
    pub delegation_scope: Option<serde_json::Value>,
}

/// Parameters for importing an identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportIdentityParams {
    /// The DID to import
    pub did: String,
    /// Private key (hex-encoded)
    pub private_key: String,
    /// Key type ("ed25519" or "secp256k1")
    pub key_type: String,
    /// Password for encrypting the keystore
    pub password: String,
}

/// Response from resolving a username
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsernameResolution {
    /// The resolved username
    #[serde(default)]
    pub username: String,
    /// The DID associated with this username
    #[serde(default)]
    pub did: String,
}

/// Identity information returned by resolve
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityInfo {
    /// The DID string
    pub did: String,
    /// Current status (active, suspended, revoked)
    #[serde(default)]
    pub status: String,
    /// Whether this is a human identity
    #[serde(default)]
    pub is_human: bool,
    /// Whether this is a machine identity
    #[serde(default)]
    pub is_machine: bool,
    /// Display name
    #[serde(default)]
    pub display_name: String,
    /// Identity type (derived from is_human/is_machine)
    #[serde(skip)]
    pub identity_type: Option<IdentityType>,
    /// Number of public keys
    #[serde(default)]
    pub key_count: usize,
    /// Number of verifiable credentials
    #[serde(default)]
    pub credential_count: usize,
    /// Number of service endpoints
    #[serde(default)]
    pub service_count: usize,
}
