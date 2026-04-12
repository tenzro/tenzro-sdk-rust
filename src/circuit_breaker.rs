//! Circuit Breaker SDK for Tenzro Network
//!
//! This module provides circuit breaker configuration and health monitoring
//! for model and TEE providers.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Circuit breaker client for provider health management
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let cb = client.circuit_breaker();
///
/// // Check provider health
/// let health = cb.get_provider_health("provider-1").await?;
/// println!("State: {}, failures: {}", health.state, health.failure_count);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct CircuitBreakerClient {
    rpc: Arc<RpcClient>,
}

impl CircuitBreakerClient {
    /// Creates a new circuit breaker client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Gets the health status of a provider's circuit breaker
    ///
    /// # Arguments
    ///
    /// * `provider_id` - The provider identifier
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let cb = client.circuit_breaker();
    /// let health = cb.get_provider_health("provider-1").await?;
    /// println!("Provider {} is {}", health.provider_id, health.state);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_provider_health(&self, provider_id: &str) -> SdkResult<ProviderHealth> {
        self.rpc
            .call(
                "tenzro_getProviderHealth",
                serde_json::json!([{
                    "provider_id": provider_id,
                }]),
            )
            .await
    }

    /// Lists all circuit breakers and their current states
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let cb = client.circuit_breaker();
    /// let breakers = cb.list_circuit_breakers().await?;
    /// for b in &breakers {
    ///     println!("{}: {} (threshold: {})", b.provider_id, b.state, b.failure_threshold);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_circuit_breakers(&self) -> SdkResult<Vec<CircuitBreakerStatus>> {
        self.rpc
            .call("tenzro_listCircuitBreakers", serde_json::json!([]))
            .await
    }

    /// Configures the circuit breaker for a provider
    ///
    /// # Arguments
    ///
    /// * `provider_id` - The provider identifier
    /// * `config` - Circuit breaker configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # use tenzro_sdk::circuit_breaker::CircuitBreakerConfig;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let cb = client.circuit_breaker();
    /// let breaker_config = CircuitBreakerConfig {
    ///     failure_threshold: 5,
    ///     recovery_timeout_secs: 60,
    ///     half_open_max_calls: 3,
    /// };
    /// let result = cb.configure_breaker("provider-1", breaker_config).await?;
    /// println!("Configured: {}", result.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn configure_breaker(
        &self,
        provider_id: &str,
        config: CircuitBreakerConfig,
    ) -> SdkResult<ConfigResult> {
        self.rpc
            .call(
                "tenzro_configureCircuitBreaker",
                serde_json::json!([{
                    "provider_id": provider_id,
                    "failure_threshold": config.failure_threshold,
                    "recovery_timeout_secs": config.recovery_timeout_secs,
                    "half_open_max_calls": config.half_open_max_calls,
                }]),
            )
            .await
    }

    /// Resets a circuit breaker to the closed (healthy) state
    ///
    /// # Arguments
    ///
    /// * `provider_id` - The provider identifier
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let cb = client.circuit_breaker();
    /// let result = cb.reset_breaker("provider-1").await?;
    /// println!("Reset: {}", result.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reset_breaker(&self, provider_id: &str) -> SdkResult<ResetResult> {
        self.rpc
            .call(
                "tenzro_resetCircuitBreaker",
                serde_json::json!([{
                    "provider_id": provider_id,
                }]),
            )
            .await
    }
}

/// Provider health status from the circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    /// Provider identifier
    #[serde(default)]
    pub provider_id: String,
    /// Circuit breaker state (e.g., "closed", "open", "half_open")
    #[serde(default)]
    pub state: String,
    /// Total failure count
    #[serde(default)]
    pub failure_count: u64,
    /// Total success count
    #[serde(default)]
    pub success_count: u64,
    /// Timestamp of last failure (Unix seconds, 0 if none)
    #[serde(default)]
    pub last_failure: u64,
    /// Timestamp of last success (Unix seconds, 0 if none)
    #[serde(default)]
    pub last_success: u64,
}

/// Circuit breaker status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStatus {
    /// Provider identifier
    #[serde(default)]
    pub provider_id: String,
    /// Circuit breaker state (e.g., "closed", "open", "half_open")
    #[serde(default)]
    pub state: String,
    /// Failure threshold before opening
    #[serde(default)]
    pub failure_threshold: u32,
    /// Recovery timeout in seconds
    #[serde(default)]
    pub recovery_timeout: u64,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures before the breaker opens
    #[serde(default)]
    pub failure_threshold: u32,
    /// Seconds to wait before transitioning from open to half-open
    #[serde(default)]
    pub recovery_timeout_secs: u64,
    /// Maximum calls allowed in half-open state before deciding
    #[serde(default)]
    pub half_open_max_calls: u32,
}

/// Result from configuring a circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResult {
    /// Provider identifier
    #[serde(default)]
    pub provider_id: String,
    /// Operation status
    #[serde(default)]
    pub status: String,
}

/// Result from resetting a circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetResult {
    /// Provider identifier
    #[serde(default)]
    pub provider_id: String,
    /// Operation status
    #[serde(default)]
    pub status: String,
}
