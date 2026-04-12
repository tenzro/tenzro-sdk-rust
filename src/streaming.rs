//! Streaming & Real-time SDK for Tenzro Network
//!
//! This module provides streaming inference, real-time event subscriptions,
//! and SSE connections to A2A/MCP servers.

use crate::error::{SdkError, SdkResult};
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Client for streaming and real-time operations
///
/// Provides streaming chat inference, event subscriptions, and SSE connections.
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let streaming = client.streaming();
///
/// // Stream inference tokens
/// let result = streaming.chat_stream("gemma4-9b", "What is Tenzro?", |token| {
///     print!("{}", token);
/// }).await?;
/// println!("\nTotal tokens: {}", result.total_tokens);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct StreamingClient {
    rpc: Arc<RpcClient>,
}

impl StreamingClient {
    /// Creates a new streaming client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Streams inference tokens in real-time
    ///
    /// Calls the model inference endpoint with streaming enabled and invokes
    /// the callback for each token as it arrives.
    ///
    /// # Arguments
    ///
    /// * `model_id` - Model identifier (e.g., "gemma4-9b")
    /// * `message` - User message to send
    /// * `on_token` - Callback invoked for each streamed token
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let streaming = client.streaming();
    /// let result = streaming.chat_stream("gemma4-9b", "Hello!", |token| {
    ///     print!("{}", token);
    /// }).await?;
    /// println!("\nGenerated {} tokens", result.total_tokens);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn chat_stream(
        &self,
        model_id: &str,
        message: &str,
        on_token: impl Fn(&str),
    ) -> SdkResult<StreamResult> {
        let endpoint = self.rpc.endpoint();
        let api_base = if endpoint.contains("rpc.tenzro.network") {
            endpoint.replace("rpc.tenzro.network", "api.tenzro.network")
        } else if endpoint.contains("localhost:8545") || endpoint.contains("127.0.0.1:8545") {
            endpoint.replace("8545", "8080")
        } else {
            endpoint.to_string()
        };

        let url = format!("{}/api/chat", api_base.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": model_id,
            "messages": [{ "role": "user", "content": message }],
            "stream": true,
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| SdkError::ConnectionError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SdkError::RpcError(format!(
                "Streaming request failed: HTTP {}",
                response.status()
            )));
        }

        let mut total_tokens: u32 = 0;
        let mut output = String::new();

        let mut stream = response.bytes_stream();
        use futures::StreamExt;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| SdkError::ConnectionError(e.to_string()))?;
            let text = String::from_utf8_lossy(&chunk);

            // Parse SSE data lines
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                            total_tokens += 1;
                            output.push_str(content);
                            on_token(content);
                        }
                    }
                }
            }
        }

        Ok(StreamResult {
            total_tokens,
            output,
        })
    }

    /// Streams inference tokens via a channel
    ///
    /// Returns an mpsc receiver that yields tokens as they arrive.
    /// Useful when you need to process tokens asynchronously.
    ///
    /// # Arguments
    ///
    /// * `model_id` - Model identifier
    /// * `message` - User message to send
    pub async fn chat_stream_channel(
        &self,
        model_id: &str,
        message: &str,
    ) -> SdkResult<mpsc::Receiver<String>> {
        let (tx, rx) = mpsc::channel::<String>(256);

        let endpoint = self.rpc.endpoint().to_string();
        let api_base = if endpoint.contains("rpc.tenzro.network") {
            endpoint.replace("rpc.tenzro.network", "api.tenzro.network")
        } else if endpoint.contains("localhost:8545") || endpoint.contains("127.0.0.1:8545") {
            endpoint.replace("8545", "8080")
        } else {
            endpoint
        };

        let url = format!("{}/api/chat", api_base.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": model_id,
            "messages": [{ "role": "user", "content": message }],
            "stream": true,
        });

        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let response = match client.post(&url).json(&body).send().await {
                Ok(r) => r,
                Err(_) => return,
            };

            let mut stream = response.bytes_stream();
            use futures::StreamExt;

            while let Some(Ok(chunk)) = stream.next().await {
                let text = String::from_utf8_lossy(&chunk);
                for line in text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            continue;
                        }
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(content) =
                                parsed["choices"][0]["delta"]["content"].as_str()
                            {
                                if tx.send(content.to_string()).await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Subscribes to a stream of events via RPC
    ///
    /// Returns a subscription ID for the event stream. Use this with SSE
    /// or polling to receive events.
    ///
    /// # Arguments
    ///
    /// * `event_types` - Event types to subscribe to (e.g., "block", "transaction")
    pub async fn subscribe_events_stream(
        &self,
        event_types: Vec<String>,
    ) -> SdkResult<SubscriptionHandle> {
        let subscription_id: String = self
            .rpc
            .call(
                "tenzro_subscribeEventsStream",
                serde_json::json!([{ "event_types": event_types }]),
            )
            .await?;

        Ok(SubscriptionHandle {
            id: subscription_id,
        })
    }

    /// Subscribes to real-time events from the node
    ///
    /// Opens a streaming connection and invokes the callback for each event.
    ///
    /// # Arguments
    ///
    /// * `event_types` - Event types to subscribe to (e.g., "block", "transaction", "settlement")
    /// * `on_event` - Callback invoked for each event
    pub async fn subscribe_events(
        &self,
        event_types: Vec<String>,
        on_event: impl Fn(&Event) + Send + 'static,
    ) -> SdkResult<SubscriptionHandle> {
        let subscription_id: String = self
            .rpc
            .call(
                "tenzro_subscribe",
                serde_json::json!([{ "event_types": event_types }]),
            )
            .await?;

        let rpc = self.rpc.clone();
        let id = subscription_id.clone();

        tokio::spawn(async move {
            loop {
                let result: Result<Vec<Event>, _> = rpc
                    .call(
                        "tenzro_pollEvents",
                        serde_json::json!([{ "subscription_id": &id }]),
                    )
                    .await;

                match result {
                    Ok(events) => {
                        for event in &events {
                            on_event(event);
                        }
                    }
                    Err(_) => break,
                }

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        Ok(SubscriptionHandle {
            id: subscription_id,
        })
    }

    /// Opens an SSE connection to an A2A or MCP endpoint
    ///
    /// Returns a handle to the connection that can be used to receive messages.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Full URL of the SSE endpoint
    pub async fn open_sse_connection(&self, endpoint: &str) -> SdkResult<SseConnection> {
        // Validate the endpoint is reachable
        let client = reqwest::Client::new();
        let _response = client
            .get(endpoint)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| SdkError::ConnectionError(e.to_string()))?;

        Ok(SseConnection {
            url: endpoint.to_string(),
        })
    }
}

/// Result from a streaming inference call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResult {
    /// Total number of tokens generated
    pub total_tokens: u32,
    /// Complete generated output
    pub output: String,
}

/// Handle to an active event subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionHandle {
    /// Subscription identifier
    pub id: String,
}

/// SSE connection handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseConnection {
    /// SSE endpoint URL
    pub url: String,
}

/// A real-time event from the node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event type (e.g., "block", "transaction", "settlement")
    #[serde(default, rename = "type")]
    pub event_type: String,
    /// Event data
    #[serde(default)]
    pub data: serde_json::Value,
    /// Event timestamp
    #[serde(default)]
    pub timestamp: String,
}
