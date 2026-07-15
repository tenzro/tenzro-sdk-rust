//! Governance SDK for Tenzro Network
//!
//! This module provides governance and voting functionality.

use crate::error::SdkResult;
use crate::rpc::RpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Governance client for on-chain governance operations
///
/// # Example
///
/// ```no_run
/// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let config = SdkConfig::testnet();
/// # let client = TenzroClient::connect(config).await?;
/// let governance = client.governance();
///
/// // List all proposals
/// let proposals = governance.list_proposals().await?;
/// for proposal in proposals {
///     println!("Proposal: {} - {}", proposal.proposal_id, proposal.title);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct GovernanceClient {
    rpc: Arc<RpcClient>,
}

impl GovernanceClient {
    /// Creates a new governance client
    pub(crate) fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }

    /// Lists all governance proposals
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let governance = client.governance();
    /// let proposals = governance.list_proposals().await?;
    /// println!("Found {} proposals", proposals.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_proposals(&self) -> SdkResult<Vec<GovernanceProposal>> {
        self.rpc
            .call("tenzro_listProposals", serde_json::json!([]))
            .await
    }

    /// Gets details of a specific proposal
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let governance = client.governance();
    /// let proposal = governance.get_proposal("proposal-123").await?;
    /// println!("Title: {}", proposal.title);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_proposal(&self, proposal_id: &str) -> SdkResult<GovernanceProposal> {
        self.rpc
            .call(
                "tenzro_getProposal",
                serde_json::json!({ "proposal_id": proposal_id }),
            )
            .await
    }

    /// Creates a new governance proposal
    ///
    /// # Arguments
    /// * `title` - Proposal title
    /// * `description` - Detailed description
    /// * `proposal_type` - One of `"parameter_change"`, `"treasury_grant"`,
    ///   `"protocol_upgrade"`, or `"text"`. `protocol_upgrade` proposals must
    ///   be created via [`create_proposal_with`](Self::create_proposal_with)
    ///   because the node requires a `code_hash` field for them.
    /// * `proposer` - Proposer address (0x-hex)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let governance = client.governance();
    /// let proposal = governance.create_proposal(
    ///     "Increase block size",
    ///     "This proposal aims to increase the maximum block size to 30MB",
    ///     "text",
    ///     "0x1234...",
    /// ).await?;
    /// println!("Created proposal: {}", proposal.proposal_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_proposal(
        &self,
        title: &str,
        description: &str,
        proposal_type: &str,
        proposer: &str,
    ) -> SdkResult<GovernanceProposal> {
        self.create_proposal_with(
            title,
            description,
            proposal_type,
            proposer,
            serde_json::json!({}),
        )
        .await
    }

    /// Creates a proposal with extra type-specific fields merged into the
    /// request: `parameter`/`new_value` for `parameter_change`,
    /// `grant_amount` for `treasury_grant`, `version`/`code_hash`
    /// (32-byte SHA-256 hex) for `protocol_upgrade`.
    pub async fn create_proposal_with(
        &self,
        title: &str,
        description: &str,
        proposal_type: &str,
        proposer: &str,
        extra: serde_json::Value,
    ) -> SdkResult<GovernanceProposal> {
        let mut params = serde_json::json!({
            "title": title,
            "description": description,
            "proposal_type": proposal_type,
            "proposer": proposer,
        });
        if let (Some(obj), Some(extra_obj)) = (params.as_object_mut(), extra.as_object()) {
            for (k, v) in extra_obj {
                obj.insert(k.clone(), v.clone());
            }
        }
        self.rpc.call("tenzro_createProposal", params).await
    }

    /// Casts a vote on a proposal
    ///
    /// The node signature-gates this RPC: the vote must be signed with the
    /// key that derives to `voter`. Sign the domain-separated message
    /// `tenzro:vote:{proposal_id}:{vote_type}` (using `vote_type` exactly as
    /// passed here) with the voter's Ed25519 or Secp256k1 key and pass the
    /// signature and public key as hex. Voting power is read from the
    /// voter's stake.
    ///
    /// # Arguments
    /// * `proposal_id` - The ID of the proposal to vote on
    /// * `voter` - Voter address (0x-hex)
    /// * `vote_type` - The vote type: "for", "against", or "abstain"
    /// * `signature` - Hex signature over `tenzro:vote:{proposal_id}:{vote_type}`
    /// * `public_key` - Hex public key that derives to `voter`
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let governance = client.governance();
    /// let receipt = governance
    ///     .vote("proposal-123", "0x1234...", "for", "a1b2...", "c3d4...")
    ///     .await?;
    /// println!("Voting power used: {}", receipt.voting_power);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn vote(
        &self,
        proposal_id: &str,
        voter: &str,
        vote_type: &str,
        signature: &str,
        public_key: &str,
    ) -> SdkResult<VoteReceipt> {
        self.rpc
            .call(
                "tenzro_vote",
                serde_json::json!({
                    "proposal_id": proposal_id,
                    "voter": voter,
                    "vote": vote_type,
                    "signature": signature,
                    "public_key": public_key,
                }),
            )
            .await
    }

    /// Casts a vote on a proposal (alias for [`vote`](Self::vote))
    ///
    /// This is a convenience alias matching the `tenzro_voteOnProposal` RPC
    /// method; the node dispatches both methods to the same handler. See
    /// [`vote`](Self::vote) for the signature requirements.
    pub async fn vote_on_proposal(
        &self,
        proposal_id: &str,
        voter: &str,
        vote: &str,
        signature: &str,
        public_key: &str,
    ) -> SdkResult<VoteReceipt> {
        self.rpc
            .call(
                "tenzro_voteOnProposal",
                serde_json::json!({
                    "proposal_id": proposal_id,
                    "voter": voter,
                    "vote": vote,
                    "signature": signature,
                    "public_key": public_key,
                }),
            )
            .await
    }

    /// Gets the voting power for an address
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let governance = client.governance();
    /// let power = governance.get_voting_power("0x1234...").await?;
    /// println!("Voting power: {} wei", power.voting_power);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_voting_power(&self, address: &str) -> SdkResult<VotingPower> {
        self.rpc
            .call(
                "tenzro_getVotingPower",
                serde_json::json!({ "address": address }),
            )
            .await
    }

    /// Delegates voting power to another address
    ///
    /// # Arguments
    /// * `delegator` - Address delegating its voting power (0x-hex)
    /// * `delegatee` - Address receiving the voting power (0x-hex)
    /// * `amount` - Amount to delegate (wei)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tenzro_sdk::{TenzroClient, config::SdkConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = SdkConfig::testnet();
    /// # let client = TenzroClient::connect(config).await?;
    /// let governance = client.governance();
    /// let result = governance.delegate("0x1234...", "0x5678...", 1000000).await?;
    /// println!("Delegation status: {}", result.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delegate(
        &self,
        delegator: &str,
        delegatee: &str,
        amount: u64,
    ) -> SdkResult<DelegationResult> {
        self.rpc
            .call(
                "tenzro_delegateVotingPower",
                serde_json::json!({
                    "delegator": delegator,
                    "delegatee": delegatee,
                    "amount": amount,
                }),
            )
            .await
    }
}

/// Governance proposal information
///
/// Mirrors the node's `tenzro_types::token::GovernanceProposal` wire shape.
/// `proposer` and `proposal_type` are kept as raw JSON because the node
/// serializes addresses as 32-byte arrays (or 0x-hex in the
/// `tenzro_createProposal` response) and proposal types as externally
/// tagged enums (e.g. `{"ParameterChange": {...}}`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceProposal {
    /// Unique proposal ID
    #[serde(default)]
    pub proposal_id: String,
    /// Proposal title
    #[serde(default)]
    pub title: String,
    /// Detailed description
    #[serde(default)]
    pub description: String,
    /// Proposal type — externally tagged enum object, e.g.
    /// `{"ParameterChange": {"parameter": "...", "new_value": "..."}}`
    #[serde(default)]
    pub proposal_type: serde_json::Value,
    /// Proposer address (32-byte array or 0x-hex string depending on RPC)
    #[serde(default)]
    pub proposer: serde_json::Value,
    /// Current status (e.g., "Active", "Passed", "Rejected")
    #[serde(default)]
    pub status: String,
    /// Votes in favor
    #[serde(default)]
    pub votes_for: u128,
    /// Votes against
    #[serde(default)]
    pub votes_against: u128,
    /// Total voting power at snapshot
    #[serde(default)]
    pub total_voting_power: u128,
    /// Voting start time (Unix millis)
    #[serde(default)]
    pub voting_start: i64,
    /// Voting end time (Unix millis)
    #[serde(default)]
    pub voting_end: i64,
    /// Execution data (if applicable)
    #[serde(default)]
    pub execution_data: Option<Vec<u8>>,
}

/// Vote receipt returned by `tenzro_vote`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteReceipt {
    /// Whether the vote was recorded
    #[serde(default)]
    pub success: bool,
    /// Proposal ID
    #[serde(default)]
    pub proposal_id: String,
    /// Voter address (0x-hex)
    #[serde(default)]
    pub voter: String,
    /// Voting power used (decimal string, wei)
    #[serde(default)]
    pub voting_power: String,
}

/// Voting power information returned by `tenzro_getVotingPower`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingPower {
    /// Address (0x-hex)
    #[serde(default)]
    pub address: String,
    /// Voting power from staked TNZO (decimal string, wei)
    #[serde(default)]
    pub voting_power: String,
}

/// Result of `tenzro_delegateVotingPower`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResult {
    /// Delegator address (0x-hex)
    #[serde(default)]
    pub delegator: String,
    /// Delegatee address (0x-hex)
    #[serde(default)]
    pub delegatee: String,
    /// Amount delegated (wei)
    #[serde(default)]
    pub amount: u64,
    /// Status (e.g., "delegated")
    #[serde(default)]
    pub status: String,
}
