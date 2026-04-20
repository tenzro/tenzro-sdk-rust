//! Tenzro Network Rust SDK
//!
//! The official Rust SDK for building applications on Tenzro Network,
//! an AI-Native, Agentic, Tokenized Settlement Layer blockchain.
//!
//! # Overview
//!
//! This SDK provides a clean, ergonomic API for developers to:
//! - Connect to Tenzro Network nodes
//! - Manage wallets and transactions
//! - Perform AI model inference
//! - Execute settlement and payments
//! - Register and interact with AI agents
//! - Participate in governance
//! - Bridge tokens cross-chain
//! - Manage identity (DIDs, credentials)
//! - Use TEE attestation and ZK proofs
//! - Deploy and interact with smart contracts
//!
//! # Quick Start
//!
//! ```no_run
//! use tenzro_sdk::TenzroClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to testnet
//!     let client = TenzroClient::new("https://rpc.tenzro.network").await?;
//!
//!     // Create wallet
//!     let wallet = client.wallet().create_wallet().await?;
//!     println!("Address: {}", wallet.address);
//!
//!     // Register identity
//!     let identity = client.identity().register_human("Alice").await?;
//!     println!("DID: {}", identity.did);
//!
//!     // List AI models
//!     let models = client.inference().list_models().await?;
//!     println!("{} models available", models.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! # Modules
//!
//! The SDK is organized into domain-specific modules accessible via the
//! [`TenzroClient`]:
//!
//! - [`wallet`] -- Create wallets, check balances, send transactions
//! - [`identity`] -- TDIP DIDs, credentials, usernames, delegation
//! - [`agent`] -- Register agents, spawn, swarms, messaging
//! - [`inference`] -- Model discovery, inference requests
//! - [`token`] -- Create tokens, cross-VM transfers, registry
//! - [`bridge`] -- LayerZero, CCIP, deBridge bridging
//! - [`settlement`] -- Escrow, micropayments, batch settlement
//! - [`payment`] -- MPP, x402 payment protocols
//! - [`governance`] -- Proposals, voting, delegation
//! - [`staking`] -- Stake, unstake, rewards
//! - [`crypto`] -- Sign, verify, encrypt, decrypt, hash
//! - [`tee`] -- TEE attestation, seal/unseal
//! - [`zk`] -- ZK proof generation and verification
//! - [`custody`] -- MPC wallets, keystores, session keys
//! - [`contract`] -- Deploy contracts, ABI encoding
//! - [`app`] -- AppClient for developer-funded patterns

pub mod agent;
pub mod agent_payments;
pub mod ap2;
pub mod app;
pub mod auth;
pub mod bridge;
pub mod canton;
pub mod cct;
pub mod circuit_breaker;
pub mod client;
pub mod compliance;
pub mod config;
pub mod contract;
pub mod crypto;
pub mod custody;
pub mod debridge;
pub mod erc7802;
pub mod erc8004;
pub mod error;
pub mod events;
pub mod governance;
pub mod identity;
pub mod inference;
pub mod marketplace;
pub mod nanopayment;
pub mod nft;
pub mod payment;
pub mod provider;
pub mod rpc;
pub mod settlement;
pub mod skill;
pub mod staking;
pub mod streaming;
pub mod task;
pub mod tee;
pub mod token;
pub mod tool;
pub mod types;
pub mod wallet;
pub mod wormhole;
pub mod zk;

// Re-export main types for convenience
pub use client::{BlockInfo, FaucetResponse, NodeInfo, NodeStatus, TenzroClient};
pub use config::SdkConfig;
pub use error::{SdkError, SdkResult};
pub use rpc::RpcClient;

// Re-export core types
pub use types::{Address, AgentIdentity, AgentTemplate, ModelInfo, TaskInfo, TaskQuote};

// Re-export client modules
pub use app::{
    AgentResult as AppAgentResult, AppClient, BridgeResult as AppBridgeResult, FundResult,
    InferenceResult as AppInferenceResult, MasterWallet, TaskResult as AppTaskResult, TxResult,
    UsageStats, UserWallet,
};
pub use agent::AgentClient;
pub use agent_payments::AgentPaymentClient;
pub use auth::{AuthClient, OnboardingKey, RevokeKeyResponse, ValidateKeyResponse};
pub use ap2::{
    Ap2Client, Ap2MandatePairValidation, Ap2MandateVerification, Ap2ProtocolInfo,
};
pub use bridge::BridgeClient;
pub use canton::CantonClient;
pub use cct::{CctClient, CctPool, CctPoolList};
pub use circuit_breaker::CircuitBreakerClient;
pub use compliance::{ComplianceClient, ComplianceResult, ComplianceRules, FreezeResult};
pub use contract::{CallResult, ContractClient, DeployResult};
pub use crypto::{
    CryptoClient, DecryptResult, DerivedKey, EncryptResult, KeyPair, SharedSecret, SignatureResult,
    VerifyResult,
};
pub use custody::{
    CustodyClient, EncryptedKeystore, KeyShare, MpcWallet, RotationResult, SessionKey,
    SpendingPolicy as CustodySpendingPolicy,
};
pub use debridge::{
    DebridgeChain, DebridgeClient, DebridgeInstructions, DebridgeSwapResult, DebridgeTokenInfo,
    DebridgeTxData,
};
pub use erc7802::Erc7802Client;
pub use erc8004::{Erc8004Agent, Erc8004AgentId, Erc8004Calldata, Erc8004Client};
pub use events::{Event, EventClient, Subscription, WebhookRegistration};
pub use governance::{GovernanceClient, GovernanceProposal, VoteReceipt, VotingPower};
pub use identity::{IdentityClient, IdentityInfo, IdentityType, UsernameResolution};
pub use inference::InferenceClient;
pub use marketplace::MarketplaceClient;
pub use nanopayment::NanopaymentClient;
pub use nft::{CollectionInfo, MintResult, NftClient, NftInfo, NftTransferResult, PointerResult};
pub use payment::{GatewayInfo, PaymentChallenge, PaymentClient, PaymentReceipt, PaymentSession};
pub use provider::{
    ChatMessage, ChatResponse, DownloadProgress, HardwareProfile, ModelEndpoint, ModelLoad,
    ParticipateResponse, ProviderClient, ProviderStats,
};
pub use settlement::{SettlementClient, SettlementRequest, SettleResponse};
pub use skill::SkillClient;
pub use staking::StakingClient;
pub use streaming::{
    Event as StreamEvent, SseConnection, StreamResult, StreamingClient, SubscriptionHandle,
};
pub use task::TaskClient;
pub use tee::{
    AttestationResult, SealedData, TeeClient, TeeInfo, TeeProvider, TeeVerifyResult, UnsealedData,
};
pub use token::{
    TokenBalance, TokenClient, TokenInfo, TokenList, TokenListEntry, TransferResult, WrapResult,
    WTNZO_EVM_ADDRESS,
};
pub use tool::ToolClient;
pub use wallet::{AssetBalance, WalletBalance, WalletClient};
pub use wormhole::{WormholeChainId, WormholeClient, WormholeTransferResult, WormholeVaaId};
pub use zk::{CircuitInfo, ProvingKey, ZkClient, ZkProof, ZkVerifyResult};
