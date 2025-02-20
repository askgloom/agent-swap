use agent_swap::{
    agent::SwapAgent,
    swap::{DexType, SwapEngine},
    Config, Result,
    utils::{setup_wallet, parse_amount},
};

use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
};
use std::str::FromStr;
use std::time::Instant;

// Token addresses (mainnet)
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const SOL: &str = "So11111111111111111111111111111111111111112";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    println!("Starting simple swap example...");

    // Load configuration
    let config = Config {
        rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
        max_slippage_bps: 100, // 1%
        use_ai_optimization: true,
        ..Config::default()
    };

    // Initialize agent and engine
    println!("Initializing agent and swap engine...");
    let (agent, engine) = agent_swap::init(config).await?;

    // Load or create wallet
    let wallet = setup_wallet(None)?;
    println!("Using wallet: {}", wallet.pubkey());

    // Set up swap parameters
    let usdc = Pubkey::from_str(USDC)?;
    let sol = Pubkey::from_str(SOL)?;
    let amount = parse_amount("100", 6)?; // 100 USDC

    println!("\nSwap Parameters:");
    println!("From: USDC ({})", usdc);
    println!("To: SOL ({})", sol);
    println!("Amount: 100 USDC");

    // Get best quote
    println!("\nGetting best quote...");
    let start = Instant::now();
    let quote = engine.get_best_quote(&usdc, &sol, amount).await?;
    
    println!("Quote received in {:?}", start.elapsed());
    println!("Best route found on {:?}", quote.dex_type);
    println!("Expected output: {} SOL", quote.amount_out as f64 / 1e9);
    println!("Price impact: {}%", quote.price_impact_bps as f64 / 100.0);
    println!("Minimum output: {} SOL", quote.minimum_out as f64 / 1e9);

    // Let agent evaluate the route
    println!("\nEvaluating route with AI agent...");
    let confidence = agent.evaluate_route(&quote).await?;
    
    println!("Agent confidence: {:.2}%", confidence.score * 100.0);
    println!("Reasoning: {}", confidence.reasoning);

    if confidence.score >= 0.8 {
        println!("\nExecuting swap...");
        let start = Instant::now();
        
        // Execute the swap
        match engine.execute_swap(&quote, &wallet).await {
            Ok(signature) => {
                println!("Swap successful!");
                println!("Transaction signature: {}", signature);
                println!("Execution time: {:?}", start.elapsed());

                // Record successful swap
                agent.record_success(&quote).await?;
                
                // Print updated metrics
                let metrics = agent.get_metrics();
                println!("\nUpdated metrics:");
                for (key, value) in metrics {
                    println!("{}: {:.2}", key, value);
                }
            }
            Err(e) => {
                println!("Swap failed: {}", e);
                agent.record_failure(&quote).await?;
            }
        }
    } else {
        println!("\nAgent rejected the route due to low confidence score");
    }

    // Example of error handling
    println!("\nTesting error handling...");
    let result = engine
        .get_best_quote(&Pubkey::new_unique(), &Pubkey::new_unique(), 1)
        .await;
    
    match result {
        Ok(_) => println!("Unexpected success with invalid tokens"),
        Err(e) => println!("Expected error received: {}", e),
    }

    println!("\nExample completed!");
    Ok(())
}