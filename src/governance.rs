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
            .call("tenzro_getProposal", serde_json::json!([proposal_id]))
            .await
    }

    /// Creates a new governance proposal
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
    ///     "parameter_change",
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
    ) -> SdkResult<GovernanceProposal> {
        self.rpc
            .call(
                "tenzro_createProposal",
                serde_json::json!([{
                    "title": title,
                    "description": description,
                    "proposal_type": proposal_type,
                }]),
            )
            .await
    }

    /// Casts a vote on a proposal
    ///
    /// # Arguments
    /// * `proposal_id` - The ID of the proposal to vote on
    /// * `vote_type` - The vote type: "for", "against", or "abstain"
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
    /// let receipt = governance.vote("proposal-123", "for").await?;
    /// println!("Vote cast with ID: {}", receipt.vote_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn vote(&self, proposal_id: &str, vote_type: &str) -> SdkResult<VoteReceipt> {
        self.rpc
            .call(
                "tenzro_vote",
                serde_json::json!([proposal_id, vote_type]),
            )
            .await
    }

    /// Casts a vote on a proposal (alias for `vote`)
    ///
    /// This is a convenience alias matching the `tenzro_voteOnProposal` RPC method.
    ///
    /// # Arguments
    ///
    /// * `proposal_id` - The ID of the proposal to vote on
    /// * `vote` - The vote: "for", "against", or "abstain"
    pub async fn vote_on_proposal(
        &self,
        proposal_id: &str,
        vote: &str,
    ) -> SdkResult<VoteReceipt> {
        self.rpc
            .call(
                "tenzro_voteOnProposal",
                serde_json::json!([proposal_id, vote]),
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
    /// println!("Voting power: {} TNZO", power.total_power);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_voting_power(&self, address: &str) -> SdkResult<VotingPower> {
        self.rpc
            .call(
                "tenzro_getVotingPower",
                serde_json::json!([address]),
            )
            .await
    }

    /// Delegates voting power to another address
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
    /// let tx_hash = governance.delegate("0x5678...", 1000000).await?;
    /// println!("Delegation tx: {}", tx_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delegate(&self, delegate: &str, amount: u128) -> SdkResult<String> {
        self.rpc
            .call(
                "tenzro_delegateVotingPower",
                serde_json::json!([delegate, amount]),
            )
            .await
    }
}

/// Governance proposal information
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
    /// Proposal type (e.g., "parameter_change", "upgrade", "treasury")
    #[serde(default)]
    pub proposal_type: String,
    /// Proposer address
    #[serde(default)]
    pub proposer: String,
    /// Current status (e.g., "pending", "active", "passed", "rejected")
    #[serde(default)]
    pub status: String,
    /// Votes in favor
    #[serde(default)]
    pub votes_for: u128,
    /// Votes against
    #[serde(default)]
    pub votes_against: u128,
    /// Abstain votes
    #[serde(default)]
    pub votes_abstain: u128,
    /// Proposal creation time (Unix timestamp)
    #[serde(default)]
    pub created_at: u64,
    /// Voting end time (Unix timestamp)
    #[serde(default)]
    pub voting_end: u64,
}

/// Vote receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteReceipt {
    /// Vote ID
    #[serde(default)]
    pub vote_id: String,
    /// Proposal ID
    #[serde(default)]
    pub proposal_id: String,
    /// Vote type ("for", "against", "abstain")
    #[serde(default)]
    pub vote_type: String,
    /// Voting power used
    #[serde(default)]
    pub voting_power: u128,
    /// Transaction hash
    #[serde(default)]
    pub tx_hash: String,
}

/// Voting power information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingPower {
    /// Address
    #[serde(default)]
    pub address: String,
    /// Total voting power
    #[serde(default)]
    pub total_power: u128,
    /// Voting power from staked TNZO
    #[serde(default)]
    pub staked_power: u128,
    /// Delegated voting power received
    #[serde(default)]
    pub delegated_power: u128,
    /// Voting power delegated to others
    #[serde(default)]
    pub delegated_out: u128,
}
