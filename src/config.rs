//! SDK configuration types
//!
//! This module provides configuration options for the Tenzro SDK client.

use crate::error::{SdkError, SdkResult};

/// SDK configuration
///
/// Use the builder pattern to configure the SDK client.
///
/// # Example
///
/// ```no_run
/// use tenzro_sdk::config::SdkConfig;
///
/// let config = SdkConfig::builder()
///     .endpoint("https://rpc.tenzro.network")
///     .timeout(10000)
///     .max_retries(5)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct SdkConfig {
    /// RPC endpoint URL
    pub endpoint: String,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Optional API key for authentication
    pub api_key: Option<String>,
    /// Chain ID (default: 1337 for testnet)
    pub chain_id: u64,
}

impl SdkConfig {
    /// Creates a new configuration builder
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tenzro_sdk::config::SdkConfig;
    ///
    /// let config = SdkConfig::builder()
    ///     .endpoint("https://rpc.tenzro.network")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> SdkConfigBuilder {
        SdkConfigBuilder::default()
    }

    /// Creates a mainnet configuration
    ///
    /// Note: Mainnet is not yet live. This configuration is a placeholder.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tenzro_sdk::config::SdkConfig;
    ///
    /// let config = SdkConfig::mainnet();
    /// ```
    pub fn mainnet() -> Self {
        Self {
            endpoint: "https://rpc.tenzro.network".to_string(),
            timeout_ms: 30000,
            max_retries: 3,
            api_key: None,
            chain_id: 1337,
        }
    }

    /// Creates a testnet configuration
    ///
    /// Connects to the live Tenzro testnet at `rpc.tenzro.network`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tenzro_sdk::config::SdkConfig;
    ///
    /// let config = SdkConfig::testnet();
    /// ```
    pub fn testnet() -> Self {
        Self {
            endpoint: "https://rpc.tenzro.network".to_string(),
            timeout_ms: 30000,
            max_retries: 3,
            api_key: None,
            chain_id: 1337,
        }
    }

    /// Creates a local development configuration
    ///
    /// Connects to a local Tenzro node at `http://localhost:8545`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tenzro_sdk::config::SdkConfig;
    ///
    /// let config = SdkConfig::local();
    /// ```
    pub fn local() -> Self {
        Self {
            endpoint: "http://localhost:8545".to_string(),
            timeout_ms: 10000,
            max_retries: 1,
            api_key: None,
            chain_id: 1337,
        }
    }
}

/// Builder for SDK configuration
#[derive(Debug, Default)]
pub struct SdkConfigBuilder {
    endpoint: Option<String>,
    timeout_ms: Option<u64>,
    max_retries: Option<u32>,
    api_key: Option<String>,
    chain_id: Option<u64>,
}

impl SdkConfigBuilder {
    /// Sets the RPC endpoint
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Sets the request timeout in milliseconds
    pub fn timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Sets the maximum number of retry attempts
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Sets the API key for authentication
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Sets the chain ID
    pub fn chain_id(mut self, chain_id: u64) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Builds the SDK configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint is not set.
    pub fn build(self) -> SdkResult<SdkConfig> {
        let endpoint = self
            .endpoint
            .ok_or_else(|| SdkError::InvalidConfiguration("endpoint is required".to_string()))?;

        Ok(SdkConfig {
            endpoint,
            timeout_ms: self.timeout_ms.unwrap_or(30000),
            max_retries: self.max_retries.unwrap_or(3),
            api_key: self.api_key,
            chain_id: self.chain_id.unwrap_or(1337),
        })
    }
}
