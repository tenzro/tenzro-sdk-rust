//! Governance example for Tenzro SDK
//!
//! This example demonstrates:
//! - Creating governance proposals
//! - Voting on proposals
//! - Checking voting power
//! - Listing proposals

use tenzro_sdk::{TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK Governance Example ===\n");

    // Connect to testnet
    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;
    let governance = client.governance();

    // Check voting power
    println!("Checking voting power...");
    let voting_power = governance.get_voting_power("0x0000000000000000000000000000000000000000").await?;
    println!("Your voting power: {} wei\n", voting_power.voting_power);

    // List existing proposals
    println!("Listing all governance proposals...");
    let proposals = governance.list_proposals().await?;
    println!("Found {} proposals\n", proposals.len());

    for proposal in &proposals {
        println!("Proposal: {}", proposal.title);
        println!("  ID: {}", proposal.proposal_id);
        println!("  Description: {}", proposal.description);
        println!("  Status: {}", proposal.status);
        println!("  Votes for: {}", proposal.votes_for);
        println!("  Votes against: {}", proposal.votes_against);
        println!();
    }

    // Create a parameter change proposal
    println!("Creating a parameter change proposal...");
    let proposal = governance.create_proposal(
        "Increase Block Size Limit",
        "This proposal suggests increasing the block size limit from 1MB to 2MB.",
        "parameter_change",
        "0x0000000000000000000000000000000000000000",
    ).await?;

    println!("Proposal created!");
    println!("  Proposal ID: {}\n", proposal.proposal_id);

    // Create a treasury grant proposal
    println!("Creating a treasury grant proposal...");
    let grant_proposal = governance.create_proposal(
        "Community Development Grant",
        "Grant 100,000 TNZO to fund community-driven development initiatives for Q1 2026.",
        "treasury_grant",
        "0x0000000000000000000000000000000000000000",
    ).await?;

    println!("Grant proposal created!");
    println!("  Proposal ID: {}\n", grant_proposal.proposal_id);

    // Vote on the first proposal
    println!("Voting on the parameter change proposal...");
    // Votes are signature-gated: sign "tenzro:vote:{proposal_id}:for" with the
    // voter's key. Placeholder signature/public key shown here — the node
    // rejects votes whose signature does not verify against the voter address.
    let vote_receipt = governance.vote(
        &proposal.proposal_id,
        "0x0000000000000000000000000000000000000000",
        "for",
        "<hex-signature>",
        "<hex-public-key>",
    ).await?;
    println!("Vote cast successfully!");
    println!("  Voting power used: {}", vote_receipt.voting_power);
    println!("  Vote: For");
    println!("  Proposal: {}\n", proposal.proposal_id);

    // Vote on the grant proposal
    println!("Voting on the grant proposal...");
    let grant_vote = governance.vote(
        &grant_proposal.proposal_id,
        "0x0000000000000000000000000000000000000000",
        "abstain",
        "<hex-signature>",
        "<hex-public-key>",
    ).await?;
    println!("Vote cast successfully!");
    println!("  Voting power used: {}", grant_vote.voting_power);
    println!("  Vote: Abstain\n");

    // Get details of a specific proposal
    println!("Fetching proposal details...");
    match governance.get_proposal(&proposal.proposal_id).await {
        Ok(details) => {
            println!("Proposal Details:");
            println!("  Title: {}", details.title);
            println!("  Proposer: {}", details.proposer);
            println!("  Voting ends: {}", details.voting_end);
            println!("  Current status: {}", details.status);
        }
        Err(e) => {
            println!("Note: Proposal details not available yet: {}", e);
        }
    }

    println!("\n=== Example completed successfully! ===");

    Ok(())
}
