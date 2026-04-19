//! Task Marketplace example for Tenzro SDK
//!
//! This example demonstrates:
//! - Posting tasks to the decentralized AI task marketplace
//! - Listing open tasks with filters
//! - Getting task details and status
//! - Submitting quotes as a model provider
//! - Cancelling tasks

use tenzro_sdk::{TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK Task Marketplace Example ===\n");

    // Connect to testnet
    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;
    let task = client.task();

    // ============================================================================
    // 1. List open tasks
    // ============================================================================
    println!("1. Listing open tasks...");
    let open_tasks = task.list_tasks(Some("open"), None, None, Some(10), Some(0)).await?;
    println!("   Found {} open tasks\n", open_tasks.len());

    for t in &open_tasks {
        println!("   Task: {}", t.title);
        println!("     ID: {}", t.task_id);
        println!("     Type: {:?}", t.task_type);
        println!("     Max price: {} TNZO (wei)", t.max_price);
        println!("     Priority: {:?}", t.priority);
        println!("     Status: {:?}", t.status);
        println!();
    }

    // ============================================================================
    // 2. List tasks filtered by type
    // ============================================================================
    println!("2. Listing code_review tasks...");
    let review_tasks = task.list_tasks(
        Some("open"),
        Some("code_review"),
        None,
        Some(5),
        Some(0),
    ).await?;
    println!("   Found {} code review tasks\n", review_tasks.len());

    // ============================================================================
    // 3. Post a new task to the marketplace
    // ============================================================================
    println!("3. Posting a new task to the marketplace...");
    let posted = task.post_task(
        "Code Review: Rust async refactor",
        "Review the attached Rust module and suggest improvements for async patterns, \
         error handling, and idiomatic Rust usage.",
        "code_review",
        50_000_000_000_000_000_000u128, // 50 TNZO max price (in wei)
        "fn main() { tokio::runtime::Runtime::new().unwrap().block_on(async { ... }); }",
    ).await?;

    println!("   Task posted!");
    println!("   Task ID: {}", posted.task_id);
    println!("   Title: {}", posted.title);
    println!("   Status: {:?}\n", posted.status);

    // ============================================================================
    // 4. Get task details
    // ============================================================================
    println!("4. Getting task details...");
    match task.get_task(&posted.task_id).await {
        Ok(details) => {
            println!("   Task: {}", details.title);
            println!("   Poster: {}", details.poster);
            println!("   Status: {:?}", details.status);
            println!("   Max price: {} wei", details.max_price);
            println!("   Created at: {}", details.created_at);
            println!();
        }
        Err(e) => {
            println!("   Note: Could not fetch task details: {}\n", e);
        }
    }

    // ============================================================================
    // 5. Submit a quote (as a model provider)
    // ============================================================================
    println!("5. Submitting a quote for the task...");
    match task.submit_quote(
        &posted.task_id,
        45_000_000_000_000_000_000u128, // 45 TNZO quote price
        "gemma3-270m",                   // model to use
        120,                             // estimated 120 seconds
        90,                              // 90% confidence
        Some("Can complete this within 2 minutes using Gemma 3 270M model. \
              Will provide detailed line-by-line review.".to_string()),
    ).await {
        Ok(quote) => {
            println!("   Quote submitted!");
            println!("   Price: {} wei", quote.price);
            println!("   Model: {}", quote.model_id);
            println!("   ETA: {}s", quote.estimated_duration_secs);
            println!("   Confidence: {}%\n", quote.confidence);
        }
        Err(e) => {
            println!("   Note: Could not submit quote: {}\n", e);
        }
    }

    // ============================================================================
    // 6. Post an inference task
    // ============================================================================
    println!("6. Posting an AI inference task...");
    let inference_task = task.post_task(
        "Summarize research paper",
        "Summarize the following abstract in 3 bullet points for a non-technical audience.",
        "inference",
        10_000_000_000_000_000_000u128, // 10 TNZO max price
        "Abstract: We present a novel approach to zero-knowledge proofs...",
    ).await?;

    println!("   Inference task posted: {}\n", inference_task.task_id);

    // ============================================================================
    // 7. Post a data analysis task
    // ============================================================================
    println!("7. Posting a data analysis task...");
    let analysis_task = task.post_task(
        "Analyze token distribution CSV",
        "Analyze the token distribution data and identify top 10 holders by percentage.",
        "data_analysis",
        25_000_000_000_000_000_000u128, // 25 TNZO max price
        "wallet_address,balance\n0xabc...,1000000\n0xdef...,500000\n...",
    ).await?;

    println!("   Analysis task posted: {}\n", analysis_task.task_id);

    // ============================================================================
    // 8. Cancel one of the tasks
    // ============================================================================
    println!("8. Cancelling the inference task...");
    match task.cancel_task(&inference_task.task_id).await {
        Ok(()) => println!("   Task {} cancelled successfully\n", inference_task.task_id),
        Err(e) => println!("   Note: Could not cancel task: {}\n", e),
    }

    // ============================================================================
    // 9. List all tasks to see current state
    // ============================================================================
    println!("9. Final task listing (all statuses)...");
    let all_tasks = task.list_tasks(None, None, None, Some(20), Some(0)).await?;
    println!("   Total tasks: {}", all_tasks.len());
    for t in &all_tasks {
        println!("   [{}] {} — {:?}", t.task_id, t.title, t.status);
    }

    println!("\n=== Task Marketplace Example Complete ===");

    Ok(())
}
