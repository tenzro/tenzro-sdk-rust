//! Core types for the Tenzro SDK
//!
//! Lightweight type definitions used across the SDK. These are self-contained
//! versions of the types from `tenzro-types` so the SDK can be published
//! standalone on crates.io without workspace path dependencies.

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Address
// ---------------------------------------------------------------------------

/// A 32-byte on-chain address.
///
/// Serialises to / deserialises from a `"0x..."` hex string for JSON-RPC
/// compatibility.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Address(pub [u8; 32]);

impl Address {
    /// Creates a new Address from a 32-byte array.
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the zero address (all bytes `0x00`).
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    /// Returns the underlying bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Creates an Address from a hex string (with or without `0x` prefix).
    ///
    /// Returns `None` if the string is not valid hex or is not exactly 32 bytes.
    pub fn from_hex(hex_str: &str) -> Option<Self> {
        let stripped = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(stripped).ok()?;
        if bytes.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Some(Self(arr))
        } else if bytes.len() == 20 {
            // EVM-style 20-byte address — left-pad to 32 bytes
            let mut arr = [0u8; 32];
            arr[12..].copy_from_slice(&bytes);
            Some(Self(arr))
        } else {
            None
        }
    }

    /// Returns the address as a `0x`-prefixed hex string.
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({})", self.to_hex())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Serialize for Address {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Address::from_hex(&s).ok_or_else(|| serde::de::Error::custom("invalid address hex"))
    }
}

impl Default for Address {
    fn default() -> Self {
        Self::zero()
    }
}

// ---------------------------------------------------------------------------
// ModelInfo
// ---------------------------------------------------------------------------

/// Information about an AI model registered on Tenzro Network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Unique model identifier
    #[serde(default)]
    pub model_id: String,
    /// Model name
    #[serde(default)]
    pub name: String,
    /// Model version
    #[serde(default)]
    pub version: String,
    /// Model description
    #[serde(default)]
    pub description: String,
    /// Model modality (e.g., "text", "image", "audio")
    #[serde(default)]
    pub modality: serde_json::Value,
    /// Model architecture
    #[serde(default)]
    pub architecture: String,
    /// Model provider address
    #[serde(default)]
    pub provider: serde_json::Value,
    /// Model hash for verification
    #[serde(default)]
    pub model_hash: serde_json::Value,
    /// Model parameters
    #[serde(default)]
    pub parameters: serde_json::Value,
    /// Model pricing
    #[serde(default)]
    pub pricing: serde_json::Value,
    /// Model status
    #[serde(default)]
    pub status: serde_json::Value,
    /// Model metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

// ---------------------------------------------------------------------------
// AgentIdentity
// ---------------------------------------------------------------------------

/// Identity of an AI agent on Tenzro Network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    /// Unique agent identifier
    #[serde(default)]
    pub agent_id: String,
    /// Agent on-chain address
    #[serde(default)]
    pub address: serde_json::Value,
    /// Agent name
    #[serde(default)]
    pub name: String,
    /// Agent version
    #[serde(default)]
    pub version: String,
    /// Agent creator/owner address
    #[serde(default)]
    pub creator: serde_json::Value,
}

// ---------------------------------------------------------------------------
// AgentTemplate
// ---------------------------------------------------------------------------

/// An agent template published to the Tenzro Network marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    /// Unique template identifier
    #[serde(default)]
    pub template_id: String,
    /// Template name
    #[serde(default)]
    pub name: String,
    /// Detailed description
    #[serde(default)]
    pub description: String,
    /// Template type (e.g., "autonomous", "specialist")
    #[serde(default)]
    pub template_type: serde_json::Value,
    /// Creator address
    #[serde(default)]
    pub creator: serde_json::Value,
    /// Version string
    #[serde(default)]
    pub version: String,
    /// Current status
    #[serde(default)]
    pub status: serde_json::Value,
    /// Creation timestamp
    #[serde(default)]
    pub created_at: serde_json::Value,
    /// Last update timestamp
    #[serde(default)]
    pub updated_at: serde_json::Value,
    /// Agent capabilities
    #[serde(default)]
    pub capabilities: Vec<serde_json::Value>,
    /// Runtime requirements
    #[serde(default)]
    pub runtime_requirements: serde_json::Value,
    /// Pricing model
    #[serde(default)]
    pub pricing: serde_json::Value,
    /// System prompt
    #[serde(default)]
    pub system_prompt: String,
    /// Example interactions
    #[serde(default)]
    pub examples: Vec<serde_json::Value>,
    /// Discovery tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Download count
    #[serde(default)]
    pub download_count: u64,
    /// Average rating (0-100)
    #[serde(default)]
    pub rating: u8,
    /// Content hash
    #[serde(default)]
    pub content_hash: Option<String>,
    /// Documentation URL
    #[serde(default)]
    pub docs_url: Option<String>,
    /// Optional creator DID (did:tenzro:... / did:pdis:...) bound at registration time
    #[serde(default)]
    pub creator_did: Option<String>,
    /// Creator payout wallet — mandatory for any non-free pricing, receives the
    /// per-invocation fee minus the `AGENT_MARKETPLACE_COMMISSION_BPS` network commission
    #[serde(default)]
    pub creator_wallet: Option<String>,
    /// Total successful invocations through `tenzro_runAgentTemplate`
    #[serde(default)]
    pub invocation_count: u64,
    /// Total TNZO revenue collected across all invocations (pre-split)
    #[serde(default)]
    pub total_revenue: u128,
}

// ---------------------------------------------------------------------------
// TaskInfo
// ---------------------------------------------------------------------------

/// A task posted to the Tenzro Network task marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    /// Unique task identifier
    #[serde(default)]
    pub task_id: String,
    /// Task title
    #[serde(default)]
    pub title: String,
    /// Task description
    #[serde(default)]
    pub description: String,
    /// Task type
    #[serde(default)]
    pub task_type: serde_json::Value,
    /// Poster address
    #[serde(default)]
    pub poster: serde_json::Value,
    /// Task status
    #[serde(default)]
    pub status: serde_json::Value,
    /// Maximum price in TNZO wei
    #[serde(default)]
    pub max_price: serde_json::Value,
    /// Task input data
    #[serde(default)]
    pub input: String,
    /// Task output/result
    #[serde(default)]
    pub output: Option<String>,
    /// Assigned agent
    #[serde(default)]
    pub assigned_agent: Option<String>,
    /// Creation timestamp
    #[serde(default)]
    pub created_at: serde_json::Value,
}

// ---------------------------------------------------------------------------
// TaskQuote
// ---------------------------------------------------------------------------

/// A quote submitted by a provider for a marketplace task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQuote {
    /// The task being quoted
    #[serde(default)]
    pub task_id: String,
    /// Provider address
    #[serde(default)]
    pub provider: serde_json::Value,
    /// Quoted price in TNZO wei
    #[serde(default)]
    pub price: serde_json::Value,
    /// Estimated duration in seconds
    #[serde(default)]
    pub estimated_duration_secs: u64,
    /// Model the provider will use
    #[serde(default)]
    pub model_id: String,
    /// Provider confidence (0-100)
    #[serde(default)]
    pub confidence: u8,
    /// Additional notes
    #[serde(default)]
    pub notes: Option<String>,
}
