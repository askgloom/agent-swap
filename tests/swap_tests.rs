use agent_swap::{
    swap::{DexType, Quote, SwapEngine},
    utils::{self, format_amount, parse_amount},
    Config, Result,
};

use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    transaction::Transaction,
};
use tokio;
use std::str::FromStr;

// Test constants
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const SOL: &str = "So11111111111111111111111111111111111111112";
const USDT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
const RAY: &str = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R";

/// Helper function to setup test environment
async fn setup_test_env() -> Result<SwapEngine> {
    let config = Config {
        rpc_url: "https://api.devnet.solana.com".to_string(),
        ..Config::default()
    };

    let (_, engine) = agent_swap::init(config).await?;
    Ok(engine)
}

#[tokio::test]
async fn test_engine_initialization() {
    let engine = setup_test_env().await.unwrap();
    assert!(engine.get_best_quote(
        &Pubkey::from_str(USDC).unwrap(),
        &Pubkey::from_str(SOL).unwrap(),
        1_000_000
    ).await.is_ok());
}

#[tokio::test]
async fn test_quote_comparison() {
    let engine = setup_test_env().await.unwrap();
    
    let usdc = Pubkey::from_str(USDC).unwrap();
    let sol = Pubkey::from_str(SOL).unwrap();
    let amount = parse_amount("100", 6).unwrap(); // 100 USDC
    
    let quote = engine.get_best_quote(&usdc, &sol, amount).await.unwrap();
    
    // Verify quote properties
    assert!(quote.amount_out > 0);
    assert!(quote.price_impact_bps < 1000); // Less than 10%
    assert_eq!(quote.amount_in, amount);
}

#[tokio::test]
async fn test_multiple_routes() {
    let engine = setup_test_env().await.unwrap();
    
    let usdc = Pubkey::from_str(USDC).unwrap();
    let ray = Pubkey::from_str(RAY).unwrap();
    let amount = parse_amount("100", 6).unwrap();
    
    // Get quotes from different DEXes
    let raydium_quote = engine.get_quote(DexType::Raydium, &usdc, &ray, amount)
        .await
        .unwrap();
    let orca_quote = engine.get_quote(DexType::Orca, &usdc, &ray, amount)
        .await
        .unwrap();
    
    // Best quote should be better than or equal to individual quotes
    let best_quote = engine.get_best_quote(&usdc, &ray, amount).await.unwrap();
    assert!(best_quote.amount_out >= raydium_quote.amount_out);
    assert!(best_quote.amount_out >= orca_quote.amount_out);
}

#[tokio::test]
async fn test_slippage_calculation() {
    let engine = setup_test_env().await.unwrap();
    
    let usdc = Pubkey::from_str(USDC).unwrap();
    let usdt = Pubkey::from_str(USDT).unwrap();
    let amount = parse_amount("1000", 6).unwrap();
    
    let quote = engine.get_best_quote(&usdc, &usdt, amount).await.unwrap();
    
    // Verify slippage protection
    assert!(quote.minimum_out >= quote.amount_out * 99 / 100); // 1% max slippage
    assert!(quote.minimum_out <= quote.amount_out);
}

#[tokio::test]
async fn test_price_impact() {
    let engine = setup_test_env().await.unwrap();
    
    // Test with different amounts
    let usdc = Pubkey::from_str(USDC).unwrap();
    let sol = Pubkey::from_str(SOL).unwrap();
    
    let small_amount = parse_amount("10", 6).unwrap();
    let large_amount = parse_amount("10000", 6).unwrap();
    
    let small_quote = engine.get_best_quote(&usdc, &sol, small_amount).await.unwrap();
    let large_quote = engine.get_best_quote(&usdc, &sol, large_amount).await.unwrap();
    
    // Larger amounts should have higher price impact
    assert!(large_quote.price_impact_bps > small_quote.price_impact_bps);
}

#[tokio::test]
async fn test_quote_caching() {
    let engine = setup_test_env().await.unwrap();
    
    let usdc = Pubkey::from_str(USDC).unwrap();
    let sol = Pubkey::from_str(SOL).unwrap();
    let amount = parse_amount("100", 6).unwrap();
    
    // Get quote twice
    let quote1 = engine.get_best_quote(&usdc, &sol, amount).await.unwrap();
    let quote2 = engine.get_best_quote(&usdc, &sol, amount).await.unwrap();
    
    // Should get same result from cache
    assert_eq!(quote1.amount_out, quote2.amount_out);
}

#[tokio::test]
async fn test_transaction_building() {
    let engine = setup_test_env().await.unwrap();
    let wallet = Keypair::new();
    
    let usdc = Pubkey::from_str(USDC).unwrap();
    let sol = Pubkey::from_str(SOL).unwrap();
    let amount = parse_amount("100", 6).unwrap();
    
    let quote = engine.get_best_quote(&usdc, &sol, amount).await.unwrap();
    
    // Verify transaction
    assert!(!quote.transaction.message.instructions.is_empty());
    assert!(quote.transaction.message.header.num_required_signatures > 0);
}

#[tokio::test]
async fn test_concurrent_quotes() {
    let engine = setup_test_env().await.unwrap();
    
    let handles: Vec<_> = (0..3).map(|_| {
        let engine_clone = engine.clone();
        let usdc = Pubkey::from_str(USDC).unwrap();
        let sol = Pubkey::from_str(SOL).unwrap();
        
        tokio::spawn(async move {
            engine_clone.get_best_quote(&usdc, &sol, 1_000_000).await
        })
    }).collect();
    
    for handle in handles {
        let quote = handle.await.unwrap().unwrap();
        assert!(quote.amount_out > 0);
    }
}

#[tokio::test]
async fn test_error_handling() {
    let engine = setup_test_env().await.unwrap();
    
    // Test with invalid token
    let invalid_token = Pubkey::new_unique();
    let sol = Pubkey::from_str(SOL).unwrap();
    
    let result = engine.get_best_quote(&invalid_token, &sol, 1_000_000).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_minimum_amount() {
    let engine = setup_test_env().await.unwrap();
    
    let usdc = Pubkey::from_str(USDC).unwrap();
    let sol = Pubkey::from_str(SOL).unwrap();
    let small_amount = 100; // Too small
    
    let result = engine.get_best_quote(&usdc, &sol, small_amount).await;
    assert!(result.is_err());
}