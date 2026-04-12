//! Error types for the Tenzro SDK
//!
//! This module defines all error types that can occur when using the SDK.

use thiserror::Error;

/// Result type for SDK operations
pub type SdkResult<T> = Result<T, SdkError>;

/// Errors that can occur when using the Tenzro SDK
#[derive(Debug, Error)]
pub enum SdkError {
    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Inference error
    #[error("Inference error: {0}")]
    InferenceError(String),

    /// Settlement error
    #[error("Settlement error: {0}")]
    SettlementError(String),

    /// Wallet error
    #[error("Wallet error: {0}")]
    WalletError(String),

    /// Agent error
    #[error("Agent error: {0}")]
    AgentError(String),

    /// RPC error
    #[error("RPC error: {0}")]
    RpcError(String),

    /// Serialization error
    #[error("Serialization error")]
    SerializationError,

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Not found error
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Insufficient funds
    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: u64, available: u64 },

    /// Transaction failed
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
}

impl From<serde_json::Error> for SdkError {
    fn from(_: serde_json::Error) -> Self {
        SdkError::SerializationError
    }
}
