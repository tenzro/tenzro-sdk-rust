//! Settlement SDK for Tenzro Network
//!
//! This module provides payment settlement and escrow functionality.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Settlement client for payment operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let settlement = client.settlement();
///
/// // Get settlement by receipt ID
/// let settlement = settlement.get_settlement("receipt-123").await?;
/// println!("Settlement: {:?}", settlement);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct SettlementClient {
    rpc: Arc<RpcClient>,
}

impl SettlementClient {
    /// Creates a new settlement client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Submits a settlement request
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig, SettlementRequest};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let settlement_client = client.settlement();
    ///
    /// let request = SettlementRequest {
    ///     request_id: "req-123".to_string(),
    ///     provider: "0xprovider...".to_string(),
    ///     customer: "0xcustomer...".to_string(),
    ///     amount: 1000000,
    ///     asset: "TNZO".to_string(),
    /// };
    ///
    /// let response = settlement_client.settle(request).await?;
    /// println!("Settlement receipt: {}", response.receipt_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn settle(&self, request: SettlementRequest) -> SdkResult<SettleResponse> {
        self.rpc
            .call(
                "tenzro_settle",
                serde_json::json!([{
                    "request_id": request.request_id,
                    "provider": request.provider,
                    "customer": request.customer,
                    "amount": request.amount,
                    "asset": request.asset,
                }]),
            )
            .await
    }

    /// Gets a settlement by receipt ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let settlement = client.settlement();
    /// let result = settlement.get_settlement("receipt-123").await?;
    /// println!("Settlement: {:?}", result);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_settlement(&self, receipt_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_getSettlement",
                serde_json::json!([receipt_id]),
            )
            .await
    }

    /// Creates an on-chain escrow via a signed `CreateEscrow` transaction.
    ///
    /// The escrow_id is derived deterministically by the VM as
    /// `SHA-256("tenzro/escrow/id/v1" || payer || nonce_le)` and the funds are
    /// transferred to a vault address derived from that escrow_id. Only the
    /// signing payer can later release or refund.
    ///
    /// **Auth model.** This method calls `tenzro_signAndSendTransaction`
    /// without a private key in the payload. The node resolves the
    /// signing wallet from the bearer JWT (DPoP-bound, RFC 9449) carried
    /// on the request via the ambient `Authorization: DPoP <jwt>` +
    /// `DPoP: <proof>` headers. Make sure `TENZRO_BEARER_JWT` and
    /// `TENZRO_DPOP_PROOF` are set in the environment before calling —
    /// see [`crate::auth::AuthClient`] for how to mint them.
    ///
    /// Returns the transaction hash; the resulting escrow_id can be
    /// inspected via [`Self::get_escrow`] once the transaction finalizes
    /// (the VM emits the derived id in the receipt log).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = TenzroClient::connect(SdkConfig::testnet()).await?;
    /// let settlement = client.settlement();
    /// let tx_hash = settlement.create_escrow(
    ///     "0xpayer...",
    ///     "0xpayee...",
    ///     1_000_000_000_000_000_000u128, // 1 TNZO in wei
    ///     "TNZO",
    ///     1_800_000_000_000u64,          // expires_at (unix ms)
    ///     "both_signatures",
    /// ).await?;
    /// println!("Escrow create tx: {}", tx_hash);
    /// # Ok(()) }
    /// ```
    pub async fn create_escrow(
        &self,
        payer: &str,
        payee: &str,
        amount: u128,
        asset: &str,
        expires_at: u64,
        release_conditions: &str,
    ) -> SdkResult<String> {
        let release_conditions_json = match release_conditions.to_lowercase().as_str() {
            "timeout" => serde_json::json!({ "type": "Timeout" }),
            "provider_signature" | "provider" => serde_json::json!({ "type": "ProviderSignature" }),
            "consumer_signature" | "consumer" => serde_json::json!({ "type": "ConsumerSignature" }),
            "both_signatures" | "both" => serde_json::json!({ "type": "BothSignatures" }),
            "verifier_signature" | "verifier" => serde_json::json!({ "type": "VerifierSignature" }),
            "custom" => serde_json::json!({ "type": "Custom", "data": "" }),
            other => {
                return Err(crate::error::SdkError::InvalidParameter(format!(
                    "unsupported release condition '{}': use timeout|provider|consumer|both|verifier|custom",
                    other
                )))
            }
        };

        let (nonce, chain_id) = self.fetch_nonce_and_chain_id(payer).await;
        let tx_type = serde_json::json!({
            "type": "CreateEscrow",
            "data": {
                "payee": payee,
                "amount": amount.to_string(),
                "asset_id": asset,
                "expires_at": expires_at,
                "release_conditions": release_conditions_json,
            }
        });

        let result: serde_json::Value = self
            .rpc
            .call(
                "tenzro_signAndSendTransaction",
                serde_json::json!({
                    "from": payer,
                    "to": payee,
                    "value": 0u64,
                    "gas_limit": 75_000u64,
                    "gas_price": 1_000_000_000u64,
                    "nonce": nonce,
                    "chain_id": chain_id,
                    "tx_type": tx_type,
                }),
            )
            .await?;

        Ok(result
            .get("tx_hash")
            .or_else(|| result.get("transaction_hash"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| result.as_str().map(|s| s.to_string()))
            .unwrap_or_default())
    }

    /// Releases an escrow to the payee via a signed `ReleaseEscrow`
    /// transaction. Authentication is via the ambient bearer JWT — see
    /// [`Self::create_escrow`] for the full auth model. The bearer DID
    /// MUST resolve to the original payer's wallet; the VM rejects
    /// releases initiated by any other address.
    pub async fn release_escrow(
        &self,
        payer: &str,
        escrow_id: [u8; 32],
        proof: Vec<u8>,
    ) -> SdkResult<String> {
        let (nonce, chain_id) = self.fetch_nonce_and_chain_id(payer).await;
        let tx_type = serde_json::json!({
            "type": "ReleaseEscrow",
            "data": {
                "escrow_id": escrow_id.to_vec(),
                "proof": {
                    "proof_type": "Timeout",
                    "proof_data": proof,
                    "signatures": []
                }
            }
        });

        let result: serde_json::Value = self
            .rpc
            .call(
                "tenzro_signAndSendTransaction",
                serde_json::json!({
                    "from": payer,
                    "to": "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "value": 0u64,
                    "gas_limit": 60_000u64,
                    "gas_price": 1_000_000_000u64,
                    "nonce": nonce,
                    "chain_id": chain_id,
                    "tx_type": tx_type,
                }),
            )
            .await?;

        Ok(result
            .get("tx_hash")
            .or_else(|| result.get("transaction_hash"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| result.as_str().map(|s| s.to_string()))
            .unwrap_or_default())
    }

    /// Refunds an escrow back to the payer via a signed `RefundEscrow`
    /// transaction. Same auth model as [`Self::create_escrow`]. The
    /// escrow must be expired (or use `Timeout` / `Custom` release
    /// conditions); refunding before expiry on a non-`Timeout` escrow is
    /// rejected with `EscrowNotExpired`.
    pub async fn refund_escrow(
        &self,
        payer: &str,
        escrow_id: [u8; 32],
    ) -> SdkResult<String> {
        let (nonce, chain_id) = self.fetch_nonce_and_chain_id(payer).await;
        let tx_type = serde_json::json!({
            "type": "RefundEscrow",
            "data": { "escrow_id": escrow_id.to_vec() }
        });

        let result: serde_json::Value = self
            .rpc
            .call(
                "tenzro_signAndSendTransaction",
                serde_json::json!({
                    "from": payer,
                    "to": "0x0000000000000000000000000000000000000000000000000000000000000000",
                    "value": 0u64,
                    "gas_limit": 50_000u64,
                    "gas_price": 1_000_000_000u64,
                    "nonce": nonce,
                    "chain_id": chain_id,
                    "tx_type": tx_type,
                }),
            )
            .await?;

        Ok(result
            .get("tx_hash")
            .or_else(|| result.get("transaction_hash"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| result.as_str().map(|s| s.to_string()))
            .unwrap_or_default())
    }

    /// Inspects an escrow record by its 32-byte escrow_id.
    pub async fn get_escrow(&self, escrow_id: [u8; 32]) -> SdkResult<serde_json::Value> {
        let escrow_id_hex = format!("0x{}", hex::encode(escrow_id));
        self.rpc
            .call(
                "tenzro_getEscrow",
                serde_json::json!({ "escrow_id": escrow_id_hex }),
            )
            .await
    }

    /// Internal helper: fetch nonce + chain_id with safe defaults if the RPC is unavailable.
    async fn fetch_nonce_and_chain_id(&self, address: &str) -> (u64, u64) {
        let nonce = self
            .rpc
            .call::<serde_json::Value>(
                "eth_getTransactionCount",
                serde_json::json!([address, "latest"]),
            )
            .await
            .ok()
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            })
            .unwrap_or(0);
        let chain_id = self
            .rpc
            .call::<serde_json::Value>("eth_chainId", serde_json::json!([]))
            .await
            .ok()
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            })
            .unwrap_or(1337);
        (nonce, chain_id)
    }

    /// Opens a micropayment channel
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let settlement = client.settlement();
    ///
    /// let channel_id = settlement.open_payment_channel(
    ///     "0xpayee...",
    ///     10000000,
    /// ).await?;
    /// println!("Channel opened: {}", channel_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open_payment_channel(
        &self,
        payee: &str,
        deposit: u64,
    ) -> SdkResult<String> {
        self.rpc
            .call(
                "tenzro_openPaymentChannel",
                serde_json::json!([{
                    "payee": payee,
                    "deposit": deposit,
                }]),
            )
            .await
    }

    /// Closes a micropayment channel
    pub async fn close_payment_channel(&self, channel_id: &str) -> SdkResult<serde_json::Value> {
        self.rpc
            .call(
                "tenzro_closePaymentChannel",
                serde_json::json!({"channel_id": channel_id}),
            )
            .await
    }
}

/// Settlement request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRequest {
    /// Unique request ID
    pub request_id: String,
    /// Provider address
    pub provider: String,
    /// Customer address
    pub customer: String,
    /// Settlement amount
    pub amount: u64,
    /// Asset symbol (e.g., "TNZO", "USDC")
    pub asset: String,
}

/// Settlement response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettleResponse {
    /// Receipt ID
    #[serde(default)]
    pub receipt_id: String,
    /// Transaction hash
    #[serde(default)]
    pub tx_hash: String,
    /// Settlement status
    #[serde(default)]
    pub status: String,
}
