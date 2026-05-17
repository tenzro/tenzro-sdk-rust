//! EIP-7702 (Set EOA Account Code) helper client.
//!
//! EIP-7702 lets an externally-owned account temporarily delegate its
//! code to a smart-contract address. The delegation is encoded as a
//! 23-byte designator (`0xef0100 || delegate_address`) written into the
//! EOA's code slot, signed by the EOA's secp256k1 key over a domain-
//! separated preimage.
//!
//! This client wraps the **stateless helper RPCs** the node exposes to
//! support 7702 tooling end-to-end:
//!
//! - `tenzro_eip7702SigningHash` — compute the secp256k1 signing hash
//!   over `MAGIC(0x05) || rlp([chain_id, delegate_address, nonce])`.
//!   The caller signs this hash with the EOA's private key client-side.
//! - `tenzro_eip7702BuildDesignator` — build the 23-byte designator
//!   that goes into the EOA's code slot once the authorization is
//!   accepted.
//! - `tenzro_eip7702ParseDesignator` — decode an account's code and
//!   extract the delegate address if it's a valid 7702 designator;
//!   `is_designator=false` otherwise.
//! - `tenzro_eip7702ProtocolInfo` — static metadata (tx type, magic
//!   byte, designator layout, signing scheme).
//!
//! # Note on transaction submission
//!
//! Full EIP-7702 txtype `0x04` RLP decoding inside
//! `eth_sendRawTransaction` is a separate mainnet task. For now,
//! produce the signing hash with `signing_hash`, sign client-side,
//! and use `build_designator` + a direct state write path to install
//! the delegation.
//!
//! # Example
//!
//! ```no_run
//! # use tenzro_sdk::TenzroClient;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = TenzroClient::new("https://rpc.tenzro.network").await?;
//! let eip7702 = client.eip7702();
//!
//! let info = eip7702.protocol_info().await?;
//! println!("EIP-7702 tx type = {} magic = {}", info.tx_type, info.magic_byte);
//!
//! let hash = eip7702.signing_hash(1337, "0xdeadbeef...", 0).await?;
//! println!("sign this with the EOA secp256k1 key: {}", hash.signing_hash);
//! # Ok(())
//! # }
//! ```

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// EIP-7702 helper client. Stateless — no on-chain writes.
#[derive(Clone)]
pub struct Eip7702Client {
    rpc: Arc<RpcClient>,
}

impl Eip7702Client {
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Compute the secp256k1 signing hash for an EIP-7702 authorization
    /// tuple `(chain_id, delegate_address, nonce)`. The returned
    /// `signing_hash` is what the EOA private key signs client-side.
    pub async fn signing_hash(
        &self,
        chain_id: u64,
        delegate_address: &str,
        nonce: u64,
    ) -> SdkResult<Eip7702SigningHash> {
        self.rpc
            .call(
                "tenzro_eip7702SigningHash",
                serde_json::json!([{
                    "chain_id": chain_id,
                    "delegate_address": delegate_address,
                    "nonce": nonce,
                }]),
            )
            .await
    }

    /// Build the 23-byte EIP-7702 delegation designator
    /// (`0xef0100 || delegate_address`) that gets written into the EOA's
    /// code slot once an authorization is accepted.
    pub async fn build_designator(
        &self,
        delegate_address: &str,
    ) -> SdkResult<Eip7702Designator> {
        self.rpc
            .call(
                "tenzro_eip7702BuildDesignator",
                serde_json::json!([{
                    "delegate_address": delegate_address,
                }]),
            )
            .await
    }

    /// Decode an account's `code` (hex with or without `0x` prefix) and
    /// extract the delegate address if it is a valid EIP-7702
    /// designator. Returns `{ is_designator: false, delegate_address:
    /// null }` for code that is not a 7702 designator.
    pub async fn parse_designator(
        &self,
        code: &str,
    ) -> SdkResult<Eip7702ParsedDesignator> {
        self.rpc
            .call(
                "tenzro_eip7702ParseDesignator",
                serde_json::json!([{ "code": code }]),
            )
            .await
    }

    /// Read static metadata about the EIP-7702 support surface (tx
    /// type, magic byte, designator layout, signing scheme).
    pub async fn protocol_info(&self) -> SdkResult<Eip7702ProtocolInfo> {
        self.rpc
            .call("tenzro_eip7702ProtocolInfo", serde_json::json!([]))
            .await
    }
}

/// Result of `tenzro_eip7702SigningHash`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Eip7702SigningHash {
    /// 32-byte keccak-256 signing hash, 0x-prefixed hex. Sign with the
    /// EOA's secp256k1 private key.
    #[serde(default)]
    pub signing_hash: String,
    /// The full signing preimage (`MAGIC(0x05) || rlp([chain_id,
    /// delegate_address, nonce])`), 0x-prefixed hex. Provided for
    /// auditing / debugging.
    #[serde(default)]
    pub signing_data: String,
    /// Always `"0x05"` — the EIP-7702 magic byte.
    #[serde(default)]
    pub magic_byte: String,
}

/// Result of `tenzro_eip7702BuildDesignator`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Eip7702Designator {
    /// 23-byte designator, 0x-prefixed hex.
    #[serde(default)]
    pub designator: String,
    /// Always 23.
    #[serde(default)]
    pub length: usize,
    /// Always `"0xef0100"`.
    #[serde(default)]
    pub prefix: String,
    /// The delegate address echoed back.
    #[serde(default)]
    pub delegate_address: String,
}

/// Result of `tenzro_eip7702ParseDesignator`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Eip7702ParsedDesignator {
    /// `true` if `code` is a valid 23-byte 7702 designator.
    #[serde(default)]
    pub is_designator: bool,
    /// The delegate address (0x-prefixed hex) if `is_designator=true`,
    /// otherwise `None`.
    #[serde(default)]
    pub delegate_address: Option<String>,
}

/// Result of `tenzro_eip7702ProtocolInfo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Eip7702ProtocolInfo {
    /// EIP-7702 transaction type (`0x04` per the EIP).
    #[serde(default)]
    pub tx_type: u8,
    /// Authorization preimage magic byte (`"0x05"`).
    #[serde(default)]
    pub magic_byte: String,
    /// Designator prefix (`"0xef0100"`).
    #[serde(default)]
    pub designator_prefix: String,
    /// Designator length in bytes (always 23).
    #[serde(default)]
    pub designator_length: usize,
    /// Signing scheme used for the authorization (`"secp256k1"`).
    #[serde(default)]
    pub signing_scheme: String,
    /// Wire format of the secp256k1 signature.
    #[serde(default)]
    pub signature_format: String,
    /// Description of the signing preimage shape.
    #[serde(default)]
    pub preimage: String,
    /// Operator note describing the current state of transaction-side
    /// support (`eth_sendRawTransaction` integration is a follow-up).
    #[serde(default)]
    pub note: String,
}
