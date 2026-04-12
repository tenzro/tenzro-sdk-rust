//! Agent Transaction Executor SDK for Tenzro Network
//!
//! This module provides agent-level spending policies and transaction
//! execution, enabling autonomous agents to pay for services within
//! configurable limits.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Agent payment client for spending policy and transaction management
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let agent_payments = client.agent_payments();
///
/// // Check an agent's daily spend
/// let spend = agent_payments.get_daily_spend("did:tenzro:machine:agent-1").await?;
/// println!("Spent today: {} / {}", spend.total_today, spend.policy_limit);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct AgentPaymentClient {
    rpc: Arc<RpcClient>,
}

impl AgentPaymentClient {
    /// Creates a new agent payment client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Sets the spending policy for an agent
    ///
    /// Defines per-transaction and daily limits, allowed recipients,
    /// and whether TEE attestation is required for payments.
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the agent
    /// * `policy` - The spending policy to apply
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # use tenzro_sdk::agent_payments::SpendingPolicy;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let agent_payments = client.agent_payments();
    /// let policy = SpendingPolicy {
    ///     max_per_transaction: 1_000_000,
    ///     max_daily_total: 10_000_000,
    ///     allowed_recipients: vec![],
    ///     require_tee_attestation: false,
    ///     allowed_operations: vec!["inference".to_string(), "storage".to_string()],
    /// };
    /// let result = agent_payments.set_spending_policy(
    ///     "did:tenzro:machine:agent-1",
    ///     policy,
    /// ).await?;
    /// println!("Policy active from: {}", result.effective_from);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_spending_policy(
        &self,
        agent_did: &str,
        policy: SpendingPolicy,
    ) -> SdkResult<PolicyResult> {
        self.rpc
            .call(
                "tenzro_setAgentSpendingPolicy",
                serde_json::json!([{
                    "agent_did": agent_did,
                    "max_per_transaction": policy.max_per_transaction,
                    "max_daily_total": policy.max_daily_total,
                    "allowed_recipients": policy.allowed_recipients,
                    "require_tee_attestation": policy.require_tee_attestation,
                    "allowed_operations": policy.allowed_operations,
                }]),
            )
            .await
    }

    /// Gets the current spending policy for an agent
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the agent
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let agent_payments = client.agent_payments();
    /// let policy = agent_payments.get_spending_policy("did:tenzro:machine:agent-1").await?;
    /// println!("Max per tx: {}", policy.max_per_transaction);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_spending_policy(&self, agent_did: &str) -> SdkResult<SpendingPolicy> {
        self.rpc
            .call(
                "tenzro_getAgentSpendingPolicy",
                serde_json::json!([{
                    "agent_did": agent_did,
                }]),
            )
            .await
    }

    /// Executes a payment from an agent to a provider for a service
    ///
    /// The payment is validated against the agent's spending policy before
    /// execution.
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the paying agent
    /// * `provider` - Provider address or DID
    /// * `amount` - Payment amount
    /// * `service_type` - Type of service being paid for
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let agent_payments = client.agent_payments();
    /// let receipt = agent_payments.pay_for_service(
    ///     "did:tenzro:machine:agent-1",
    ///     "did:tenzro:machine:provider-1",
    ///     500_000,
    ///     "inference",
    /// ).await?;
    /// println!("Payment tx: {}", receipt.tx_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pay_for_service(
        &self,
        agent_did: &str,
        provider: &str,
        amount: u64,
        service_type: &str,
    ) -> SdkResult<AgentPaymentReceipt> {
        self.rpc
            .call(
                "tenzro_agentPayForService",
                serde_json::json!([{
                    "agent_did": agent_did,
                    "provider": provider,
                    "amount": amount,
                    "service_type": service_type,
                }]),
            )
            .await
    }

    /// Gets the daily spend summary for an agent
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the agent
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let agent_payments = client.agent_payments();
    /// let spend = agent_payments.get_daily_spend("did:tenzro:machine:agent-1").await?;
    /// println!("Remaining budget: {}", spend.remaining);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_daily_spend(&self, agent_did: &str) -> SdkResult<DailySpend> {
        self.rpc
            .call(
                "tenzro_getAgentDailySpend",
                serde_json::json!([{
                    "agent_did": agent_did,
                }]),
            )
            .await
    }

    /// Lists recent transactions for an agent
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the agent
    /// * `limit` - Maximum number of transactions to return
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let agent_payments = client.agent_payments();
    /// let txs = agent_payments.list_agent_transactions(
    ///     "did:tenzro:machine:agent-1",
    ///     20,
    /// ).await?;
    /// for tx in &txs {
    ///     println!("{}: {} for {}", tx.tx_id, tx.amount, tx.service);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_agent_transactions(
        &self,
        agent_did: &str,
        limit: u32,
    ) -> SdkResult<Vec<AgentTransaction>> {
        self.rpc
            .call(
                "tenzro_listAgentTransactions",
                serde_json::json!([{
                    "agent_did": agent_did,
                    "limit": limit,
                }]),
            )
            .await
    }
}

/// Spending policy for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingPolicy {
    /// Maximum amount per single transaction
    #[serde(default)]
    pub max_per_transaction: u64,
    /// Maximum total daily spend
    #[serde(default)]
    pub max_daily_total: u64,
    /// Allowed recipient addresses or DIDs (empty = all allowed)
    #[serde(default)]
    pub allowed_recipients: Vec<String>,
    /// Whether TEE attestation is required for payments
    #[serde(default)]
    pub require_tee_attestation: bool,
    /// Allowed operation types (empty = all allowed)
    #[serde(default)]
    pub allowed_operations: Vec<String>,
}

/// Receipt for an agent payment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPaymentReceipt {
    /// Receipt identifier
    #[serde(default)]
    pub receipt_id: String,
    /// DID of the paying agent
    #[serde(default)]
    pub agent_did: String,
    /// Provider address or DID
    #[serde(default)]
    pub provider: String,
    /// Amount paid
    #[serde(default)]
    pub amount: u64,
    /// Service type
    #[serde(default)]
    pub service: String,
    /// On-chain transaction hash
    #[serde(default)]
    pub tx_hash: String,
    /// Payment timestamp (Unix seconds)
    #[serde(default)]
    pub timestamp: u64,
}

/// Daily spend summary for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySpend {
    /// DID of the agent
    #[serde(default)]
    pub agent_did: String,
    /// Total spent today
    #[serde(default)]
    pub total_today: u64,
    /// Remaining daily budget
    #[serde(default)]
    pub remaining: u64,
    /// Policy daily limit
    #[serde(default)]
    pub policy_limit: u64,
    /// Daily reset timestamp (Unix seconds)
    #[serde(default)]
    pub reset_at: u64,
}

/// A single agent transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTransaction {
    /// Transaction identifier
    #[serde(default)]
    pub tx_id: String,
    /// DID of the agent
    #[serde(default)]
    pub agent_did: String,
    /// Amount paid
    #[serde(default)]
    pub amount: u64,
    /// Service type
    #[serde(default)]
    pub service: String,
    /// Transaction status (e.g., "confirmed", "pending", "failed")
    #[serde(default)]
    pub status: String,
    /// Transaction timestamp (Unix seconds)
    #[serde(default)]
    pub timestamp: u64,
}

/// Result from setting a spending policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    /// DID of the agent
    #[serde(default)]
    pub agent_did: String,
    /// Hash of the policy for verification
    #[serde(default)]
    pub policy_hash: String,
    /// Timestamp when the policy becomes effective
    #[serde(default)]
    pub effective_from: String,
}
