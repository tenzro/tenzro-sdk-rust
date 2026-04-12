//! Agent Template Marketplace example for Tenzro SDK
//!
//! This example demonstrates:
//! - Browsing the decentralized agent template marketplace
//! - Listing templates by type or pricing model
//! - Registering new agent templates
//! - Getting detailed template information

use tenzro_sdk::{TenzroClient, config::SdkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("=== Tenzro SDK Agent Template Marketplace Example ===\n");

    // Connect to testnet
    let config = SdkConfig::testnet();
    let client = TenzroClient::connect(config).await?;
    let marketplace = client.marketplace();

    // ============================================================================
    // 1. Browse all free templates
    // ============================================================================
    println!("1. Browsing free agent templates...");
    let free_templates = marketplace.list_agent_templates(
        Some(true),  // free_only = true
        Some(10),    // limit
        Some(0),     // offset
    ).await?;
    println!("   Found {} free templates\n", free_templates.len());

    for tmpl in &free_templates {
        println!("   Template: {}", tmpl.name);
        println!("     ID: {}", tmpl.template_id);
        println!("     Type: {:?}", tmpl.template_type);
        println!("     Downloads: {}", tmpl.download_count);
        println!("     Rating: {}/100", tmpl.rating);
        println!("     Tags: {:?}", tmpl.tags);
        println!();
    }

    // ============================================================================
    // 2. Browse all templates (including paid)
    // ============================================================================
    println!("2. Browsing all templates...");
    let all_templates = marketplace.list_agent_templates(
        None,     // free_only = false (all)
        Some(20),
        Some(0),
    ).await?;
    println!("   Found {} total templates\n", all_templates.len());

    // ============================================================================
    // 3. Register a free specialist agent template
    // ============================================================================
    println!("3. Registering a free Rust code reviewer template...");
    let rust_reviewer = marketplace.register_agent_template(
        "Rust Code Reviewer",
        "Autonomous agent that reviews Rust code for correctness, performance, memory safety, \
         and adherence to idiomatic Rust patterns. Provides specific, actionable feedback \
         with examples.",
        "specialist",
        "You are an expert Rust code reviewer with deep knowledge of the Rust ecosystem, \
         ownership model, lifetimes, async/await patterns, and best practices. \
         When reviewing code, check for: memory safety, error handling with Result/Option, \
         proper use of iterators, avoiding unnecessary clones, and correct async patterns. \
         Provide specific line-by-line feedback with suggested improvements.",
        vec![
            "rust".to_string(),
            "code-review".to_string(),
            "programming".to_string(),
            "security".to_string(),
        ],
        serde_json::json!({ "type": "free" }),
    ).await?;

    println!("   Template registered!");
    println!("   Template ID: {}", rust_reviewer.template_id);
    println!("   Name: {}", rust_reviewer.name);
    println!("   Type: {:?}", rust_reviewer.template_type);
    println!("   Downloads: {}", rust_reviewer.download_count);
    println!("   Rating: {}/100\n", rust_reviewer.rating);

    // ============================================================================
    // 4. Register an autonomous trading agent template (paid per execution)
    // ============================================================================
    println!("4. Registering a paid trading agent template...");
    let trading_agent = marketplace.register_agent_template(
        "DeFi Trading Strategy Agent",
        "Autonomous agent for analyzing DeFi market data, identifying arbitrage opportunities, \
         and executing token swaps on supported DEXes. Uses real-time price feeds and \
         on-chain analytics.",
        "autonomous",
        "You are an autonomous DeFi trading agent. You have access to real-time price feeds, \
         liquidity pool data, and on-chain analytics. Your goal is to identify and execute \
         profitable trading strategies while managing risk. Always verify: 1) Price impact < 0.5%, \
         2) Gas costs < 10% of expected profit, 3) Slippage tolerance within bounds. \
         Never execute trades larger than the configured max_position_size.",
        vec![
            "defi".to_string(),
            "trading".to_string(),
            "autonomous".to_string(),
            "finance".to_string(),
        ],
        serde_json::json!({
            "type": "per_execution",
            "price": "5000000000000000000"  // 5 TNZO per execution
        }),
    ).await?;

    println!("   Template registered!");
    println!("   Template ID: {}", trading_agent.template_id);
    println!("   Pricing: per_execution\n");

    // ============================================================================
    // 5. Register an orchestrator template (subscription-based)
    // ============================================================================
    println!("5. Registering a subscription-based orchestrator template...");
    let orchestrator = marketplace.register_agent_template(
        "Research Pipeline Orchestrator",
        "Workflow orchestrator that coordinates multiple specialist agents to conduct \
         comprehensive research. Spawns web search agents, data analysis agents, and \
         report generation agents, then synthesizes results into structured reports.",
        "orchestrator",
        "You are a research pipeline orchestrator. When given a research topic, you will: \
         1) Decompose the topic into subtasks, 2) Delegate to specialist agents, \
         3) Collect and validate results, 4) Synthesize findings, 5) Generate a structured report. \
         Coordinate up to 5 parallel agents. Track progress and handle failures gracefully.",
        vec![
            "research".to_string(),
            "orchestration".to_string(),
            "multi-agent".to_string(),
            "workflows".to_string(),
        ],
        serde_json::json!({
            "type": "subscription",
            "monthly_rate": "50000000000000000000"  // 50 TNZO per month
        }),
    ).await?;

    println!("   Template registered!");
    println!("   Template ID: {}", orchestrator.template_id);
    println!("   Pricing: subscription (50 TNZO/month)\n");

    // ============================================================================
    // 6. Register a multi-modal template
    // ============================================================================
    println!("6. Registering a multi-modal document processing template...");
    let doc_agent = marketplace.register_agent_template(
        "Document Intelligence Agent",
        "Multi-modal agent that processes PDFs, images, and spreadsheets. Extracts text, \
         tables, charts, and structured data. Supports OCR, table extraction, and \
         diagram interpretation.",
        "multi_modal",
        "You are a document intelligence agent capable of processing multiple file types. \
         For PDFs: extract text, tables, and identify document structure. \
         For images: perform OCR and describe visual content. \
         For spreadsheets: analyze data patterns and generate summaries. \
         Always return structured JSON output with extracted data.",
        vec![
            "documents".to_string(),
            "ocr".to_string(),
            "multi-modal".to_string(),
            "data-extraction".to_string(),
        ],
        serde_json::json!({
            "type": "per_token",
            "price_per_token": "1000000000000"  // 0.000001 TNZO per token
        }),
    ).await?;

    println!("   Template registered!");
    println!("   Template ID: {}", doc_agent.template_id);
    println!("   Pricing: per_token\n");

    // ============================================================================
    // 7. Get detailed information for a registered template
    // ============================================================================
    println!("7. Fetching detailed template information...");
    match marketplace.get_agent_template(&rust_reviewer.template_id).await {
        Ok(details) => {
            println!("   Template: {}", details.name);
            println!("   Creator: {}", details.creator);
            println!("   Version: {}", details.version);
            println!("   Status: {:?}", details.status);
            println!("   Downloads: {}", details.download_count);
            println!("   Rating: {}/100", details.rating);
            println!("   Tags: {:?}", details.tags);
            println!("   Description: {}", &details.description[..details.description.len().min(80)]);
            println!();
        }
        Err(e) => {
            println!("   Note: Could not fetch template details: {}\n", e);
        }
    }

    // ============================================================================
    // 8. Summary of all registered templates
    // ============================================================================
    println!("8. Summary of all templates registered in this session:");
    println!("   - Rust Code Reviewer (ID: {})", &rust_reviewer.template_id[..8]);
    println!("     Type: Specialist | Pricing: Free");
    println!("   - DeFi Trading Strategy Agent (ID: {})", &trading_agent.template_id[..8]);
    println!("     Type: Autonomous | Pricing: 5 TNZO per execution");
    println!("   - Research Pipeline Orchestrator (ID: {})", &orchestrator.template_id[..8]);
    println!("     Type: Orchestrator | Pricing: 50 TNZO/month subscription");
    println!("   - Document Intelligence Agent (ID: {})", &doc_agent.template_id[..8]);
    println!("     Type: Multi-Modal | Pricing: per-token billing");

    println!("\n=== Agent Template Marketplace Example Complete ===");

    Ok(())
}
