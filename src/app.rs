//! Application client for developer-funded app pattern
//!
//! The `AppClient` is the top-level developer entry point for building applications
//! on Tenzro Network. It wraps a master wallet and provides methods to:
//!
//! - Create and fund user sub-wallets
//! - Sponsor gas for user transactions (paymaster)
//! - Manage spending policies and session keys
//! - Track usage and costs
//!
//! # Architecture
//!
//! ```text
//! Developer registers app -> gets API key + master wallet
//!     |
//!     +-- Master wallet holds TNZO (funded by developer)
//!     |
//!     +-- App spawns user sub-wallets (auto-funded from master)
//!     |     +-- Each sub-wallet has spending limits, session scopes
//!     |
//!     +-- Master wallet acts as paymaster:
//!     |     +-- Pays gas for user transactions
//!     |     +-- Pays for inference requests
//!     |     +-- Pays for agent operations
//!     |     +-- Pays for bridge fees
//!     |
//!     +-- Developer dashboard: usage, costs, rate limits
//! ```
//!
//! # Example
//!
//! ```no_run
//! use tenzro_sdk::AppClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let app = AppClient::new("https://rpc.tenzro.network", "your-master-wallet-private-key").await?;
//!
//!     // Create a user wallet (funded from master)
//!     let user = app.create_user_wallet("alice", 100_000_000_000_000_000).await?; // 0.1 TNZO
//!
//!     // Sponsor an inference request for the user
//!     let result = app.sponsor_inference(&user.address, "gemma3-270m", "Hello world").await?;
//!
//!     // Check master wallet balance and usage
//!     let stats = app.get_usage_stats().await?;
//!     println!("Total gas spent: {} wei", stats.total_gas_spent);
//!     Ok(())
//! }
//! ```

use crate::client::TenzroClient;
use crate::config::SdkConfig;
use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Re-export custody types used in our public API so callers can reference them
// via `tenzro_sdk::app::SessionKey` and `tenzro_sdk::app::SpendingPolicy`.
pub use crate::custody::{SessionKey, SpendingPolicy};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Master wallet information derived from the developer's private key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterWallet {
    /// Hex-encoded master wallet address (with 0x prefix)
    pub address: String,
    /// Hex-encoded public key
    pub public_key: String,
}

/// A user sub-wallet created and funded by the master wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWallet {
    /// Hex-encoded user wallet address (with 0x prefix)
    pub address: String,
    /// Human-readable label
    pub label: String,
    /// ISO-8601 creation timestamp
    pub created_at: String,
    /// Optional spending policy applied to this wallet
    pub spending_policy: Option<SpendingPolicy>,
}

/// Result of funding a user wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundResult {
    /// Transaction hash of the funding transfer
    pub tx_hash: String,
    /// Amount transferred in wei
    pub amount: u128,
}

/// Aggregated usage statistics for the master wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// Total gas spent across all sponsored transactions (wei)
    pub total_gas_spent: u128,
    /// Estimated total cost of inference requests (TNZO, human-readable)
    pub total_inference_cost: f64,
    /// Total bridge fees paid (wei)
    pub total_bridge_fees: u128,
    /// Number of user wallets created
    pub user_count: u32,
    /// Total number of sponsored transactions
    pub transaction_count: u64,
}

/// Result of a sponsored inference request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    /// Model output text
    pub output: String,
    /// Number of tokens generated
    pub tokens: u32,
    /// Estimated cost in TNZO (human-readable)
    pub cost: f64,
    /// Model used
    pub model_id: String,
}

/// Result of a sponsored agent registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// Registered agent ID
    pub agent_id: String,
    /// Agent's wallet address
    pub wallet_address: String,
}

/// Result of a sponsored bridge transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeResult {
    /// Bridge transaction hash
    pub tx_hash: String,
    /// Current status of the bridge transfer
    pub status: String,
}

/// Result of a sponsored task posting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Posted task ID
    pub task_id: String,
}

/// Result of a sponsored transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxResult {
    /// Transaction hash
    pub tx_hash: String,
}

// ---------------------------------------------------------------------------
// Internal tracking state
// ---------------------------------------------------------------------------

/// In-memory tracker for user wallets and cumulative usage.
struct AppState {
    users: HashMap<String, UserWallet>,
    policies: HashMap<String, SpendingPolicy>,
    sessions: HashMap<String, SessionKey>,
    stats: UsageStats,
}

impl AppState {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
            policies: HashMap::new(),
            sessions: HashMap::new(),
            stats: UsageStats {
                total_gas_spent: 0,
                total_inference_cost: 0.0,
                total_bridge_fees: 0,
                user_count: 0,
                transaction_count: 0,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// AppClient
// ---------------------------------------------------------------------------

/// Application client -- the primary interface for developers building on Tenzro.
///
/// The `AppClient` wraps a master wallet and provides methods to:
/// - Create and fund user wallets
/// - Sponsor gas for user transactions (paymaster)
/// - Manage spending policies and session keys
/// - Track usage and costs
///
/// # Example
///
/// ```no_run
/// use tenzro_sdk::AppClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let app = AppClient::new("https://rpc.tenzro.network", "your-master-wallet-private-key").await?;
///
///     // Create a user wallet (funded from master)
///     let user = app.create_user_wallet("alice", 100_000_000_000_000_000).await?;
///
///     // Sponsor an inference request
///     let result = app.sponsor_inference(&user.address, "gemma3-270m", "Hello world").await?;
///
///     // Check usage
///     let stats = app.get_usage_stats().await?;
///     println!("Total gas spent: {} wei", stats.total_gas_spent);
///     Ok(())
/// }
/// ```
pub struct AppClient {
    rpc: Arc<RpcClient>,
    client: TenzroClient,
    master_wallet: MasterWallet,
    state: Arc<RwLock<AppState>>,
}

impl AppClient {
    /// Create a new `AppClient` with a master wallet.
    ///
    /// The `master_private_key` should be the hex-encoded Ed25519 or Secp256k1
    /// private key that controls the developer's master wallet.
    ///
    /// The constructor derives the master address and public key via
    /// `tenzro_createAccount`, which provisions a server-side keypair and
    /// returns the corresponding address.
    pub async fn new(rpc_url: &str, master_private_key: &str) -> SdkResult<Self> {
        let config = SdkConfig::builder()
            .endpoint(rpc_url)
            .build()?;
        let client = TenzroClient::connect(config).await?;
        let rpc = client.rpc.clone();

        // Derive master wallet address from the private key.
        // We call tenzro_createAccount with the key material so the node
        // can resolve the corresponding address and public key.
        let wallet_info: serde_json::Value = rpc
            .call(
                "tenzro_createAccount",
                serde_json::json!([{
                    "private_key": master_private_key,
                }]),
            )
            .await?;

        let address = wallet_info["address"]
            .as_str()
            .unwrap_or("0x0")
            .to_string();
        let public_key = wallet_info["public_key"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let master_wallet = MasterWallet {
            address,
            public_key,
        };

        tracing::info!(
            "AppClient initialized with master wallet {}",
            master_wallet.address
        );

        Ok(Self {
            rpc,
            client,
            master_wallet,
            state: Arc::new(RwLock::new(AppState::new())),
        })
    }

    /// Connect using an API key that resolves to a master wallet on the server.
    ///
    /// The API key is passed as a bearer token in every RPC call. The server
    /// maps it to the pre-registered master wallet.
    pub async fn from_api_key(rpc_url: &str, api_key: &str) -> SdkResult<Self> {
        let config = SdkConfig::builder()
            .endpoint(rpc_url)
            .api_key(api_key)
            .build()?;
        let client = TenzroClient::connect(config).await?;
        let rpc = client.rpc.clone();

        // Resolve the master wallet associated with this API key.
        let wallet_info: serde_json::Value = rpc
            .call(
                "tenzro_resolveApiKey",
                serde_json::json!([{ "api_key": api_key }]),
            )
            .await?;

        let address = wallet_info["address"]
            .as_str()
            .unwrap_or("0x0")
            .to_string();
        let public_key = wallet_info["public_key"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let master_wallet = MasterWallet {
            address,
            public_key,
        };

        tracing::info!(
            "AppClient initialized via API key with master wallet {}",
            master_wallet.address
        );

        Ok(Self {
            rpc,
            client,
            master_wallet,
            state: Arc::new(RwLock::new(AppState::new())),
        })
    }

    // -----------------------------------------------------------------------
    // User Management
    // -----------------------------------------------------------------------

    /// Create a sub-wallet for a user, funded from the master wallet.
    ///
    /// 1. Creates a new keypair via `tenzro_createWallet`
    /// 2. Transfers `initial_funding_wei` TNZO from master to the new wallet
    /// 3. Tracks the user locally for policy enforcement
    pub async fn create_user_wallet(
        &self,
        label: &str,
        initial_funding_wei: u128,
    ) -> SdkResult<UserWallet> {
        // Step 1: create a new wallet
        let wallet_info: serde_json::Value = self
            .rpc
            .call("tenzro_createWallet", serde_json::json!([]))
            .await?;

        let user_address = wallet_info["address"]
            .as_str()
            .unwrap_or("0x0")
            .to_string();

        // Step 2: fund from master wallet
        if initial_funding_wei > 0 {
            let _tx_hash: String = self
                .rpc
                .call(
                    "eth_sendRawTransaction",
                    serde_json::json!([{
                        "from": self.master_wallet.address,
                        "to": user_address,
                        "value": format!("0x{:x}", initial_funding_wei),
                    }]),
                )
                .await?;
        }

        let now = chrono_now_iso();
        let user = UserWallet {
            address: user_address.clone(),
            label: label.to_string(),
            created_at: now,
            spending_policy: None,
        };

        // Track locally
        let mut state = self.state.write().await;
        state.users.insert(user_address, user.clone());
        state.stats.user_count += 1;
        state.stats.transaction_count += 1;

        tracing::info!("Created user wallet '{}' at {}", label, user.address);
        Ok(user)
    }

    /// Fund an existing user wallet from the master wallet.
    pub async fn fund_user_wallet(
        &self,
        user_address: &str,
        amount_wei: u128,
    ) -> SdkResult<FundResult> {
        let tx_hash: String = self
            .rpc
            .call(
                "eth_sendRawTransaction",
                serde_json::json!([{
                    "from": self.master_wallet.address,
                    "to": user_address,
                    "value": format!("0x{:x}", amount_wei),
                }]),
            )
            .await?;

        let mut state = self.state.write().await;
        state.stats.transaction_count += 1;

        Ok(FundResult {
            tx_hash,
            amount: amount_wei,
        })
    }

    /// List all user wallets created by this app.
    pub async fn list_user_wallets(&self) -> SdkResult<Vec<UserWallet>> {
        let state = self.state.read().await;
        Ok(state.users.values().cloned().collect())
    }

    /// Set spending limits for a user wallet.
    ///
    /// The policy is enforced locally before submitting sponsored transactions.
    pub async fn set_user_limits(
        &self,
        user_address: &str,
        daily_limit: u128,
        per_tx_limit: u128,
    ) -> SdkResult<SpendingPolicy> {
        let policy = SpendingPolicy {
            daily_limit,
            per_tx_limit,
            daily_spent: 0,
        };

        let mut state = self.state.write().await;
        state
            .policies
            .insert(user_address.to_string(), policy.clone());

        // Also update the user record if present
        if let Some(user) = state.users.get_mut(user_address) {
            user.spending_policy = Some(policy.clone());
        }

        Ok(policy)
    }

    /// Create a session key for a user with scoped permissions and expiry.
    pub async fn create_session_key(
        &self,
        user_address: &str,
        duration_secs: u64,
        allowed_operations: Vec<String>,
    ) -> SdkResult<SessionKey> {
        let session_id = format!(
            "sess_{}_{:x}",
            &user_address[..8.min(user_address.len())],
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );

        let expires_at_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + duration_secs;

        let session = SessionKey {
            session_id: session_id.clone(),
            expires_at: format!("{}Z", expires_at_secs),
            operations: allowed_operations,
        };

        let mut state = self.state.write().await;
        state.sessions.insert(session_id, session.clone());

        Ok(session)
    }

    // -----------------------------------------------------------------------
    // Sponsored Operations (master wallet pays)
    // -----------------------------------------------------------------------

    /// Send a transaction on behalf of a user (master pays gas).
    ///
    /// The transaction originates from the master wallet with the `to` field
    /// set to the target recipient. The `user_address` is recorded for
    /// spending-policy enforcement and usage tracking.
    pub async fn sponsor_transaction(
        &self,
        user_address: &str,
        to: &str,
        amount_wei: u128,
    ) -> SdkResult<TxResult> {
        self.enforce_spending_policy(user_address, amount_wei)
            .await?;

        let tx_hash: String = self
            .rpc
            .call(
                "eth_sendRawTransaction",
                serde_json::json!([{
                    "from": self.master_wallet.address,
                    "to": to,
                    "value": format!("0x{:x}", amount_wei),
                    "sponsor": self.master_wallet.address,
                    "on_behalf_of": user_address,
                }]),
            )
            .await?;

        let mut state = self.state.write().await;
        state.stats.transaction_count += 1;
        // Estimate gas cost at 21000 * 1 gwei for tracking
        state.stats.total_gas_spent += 21_000_000_000_000u128;

        if let Some(policy) = state.policies.get_mut(user_address) {
            policy.daily_spent += amount_wei;
        }

        Ok(TxResult { tx_hash })
    }

    /// Run inference on behalf of a user (master pays).
    ///
    /// Calls `tenzro_chat` with the master wallet as the billing address.
    pub async fn sponsor_inference(
        &self,
        user_address: &str,
        model_id: &str,
        message: &str,
    ) -> SdkResult<InferenceResult> {
        let response: serde_json::Value = self
            .rpc
            .call(
                "tenzro_chat",
                serde_json::json!([{
                    "model": model_id,
                    "messages": [{"role": "user", "content": message}],
                    "caller_address": self.master_wallet.address,
                    "on_behalf_of": user_address,
                }]),
            )
            .await?;

        let output = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let tokens = response["usage"]["total_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;
        let cost = response["usage"]["cost"]
            .as_f64()
            .unwrap_or(0.0);

        let mut state = self.state.write().await;
        state.stats.total_inference_cost += cost;
        state.stats.transaction_count += 1;

        Ok(InferenceResult {
            output,
            tokens,
            cost,
            model_id: model_id.to_string(),
        })
    }

    /// Spawn an agent on behalf of a user (master pays).
    ///
    /// Calls `tenzro_registerAgent` with the master wallet as the creator
    /// and funding source.
    pub async fn sponsor_agent(
        &self,
        user_address: &str,
        agent_name: &str,
        capabilities: Vec<String>,
    ) -> SdkResult<AgentResult> {
        let response: serde_json::Value = self
            .rpc
            .call(
                "tenzro_registerAgent",
                serde_json::json!([{
                    "name": agent_name,
                    "display_name": agent_name,
                    "capabilities": capabilities,
                    "creator": self.master_wallet.address,
                    "on_behalf_of": user_address,
                }]),
            )
            .await?;

        let agent_id = response["agent_id"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let wallet_address = response["wallet_address"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let mut state = self.state.write().await;
        state.stats.transaction_count += 1;

        Ok(AgentResult {
            agent_id,
            wallet_address,
        })
    }

    /// Bridge tokens on behalf of a user (master pays bridge fees).
    ///
    /// Calls `tenzro_bridgeTokens` funded by the master wallet.
    pub async fn sponsor_bridge(
        &self,
        user_address: &str,
        token: &str,
        from_chain: &str,
        to_chain: &str,
        amount: &str,
        recipient: &str,
    ) -> SdkResult<BridgeResult> {
        let response: serde_json::Value = self
            .rpc
            .call(
                "tenzro_bridgeTokens",
                serde_json::json!([{
                    "token": token,
                    "from_chain": from_chain,
                    "to_chain": to_chain,
                    "amount": amount,
                    "recipient": recipient,
                    "fee_payer": self.master_wallet.address,
                    "on_behalf_of": user_address,
                }]),
            )
            .await?;

        let tx_hash = response["tx_hash"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let status = response["status"]
            .as_str()
            .unwrap_or("pending")
            .to_string();

        let mut state = self.state.write().await;
        let bridge_fee = response["fee"]
            .as_str()
            .and_then(|s| s.parse::<u128>().ok())
            .unwrap_or(0);
        state.stats.total_bridge_fees += bridge_fee;
        state.stats.transaction_count += 1;

        Ok(BridgeResult { tx_hash, status })
    }

    /// Post a task to the marketplace on behalf of a user (master pays budget).
    pub async fn sponsor_task(
        &self,
        user_address: &str,
        title: &str,
        description: &str,
        task_type: &str,
        budget_wei: u128,
    ) -> SdkResult<TaskResult> {
        self.enforce_spending_policy(user_address, budget_wei)
            .await?;

        let response: serde_json::Value = self
            .rpc
            .call(
                "tenzro_postTask",
                serde_json::json!([{
                    "title": title,
                    "description": description,
                    "task_type": task_type,
                    "max_price": format!("0x{:x}", budget_wei),
                    "poster": self.master_wallet.address,
                    "on_behalf_of": user_address,
                }]),
            )
            .await?;

        let task_id = response["task_id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let mut state = self.state.write().await;
        state.stats.transaction_count += 1;

        if let Some(policy) = state.policies.get_mut(user_address) {
            policy.daily_spent += budget_wei;
        }

        Ok(TaskResult { task_id })
    }

    // -----------------------------------------------------------------------
    // Master Wallet Info
    // -----------------------------------------------------------------------

    /// Get master wallet balance in wei.
    pub async fn get_master_balance(&self) -> SdkResult<u128> {
        let hex: String = self
            .rpc
            .call(
                "tenzro_getBalance",
                serde_json::json!([self.master_wallet.address]),
            )
            .await?;

        let stripped = hex.strip_prefix("0x").unwrap_or(&hex);
        u128::from_str_radix(stripped, 16)
            .map_err(|e| SdkError::RpcError(format!("Failed to parse balance: {}", e)))
    }

    /// Get aggregated usage statistics for this app instance.
    ///
    /// Statistics are tracked locally across all sponsored operations since
    /// the `AppClient` was created.
    pub async fn get_usage_stats(&self) -> SdkResult<UsageStats> {
        let state = self.state.read().await;
        Ok(state.stats.clone())
    }

    /// Returns the master wallet information.
    pub fn master_wallet(&self) -> &MasterWallet {
        &self.master_wallet
    }

    // -----------------------------------------------------------------------
    // Convenience: Access the full TenzroClient
    // -----------------------------------------------------------------------

    /// Get the underlying `TenzroClient` for advanced operations.
    pub fn client(&self) -> &TenzroClient {
        &self.client
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Enforce the spending policy for a user, if one is set.
    async fn enforce_spending_policy(
        &self,
        user_address: &str,
        amount_wei: u128,
    ) -> SdkResult<()> {
        let state = self.state.read().await;
        if let Some(policy) = state.policies.get(user_address) {
            if amount_wei > policy.per_tx_limit {
                return Err(SdkError::InvalidParameter(format!(
                    "Amount {} exceeds per-transaction limit {} for user {}",
                    amount_wei, policy.per_tx_limit, user_address
                )));
            }
            if policy.daily_spent + amount_wei > policy.daily_limit {
                return Err(SdkError::InvalidParameter(format!(
                    "Amount {} would exceed daily limit {} (already spent {}) for user {}",
                    amount_wei, policy.daily_limit, policy.daily_spent, user_address
                )));
            }
        }
        Ok(())
    }
}

/// Returns a simple ISO-8601-ish timestamp without pulling in the `chrono` crate.
fn chrono_now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}Z", secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_stats_default() {
        let state = AppState::new();
        assert_eq!(state.stats.user_count, 0);
        assert_eq!(state.stats.transaction_count, 0);
        assert_eq!(state.stats.total_gas_spent, 0);
    }

    #[test]
    fn test_chrono_now_iso_format() {
        let ts = chrono_now_iso();
        assert!(ts.ends_with('Z'));
        // Should be a valid integer before Z
        let num_part = &ts[..ts.len() - 1];
        assert!(num_part.parse::<u64>().is_ok());
    }

    #[test]
    fn test_master_wallet_serde() {
        let mw = MasterWallet {
            address: "0xabc123".to_string(),
            public_key: "0xpub456".to_string(),
        };
        let json = serde_json::to_string(&mw).unwrap();
        let deserialized: MasterWallet = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.address, "0xabc123");
    }

    #[test]
    fn test_user_wallet_serde() {
        let uw = UserWallet {
            address: "0xuser1".to_string(),
            label: "alice".to_string(),
            created_at: "1234567890Z".to_string(),
            spending_policy: Some(SpendingPolicy {
                daily_limit: 1000,
                per_tx_limit: 100,
                daily_spent: 50,
            }),
        };
        let json = serde_json::to_string(&uw).unwrap();
        let deserialized: UserWallet = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.label, "alice");
        assert_eq!(
            deserialized.spending_policy.unwrap().daily_spent,
            50
        );
    }
}
