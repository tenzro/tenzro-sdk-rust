//! Agent Payment SDK for Tenzro Network
//!
//! Wraps the five `tenzro_*` agent-payment RPCs exposed by `tenzro-node`:
//! `tenzro_setSpendingPolicy`, `tenzro_getSpendingPolicy`,
//! `tenzro_agentPayForService`, `tenzro_getAgentDailySpend`, and
//! `tenzro_listAgentTransactions`. The wallet kernel's TypeScript SDK
//! (`agent-payments.ts`) calls the same RPC surface; the two SDKs are
//! kept in lockstep so an agent's runtime spending policy is observable
//! from either client.
//!
//! The runtime axis enforced by these RPCs is the per-machine
//! `SpendingPolicy` (max_per_transaction + max_daily_spend); the
//! protocol axis (`DelegationScope` set at identity registration) is
//! enforced separately by the payment gate.

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
/// println!("Spent today: {} / {}", spend.current_daily_spend, spend.max_daily_spend);
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

    /// Sets the runtime spending policy for a machine agent.
    ///
    /// Defines the per-transaction and daily-spend ceilings that the
    /// node-level payment gate enforces alongside the protocol-level
    /// `DelegationScope`.
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the machine agent (canonical
    ///   `did:tenzro:machine:...` form)
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
    ///     max_daily_spend: 10_000_000,
    ///     active: true,
    ///     allowed_services: vec!["inference".to_string(), "storage".to_string()],
    /// };
    /// let result = agent_payments.set_spending_policy(
    ///     "did:tenzro:machine:agent-1",
    ///     &policy,
    /// ).await?;
    /// println!("Policy set for {}", result.agent_did);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_spending_policy(
        &self,
        agent_did: &str,
        policy: &SpendingPolicy,
    ) -> SdkResult<PolicyResult> {
        self.rpc
            .call(
                "tenzro_setSpendingPolicy",
                serde_json::json!([{
                    "agent_did": agent_did,
                    "max_per_transaction": policy.max_per_transaction,
                    "max_daily_spend": policy.max_daily_spend,
                    "active": policy.active,
                    "allowed_services": policy.allowed_services,
                }]),
            )
            .await
    }

    /// Gets the current runtime spending policy for a machine agent.
    /// Returns `None` when no policy is bound.
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the machine agent
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
    /// if let Some(policy) = agent_payments
    ///     .get_spending_policy("did:tenzro:machine:agent-1")
    ///     .await?
    /// {
    ///     println!("Max per tx: {}", policy.max_per_transaction);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_spending_policy(
        &self,
        agent_did: &str,
    ) -> SdkResult<Option<SpendingPolicySnapshot>> {
        self.rpc
            .call(
                "tenzro_getSpendingPolicy",
                serde_json::json!([{
                    "agent_did": agent_did,
                }]),
            )
            .await
    }

    /// Records a service payment from an agent to a provider against the
    /// agent's runtime spending policy. Per-transaction and daily-spend
    /// ceilings are enforced *before* the payment is recorded; a violation
    /// returns a JSON-RPC error and no audit record is written.
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the paying machine agent
    /// * `provider` - Provider DID, hex address, or service URL
    /// * `amount` - Payment amount in smallest TNZO unit
    /// * `service_type` - Free-form category label (e.g. `"inference"`,
    ///   `"tee"`, `"settlement"`)
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
    /// println!("Receipt id: {}", receipt.receipt_id);
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

    /// Reads the current-day spend + remaining cap for a machine agent.
    /// Triggers the daily-window reset if the wall-clock has rolled past
    /// a UTC midnight since the last recorded transaction.
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the machine agent
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

    /// Lists the audit trail of service payments recorded for a machine
    /// agent in chronological order (oldest first within the returned
    /// slice). When `limit` is provided, only the most recent `limit`
    /// records are returned.
    ///
    /// # Arguments
    ///
    /// * `agent_did` - DID of the machine agent
    /// * `limit` - Optional cap on the number of records returned
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
    /// let txs = agent_payments.list_transactions(
    ///     "did:tenzro:machine:agent-1",
    ///     Some(20),
    /// ).await?;
    /// for tx in &txs {
    ///     println!("{}: {} for {}", tx.receipt_id, tx.amount, tx.service_type);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_transactions(
        &self,
        agent_did: &str,
        limit: Option<u32>,
    ) -> SdkResult<Vec<AgentTransactionRecord>> {
        let mut params = serde_json::json!({
            "agent_did": agent_did,
        });
        if let Some(n) = limit {
            params["limit"] = serde_json::json!(n);
        }
        self.rpc
            .call(
                "tenzro_listAgentTransactions",
                serde_json::json!([params]),
            )
            .await
    }
}

/// Runtime spending policy for a machine agent.
///
/// Mirrors the node-side [`tenzro_agent::SpendingPolicy`] shape exposed
/// over the wallet-kernel public API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingPolicy {
    /// Maximum amount per single transaction (smallest TNZO unit)
    #[serde(default)]
    pub max_per_transaction: u64,
    /// Maximum daily spend (smallest TNZO unit)
    #[serde(default)]
    pub max_daily_spend: u64,
    /// Whether the policy is currently enforced. When `false`, the gate
    /// short-circuits to "allow" — useful for parking a machine without
    /// rewriting its policy.
    #[serde(default = "default_active")]
    pub active: bool,
    /// Human-readable hints to the wallet kernel about which service
    /// categories the operator opted the agent into. Not enforced at the
    /// runtime layer; kept for client-side UX.
    #[serde(default)]
    pub allowed_services: Vec<String>,
}

fn default_active() -> bool {
    true
}

/// Snapshot returned by `get_spending_policy` — the policy plus the
/// node's current daily-window state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingPolicySnapshot {
    /// DID of the machine agent
    #[serde(default)]
    pub agent_did: String,
    /// Maximum amount per single transaction
    #[serde(default)]
    pub max_per_transaction: u64,
    /// Maximum daily spend
    #[serde(default)]
    pub max_daily_spend: u64,
    /// Spend recorded for the current daily window
    #[serde(default)]
    pub current_daily_spend: u64,
    /// Unix-seconds timestamp when the daily window last reset
    #[serde(default)]
    pub last_reset: i64,
    /// Whether the policy is currently enforced
    #[serde(default = "default_active")]
    pub active: bool,
}

/// Receipt for a service payment recorded via `pay_for_service`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPaymentReceipt {
    /// DID of the paying agent
    #[serde(default)]
    pub agent_did: String,
    /// Provider counterparty
    #[serde(default)]
    pub provider: String,
    /// Settled amount
    #[serde(default)]
    pub amount: u64,
    /// Service category label
    #[serde(default)]
    pub service_type: String,
    /// Receipt identifier minted by the node
    #[serde(default)]
    pub receipt_id: String,
    /// Unix-seconds timestamp when the payment cleared
    #[serde(default)]
    pub timestamp: i64,
    /// `true` when the runtime gate accepted the payment
    #[serde(default)]
    pub success: bool,
}

/// Daily spend summary returned by `get_daily_spend`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySpend {
    /// DID of the agent
    #[serde(default)]
    pub agent_did: String,
    /// Spend recorded for the current daily window
    #[serde(default)]
    pub current_daily_spend: u64,
    /// Daily cap configured on the policy
    #[serde(default)]
    pub max_daily_spend: u64,
    /// Remaining spend before the daily cap is hit
    #[serde(default)]
    pub remaining: u64,
    /// Unix-seconds timestamp when the daily window last reset
    #[serde(default)]
    pub last_reset: i64,
}

/// Single audit record returned by `list_transactions`. Mirrors the
/// node-side [`tenzro_agent::AgentTransactionRecord`] shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTransactionRecord {
    /// DID of the paying agent
    #[serde(default)]
    pub agent_did: String,
    /// Provider counterparty
    #[serde(default)]
    pub provider: String,
    /// Service category label
    #[serde(default)]
    pub service_type: String,
    /// Settled amount
    #[serde(default)]
    pub amount: u64,
    /// Unix-seconds timestamp when the payment cleared
    #[serde(default)]
    pub timestamp: i64,
    /// Receipt identifier minted by the node
    #[serde(default)]
    pub receipt_id: String,
}

/// Result of `set_spending_policy` — confirms which DID's policy was updated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    /// `true` when the policy was successfully written
    #[serde(default)]
    pub success: bool,
    /// DID of the agent the policy was set for
    #[serde(default)]
    pub agent_did: String,
}
