//! Event Subscription SDK for Tenzro Network
//!
//! This module provides event querying and subscription functionality for
//! monitoring on-chain activity. Supports block-range queries, real-time
//! subscriptions, and webhook registration for server-side event delivery.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Event client for querying and subscribing to on-chain events
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let events = client.events();
///
/// // Get recent transfer events
/// let result = events.get_events(None, None, Some("transfer"), None).await?;
/// println!("Found {} events", result.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct EventClient {
    rpc: Arc<RpcClient>,
}

impl EventClient {
    /// Creates a new event client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Queries historical events with optional filters
    ///
    /// Returns events matching the specified criteria. All filter parameters
    /// are optional; omitting all filters returns recent events up to the
    /// node's default limit.
    ///
    /// # Arguments
    ///
    /// * `from_block` - Start block height (inclusive)
    /// * `to_block` - End block height (inclusive)
    /// * `event_type` - Event type filter (e.g., "transfer", "mint", "stake", "governance", "settlement")
    /// * `addresses` - Optional list of addresses to filter by (sender or recipient)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let events = client.events();
    ///
    /// // Get transfer events in a block range
    /// let result = events.get_events(
    ///     Some(1000),
    ///     Some(2000),
    ///     Some("transfer"),
    ///     None,
    /// ).await?;
    ///
    /// for event in &result {
    ///     println!("[{}] {} at block {}", event.event_type, event.tx_hash, event.block_height);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_events(
        &self,
        from_block: Option<u64>,
        to_block: Option<u64>,
        event_type: Option<&str>,
        addresses: Option<&[&str]>,
    ) -> SdkResult<Vec<Event>> {
        let mut params = serde_json::Map::new();

        if let Some(fb) = from_block {
            params.insert("from_block".to_string(), serde_json::json!(fb));
        }
        if let Some(tb) = to_block {
            params.insert("to_block".to_string(), serde_json::json!(tb));
        }
        if let Some(et) = event_type {
            params.insert("event_type".to_string(), serde_json::json!(et));
        }
        if let Some(addrs) = addresses {
            params.insert("addresses".to_string(), serde_json::json!(addrs));
        }

        self.rpc
            .call(
                "tenzro_getEvents",
                serde_json::json!([serde_json::Value::Object(params)]),
            )
            .await
    }

    /// Subscribes to real-time events by type
    ///
    /// Creates a subscription that the node will push events to. Returns
    /// a subscription ID that can be used to receive events via the
    /// node's WebSocket or SSE interface.
    ///
    /// # Arguments
    ///
    /// * `event_types` - List of event types to subscribe to
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let events = client.events();
    /// let sub = events.subscribe_events(&["transfer", "mint"]).await?;
    /// println!("Subscription ID: {}", sub.subscription_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn subscribe_events(
        &self,
        event_types: &[&str],
    ) -> SdkResult<Subscription> {
        self.rpc
            .call(
                "tenzro_subscribeEvents",
                serde_json::json!([{
                    "event_types": event_types,
                }]),
            )
            .await
    }

    /// Registers a webhook for server-side event delivery
    ///
    /// The node will POST event payloads to the specified URL when matching
    /// events occur. If a `secret` is provided, the node signs each payload
    /// with HMAC-SHA256 in the `X-Tenzro-Signature` header.
    ///
    /// # Arguments
    ///
    /// * `url` - Webhook endpoint URL (must be HTTPS in production)
    /// * `event_types` - Optional list of event types to filter (all events if None)
    /// * `secret` - Optional HMAC secret for payload signature verification
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let events = client.events();
    /// let hook = events.register_webhook(
    ///     "https://myapp.example.com/webhooks/tenzro",
    ///     Some(&["transfer", "settlement"]),
    ///     Some("whsec_my_secret_key"),
    /// ).await?;
    /// println!("Webhook ID: {}", hook.webhook_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register_webhook(
        &self,
        url: &str,
        event_types: Option<&[&str]>,
        secret: Option<&str>,
    ) -> SdkResult<WebhookRegistration> {
        self.register_webhook_with_addresses(url, event_types, None, secret)
            .await
    }

    /// Register a webhook with an optional `addresses` filter. When
    /// `addresses` is non-empty, the node only delivers events whose
    /// `addresses` array intersects this list. Use this for per-tenant /
    /// per-user webhook subscriptions.
    ///
    /// `secret` must be at least 16 characters — the node refuses
    /// shorter secrets. Webhook URL must be `https://`.
    pub async fn register_webhook_with_addresses(
        &self,
        url: &str,
        event_types: Option<&[&str]>,
        addresses: Option<&[&str]>,
        secret: Option<&str>,
    ) -> SdkResult<WebhookRegistration> {
        let mut params = serde_json::json!({ "url": url });
        if let Some(types) = event_types {
            params["event_types"] = serde_json::json!(types);
        }
        if let Some(addrs) = addresses {
            params["addresses"] = serde_json::json!(addrs);
        }
        if let Some(s) = secret {
            params["secret"] = serde_json::json!(s);
        }
        self.rpc
            .call("tenzro_registerWebhook", serde_json::json!([params]))
            .await
    }

    /// List every registered webhook. Returns each webhook's id, url,
    /// active flag, event_types/addresses filters, and delivery counters.
    /// Secret hashes are NOT returned — secrets are write-only.
    pub async fn list_webhooks(&self) -> SdkResult<WebhookList> {
        self.rpc
            .call("tenzro_listWebhooks", serde_json::json!([{}]))
            .await
    }

    /// Delete a webhook by id. Returns `-32602` if the id is unknown.
    pub async fn delete_webhook(&self, webhook_id: &str) -> SdkResult<WebhookDeletion> {
        self.rpc
            .call(
                "tenzro_deleteWebhook",
                serde_json::json!([{ "webhook_id": webhook_id }]),
            )
            .await
    }
}

/// An on-chain event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event type (e.g., "transfer", "mint", "stake", "governance", "settlement")
    #[serde(default)]
    pub event_type: String,
    /// Transaction hash that emitted the event
    #[serde(default)]
    pub tx_hash: String,
    /// Block height where the event was emitted
    #[serde(default)]
    pub block_height: u64,
    /// Block timestamp (ISO 8601)
    #[serde(default)]
    pub timestamp: String,
    /// Addresses involved in the event
    #[serde(default)]
    pub addresses: Vec<String>,
    /// Event-specific data
    #[serde(default)]
    pub data: serde_json::Value,
    /// Log index within the transaction
    #[serde(default)]
    pub log_index: u64,
}

/// An event subscription handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// Unique subscription identifier
    #[serde(default)]
    pub subscription_id: String,
    /// Event types being subscribed to
    #[serde(default)]
    pub event_types: Vec<String>,
    /// Operation status
    #[serde(default)]
    pub status: String,
}

/// A registered webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookRegistration {
    /// Unique webhook identifier
    #[serde(default)]
    pub webhook_id: String,
    /// Webhook endpoint URL
    #[serde(default)]
    pub url: String,
    /// Event types the webhook receives
    #[serde(default)]
    pub event_types: Vec<String>,
    /// Address filter (empty = no address restriction)
    #[serde(default)]
    pub addresses: Vec<String>,
    /// Whether the webhook is currently active. Defaults to `true`
    /// on freshly-registered hooks; `list_webhooks` returns the
    /// stored flag.
    #[serde(default = "default_true")]
    pub active: bool,
    /// Cumulative deliveries attempted by the node.
    #[serde(default)]
    pub total_deliveries: u64,
    /// Cumulative deliveries that returned a non-2xx response or
    /// timed out.
    #[serde(default)]
    pub failed_deliveries: u64,
    /// Operation status (set on register/delete; empty on list).
    #[serde(default)]
    pub status: String,
}

fn default_true() -> bool { true }

/// Result of `tenzro_listWebhooks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookList {
    #[serde(default)]
    pub webhooks: Vec<WebhookRegistration>,
    #[serde(default)]
    pub total: usize,
}

/// Result of `tenzro_deleteWebhook`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDeletion {
    #[serde(default)]
    pub webhook_id: String,
    #[serde(default)]
    pub status: String,
}
