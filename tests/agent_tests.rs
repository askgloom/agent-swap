use agent_swap::{
    agent::{Memory, SwapAgent},
    swap::{DexType, Quote, SwapEngine},
    Config, Result,
};

use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    transaction::Transaction,
};
use tokio;

// Test constants
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const SOL: &str = "So11111111111111111111111111111111111111112";

/// Helper function to create a test quote
fn create_test_quote(amount_in: u64, amount_out: u64) -> Quote {
    Quote {
        dex_type: DexType::Raydium,
        amount_in,
        amount_out,
        price_impact_bps: 50,
        minimum_out: amount_out * 99 / 100,
        transaction: Transaction::default(),
    }
}

/// Helper function to setup test environment
async fn setup_test_env() -> Result<(SwapAgent, SwapEngine)> {
    let config = Config {
        rpc_url: "https://api.devnet.solana.com".to_string(),
        use_ai_optimization: true,
        ..Config::default()
    };

    agent_swap::init(config).await
}

#[tokio::test]
async fn test_agent_initialization() {
    let (agent, _) = setup_test_env().await.unwrap();
    assert!(agent.get_metrics().is_empty());
}

#[tokio::test]
async fn test_route_evaluation() {
    let (agent, _) = setup_test_env().await.unwrap();
    
    let token_in = Pubkey::new_unique();
    let token_out = Pubkey::new_unique();
    let quote = create_test_quote(1_000_000, 900_000);

    let confidence = agent.evaluate_route(&quote).await.unwrap();
    assert!(confidence.score > 0.0 && confidence.score <= 1.0);
}

#[tokio::test]
async fn test_memory_recording() {
    let (mut agent, _) = setup_test_env().await.unwrap();
    
    let quote = create_test_quote(1_000_000, 900_000);
    
    // Record successful swap
    agent.record_success(&quote).await.unwrap();
    
    // Verify memory update
    let metrics = agent.get_metrics();
    assert!(!metrics.is_empty());
}

#[tokio::test]
async fn test_slippage_protection() {
    let (agent, engine) = setup_test_env().await.unwrap();
    
    let token_in = USDC.parse::<Pubkey>().unwrap();
    let token_out = SOL.parse::<Pubkey>().unwrap();
    
    let quote = engine.get_best_quote(&token_in, &token_out, 1_000_000)
        .await
        .unwrap();
    
    // Verify slippage is within limits
    assert!(quote.minimum_out > 0);
    assert!(quote.minimum_out < quote.amount_out);
}

#[tokio::test]
async fn test_price_impact_rejection() {
    let (agent, _) = setup_test_env().await.unwrap();
    
    // Create quote with high price impact
    let mut quote = create_test_quote(1_000_000, 900_000);
    quote.price_impact_bps = 1000; // 10%
    
    let confidence = agent.evaluate_route(&quote).await.unwrap();
    assert!(confidence.score < 0.5); // Should have low confidence
}

#[tokio::test]
async fn test_memory_persistence() {
    let mut memory = Memory::new(100);
    
    // Add some test swaps
    let quote = create_test_quote(1_000_000, 900_000);
    memory.add_swap(quote.clone(), true).unwrap();
    
    // Check success rate
    let success_rate = memory.get_success_rate(
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        DexType::Raydium,
    );
    assert_eq!(success_rate, 1.0);
}

#[tokio::test]
async fn test_agent_learning() {
    let (mut agent, _) = setup_test_env().await.unwrap();
    
    // Simulate multiple swaps
    for i in 0..5 {
        let quote = create_test_quote(1_000_000, 900_000 - i * 1000);
        
        if i % 2 == 0 {
            agent.record_success(&quote).await.unwrap();
        } else {
            agent.record_failure(&quote).await.unwrap();
        }
    }
    
    // Evaluate similar route
    let new_quote = create_test_quote(1_000_000, 900_000);
    let confidence = agent.evaluate_route(&new_quote).await.unwrap();
    
    // Agent should have learned from history
    assert!(confidence.score != 0.5);
}

#[tokio::test]
async fn test_concurrent_operations() {
    let (agent, engine) = setup_test_env().await.unwrap();
    
    let handles: Vec<_> = (0..3).map(|_| {
        let agent_clone = agent.clone();
        let engine_clone = engine.clone();
        
        tokio::spawn(async move {
            let quote = engine_clone.get_best_quote(
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                1_000_000,
            ).await.unwrap();
            
            agent_clone.evaluate_route(&quote).await.unwrap()
        })
    }).collect();
    
    for handle in handles {
        let confidence = handle.await.unwrap();
        assert!(confidence.score > 0.0);
    }
}

#[tokio::test]
async fn test_error_handling() {
    let (agent, _) = setup_test_env().await.unwrap();
    
    // Test with invalid quote
    let quote = create_test_quote(0, 0);
    let result = agent.evaluate_route(&quote).await;
    assert!(result.is_err());
}