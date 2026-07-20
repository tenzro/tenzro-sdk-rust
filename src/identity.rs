//! Identity SDK for Tenzro Network
//!
//! This module provides identity management functionality for
//! TDIP (Tenzro Decentralized Identity Protocol) identities.

use crate::app::EnvelopeSigner;
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

    /// Adds a verifiable credential to an identity with a pre-built
    /// DID-envelope header value. The envelope must be signed by the issuer
    /// (`method = "tenzro_addCredential"`, `params_hash` over
    /// [`identity_credential_params`]). `proof_value` is an optional hex
    /// Ed25519 signature by the issuer over the credential subject's
    /// canonical bytes (`{"claims":{...},"id":"<did>"}` sorted-key JSON).
    pub async fn add_credential_presigned(
        &self,
        did: &str,
        credential_type: &str,
        issuer: &str,
        claims: &serde_json::Value,
        envelope_header: &str,
        proof_value: Option<&str>,
        proof_type: Option<&str>,
    ) -> SdkResult<serde_json::Value> {
        let mut params = serde_json::json!({
            "did": did,
            "type": credential_type,
            "issuer": issuer,
            "claims": claims,
            "envelope": envelope_header,
        });
        if let Some(pv) = proof_value {
            params["proof_value"] = serde_json::json!(pv);
            params["proof_type"] =
                serde_json::json!(proof_type.unwrap_or("Ed25519Signature2020"));
        }
        self.rpc.call("tenzro_addCredential", params).await
    }

    /// Adds a verifiable credential, signing the DID envelope locally with
    /// `signer` (the issuer's Ed25519 key). When `sign_proof` is true the
    /// same signer also produces the durable credential proof over the
    /// subject's canonical bytes.
    pub async fn add_credential(
        &self,
        signer: &Arc<dyn crate::app::EnvelopeSigner>,
        did: &str,
        credential_type: &str,
        issuer: &str,
        claims: &serde_json::Value,
        sign_proof: bool,
    ) -> SdkResult<serde_json::Value> {
        let claims_canonical = serde_json::to_vec(claims)
            .map_err(|e| crate::error::SdkError::InvalidParameter(e.to_string()))?;
        let params =
            identity_credential_params(did, credential_type, issuer, &claims_canonical);
        let env =
            crate::app::build_envelope(signer, issuer, "tenzro_addCredential", &params).await?;
        let proof_value = if sign_proof {
            let subject_bytes = credential_subject_canonical_bytes(did, claims)?;
            let sig = signer
                .sign_preimage(&subject_bytes)
                .await
                .map_err(crate::error::SdkError::from)?;
            Some(hex::encode(sig))
        } else {
            None
        };
        self.add_credential_presigned(
            did,
            credential_type,
            issuer,
            claims,
            &env.to_header_value(),
            proof_value.as_deref(),
            None,
        )
        .await
    }

    /// Adds a service endpoint to an identity with a pre-built DID-envelope
    /// header value. The envelope must be signed by the subject DID or its
    /// controller (`method = "tenzro_addService"`, `params_hash` over
    /// [`identity_service_params`]).
    pub async fn add_service_presigned(
        &self,
        did: &str,
        service_type: &str,
        endpoint: &str,
        envelope_header: &str,
    ) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_addService",
                serde_json::json!({
                    "did": did,
                    "type": service_type,
                    "endpoint": endpoint,
                    "envelope": envelope_header,
                }),
            )
            .await
    }

    /// Adds a service endpoint, signing the DID envelope locally. `signer_did`
    /// is the DID whose key `signer` holds — the subject DID itself, or its
    /// controller when the controller authorizes the write.
    pub async fn add_service(
        &self,
        signer: &Arc<dyn crate::app::EnvelopeSigner>,
        signer_did: &str,
        did: &str,
        service_type: &str,
        endpoint: &str,
    ) -> SdkResult<serde_json::Value> {
        let params = identity_service_params(did, service_type, endpoint);
        let env =
            crate::app::build_envelope(signer, signer_did, "tenzro_addService", &params).await?;
        self.add_service_presigned(did, service_type, endpoint, &env.to_header_value())
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

    /// Lists the public JWK Set published by this node (RFC 7517 / RFC 9421 keyid resolution).
    ///
    /// Each entry's `kid` field is the canonical RFC 9421 keyid in the form
    /// `<did>#<key_fragment>` and resolves directly via [`get_jwk`].
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
    /// let jwks = identity.list_jwks().await?;
    /// for jwk in &jwks.keys {
    ///     println!("{} ({})", jwk.kid.as_deref().unwrap_or(""), jwk.kty);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_jwks(&self) -> SdkResult<JwkSet> {
        self.rpc
            .call("tenzro_listAgentJwks", serde_json::json!([]))
            .await
    }

    /// Looks up a single JWK by its `kid` (RFC 9421 keyid resolution).
    ///
    /// `keyid` is typically `<did>#<key_fragment>`.
    pub async fn get_jwk(&self, keyid: &str) -> SdkResult<Jwk> {
        self.rpc
            .call("tenzro_getAgentJwk", serde_json::json!([keyid]))
            .await
    }
}

// ---------------------------------------------------------------------------
// Canonical params for identity-write envelopes
// ---------------------------------------------------------------------------

const IDENTITY_CREDENTIAL_DOMAIN: &[u8] = b"tenzro/identity/credential";
const IDENTITY_SERVICE_DOMAIN: &[u8] = b"tenzro/identity/service";
const IDENTITY_CLAIM_DOMAIN: &[u8] = b"tenzro/identity/claim";

/// Canonical params for `tenzro_addCredential`, byte-identical to the node's
/// builder. `claims_canonical` is the `serde_json` serialization of the claims
/// object — `Value::Object` is sorted-key, so client and server derive
/// identical bytes.
pub fn identity_credential_params(
    did: &str,
    credential_type: &str,
    issuer: &str,
    claims_canonical: &[u8],
) -> Vec<u8> {
    let mut buf = IDENTITY_CREDENTIAL_DOMAIN.to_vec();
    crate::app::push_bytes(&mut buf, did.as_bytes());
    crate::app::push_bytes(&mut buf, credential_type.as_bytes());
    crate::app::push_bytes(&mut buf, issuer.as_bytes());
    crate::app::push_bytes(&mut buf, claims_canonical);
    buf
}

/// Canonical params for `tenzro_addService`, byte-identical to the node's
/// builder.
pub fn identity_service_params(did: &str, service_type: &str, endpoint: &str) -> Vec<u8> {
    let mut buf = IDENTITY_SERVICE_DOMAIN.to_vec();
    crate::app::push_bytes(&mut buf, did.as_bytes());
    crate::app::push_bytes(&mut buf, service_type.as_bytes());
    crate::app::push_bytes(&mut buf, endpoint.as_bytes());
    buf
}

/// Canonical params for `tenzro_addIdentityClaim`, byte-identical to the
/// node's builder. `address_hex_lower` is the 0x-stripped lowercase hex form.
pub fn identity_claim_params(
    address_hex_lower: &str,
    topic: u64,
    issuer: &str,
    data: &str,
    valid_from: &str,
    valid_to: &str,
) -> Vec<u8> {
    let mut buf = IDENTITY_CLAIM_DOMAIN.to_vec();
    crate::app::push_bytes(&mut buf, address_hex_lower.as_bytes());
    buf.extend_from_slice(&topic.to_be_bytes());
    crate::app::push_bytes(&mut buf, issuer.as_bytes());
    crate::app::push_bytes(&mut buf, data.as_bytes());
    crate::app::push_bytes(&mut buf, valid_from.as_bytes());
    crate::app::push_bytes(&mut buf, valid_to.as_bytes());
    buf
}

/// Canonical bytes of the credential subject the issuer's durable proof
/// signs — the sorted-key compact JSON of `{"claims": {...}, "id": "<did>"}`,
/// matching `tenzro_identity::credential::CredentialSubject::canonical_bytes`.
pub fn credential_subject_canonical_bytes(
    did: &str,
    claims: &serde_json::Value,
) -> SdkResult<Vec<u8>> {
    let subject = serde_json::json!({ "id": did, "claims": claims });
    serde_json::to_vec(&subject)
        .map_err(|e| crate::error::SdkError::InvalidParameter(e.to_string()))
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

/// JSON Web Key (RFC 7517 §4) as published by `tenzro_listAgentJwks` /
/// `tenzro_getAgentJwk`.
///
/// Only the public-key half of the key material is ever published. Algorithm-
/// dependent fields (`x`, `y`, `n`, `e`, `crv`) are populated according to
/// RFC 7518; unused fields are omitted.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Jwk {
    /// Key type (RFC 7518 §6) — `OKP` (Ed25519), `EC` (P-256, P-384), `RSA`.
    pub kty: String,
    /// Key ID — canonical form `<did>#<key_fragment>` for Tenzro keys.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kid: Option<String>,
    /// JWA algorithm identifier (e.g., `EdDSA`, `ES256`, `ES384`, `PS256`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alg: Option<String>,
    /// Curve identifier for `OKP` / `EC` keys (`Ed25519`, `P-256`, `P-384`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crv: Option<String>,
    /// Public-key use — typically `sig`.
    #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    /// Permitted key operations (e.g., `["verify"]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_ops: Option<Vec<String>>,
    /// X coordinate (`OKP` raw key, `EC` x).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,
    /// Y coordinate (`EC` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
    /// RSA modulus (`RSA` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,
    /// RSA public exponent (`RSA` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub e: Option<String>,
}

/// JSON Web Key Set (RFC 7517 §5) — the wire format published at
/// `/.well-known/jwks.json` and via `tenzro_listAgentJwks`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JwkSet {
    /// The set of published JWKs.
    #[serde(default)]
    pub keys: Vec<Jwk>,
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
