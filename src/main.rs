use agent_swap::{
    agent::{Memory, SwapAgent},
    swap::{DexType, SwapEngine},
    utils::solana::{setup_client, setup_wallet},
};

use anchor_client::Client;
use anyhow::Result;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
};
use std::str::FromStr;
use tracing::{info, warn, error};

// Configuration constants
const RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const WALLET_PATH: &str = "~/.config/solana/id.json";

// Token addresses (mainnet)
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const SOL: &str = "So11111111111111111111111111111111111111112";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    info!("Starting Agent-Swap");

    // Setup Solana client
    let client = setup_client(RPC_URL, CommitmentConfig::confirmed())?;
    info!("Connected to Solana network");

    // Load wallet
    let wallet = match setup_wallet(WALLET_PATH) {
        Ok(keypair) => keypair,
        Err(e) => {
            error!("Failed to load wallet: {}", e);
            return Err(anyhow::anyhow!("Wallet setup failed"));
        }
    };
    info!("Wallet loaded successfully");

    // Initialize swap engine
    let swap_engine = SwapEngine::new(client.clone())?;
    info!("Swap engine initialized");

    // Initialize agent with Gloom
    let agent = SwapAgent::new(
        client,
        Memory::default(),
        wallet.pubkey(),
    )?;
    info!("Agent initialized with Gloom integration");

    // Example: Get quote for USDC -> SOL
    let usdc = Pubkey::from_str(USDC)?;
    let sol = Pubkey::from_str(SOL)?;
    let amount = 100_000_000; // 100 USDC (6 decimals)

    info!("Requesting quote for USDC -> SOL swap");
    match swap_engine.get_best_route(&usdc, &sol, amount).await {
        Ok(route) => {
            info!(
                "Best route found: {} -> {} via {:?}",
                "USDC", "SOL", route.dex_type
            );
            
            // Let agent evaluate the route
            if agent.evaluate_route(&route).await? {
                info!("Agent approved route, executing swap...");
                
                match swap_engine.execute_swap(&route, &wallet).await {
                    Ok(signature) => {
                        info!("Swap executed successfully! Signature: {}", signature);
                        
                        // Update agent memory with successful swap
                        agent.record_swap(route, signature).await?;
                    }
                    Err(e) => {
                        error!("Swap execution failed: {}", e);
                        // Update agent memory with failed attempt
                        agent.record_failure(route, e.to_string()).await?;
                    }
                }
            } else {
                warn!("Agent rejected route based on analysis");
            }
        }
        Err(e) => {
            error!("Failed to find swap route: {}", e);
        }
    }

    info!("Agent-Swap completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_swap_flow() {
        // Add integration tests here
    }

    #[test]
    fn test_client_setup() {
        // Add unit tests here
    }
}