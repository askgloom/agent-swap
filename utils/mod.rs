//! Agent-Swap: Intelligent DEX aggregator for Solana
//! 
//! This module serves as the root module for the agent-swap library,
//! re-exporting the main components and providing the primary interface.

pub mod agent;
pub mod swap;
pub mod utils;

use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

/// Re-export main components
pub use agent::{SwapAgent, Memory};
pub use swap::{SwapEngine, DexType, Quote};
pub use utils::{setup_client, setup_wallet};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Error types for agent-swap operations
#[derive(Error, Debug)]
pub enum AgentSwapError {
    #[error("Agent error: {0}")]
    AgentError(String),

    #[error("Swap error: {0}")]
    SwapError(String),

    #[error("Route not found between {from} and {to}")]
    RouteNotFound {
        from: Pubkey,
        to: Pubkey,
    },

    #[error("Insufficient balance: required {required}, found {available}")]
    InsufficientBalance {
        required: u64,
        available: u64,
    },

    #[error("Price impact too high: {0}bps")]
    PriceImpactTooHigh(u16),

    #[error("Slippage exceeded: expected {expected}, got {actual}")]
    SlippageExceeded {
        expected: u64,
        actual: u64,
    },

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error(transparent)]
    SolanaError(#[from] solana_client::client_error::ClientError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

/// Result type for agent-swap operations
pub type Result<T> = std::result::Result<T, AgentSwapError>;

/// Configuration for agent-swap
#[derive(Debug, Clone)]
pub struct Config {
    /// Maximum acceptable slippage (in basis points)
    pub max_slippage_bps: u16,
    /// Maximum acceptable price impact (in basis points)
    pub max_price_impact_bps: u16,
    /// Minimum amount to swap (in USDC)
    pub min_amount_usdc: u64,
    /// Whether to use AI optimization
    pub use_ai_optimization: bool,
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Commitment level
    pub commitment: solana_sdk::commitment_config::CommitmentConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_slippage_bps: 100,    // 1%
            max_price_impact_bps: 300, // 3%
            min_amount_usdc: 1_000_000, // 1 USDC
            use_ai_optimization: true,
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            commitment: solana_sdk::commitment_config::CommitmentConfig::confirmed(),
        }
    }
}

/// Initialize agent-swap with configuration
pub async fn init(config: Config) -> Result<(SwapAgent, SwapEngine)> {
    // Setup Solana client
    let client = utils::setup_client(&config.rpc_url, config.commitment)?;

    // Initialize swap engine
    let swap_engine = SwapEngine::new()?;

    // Initialize agent
    let agent = SwapAgent::new(
        client,
        Memory::default(),
        Pubkey::default(), // Replace with actual wallet
    )?;

    Ok((agent, swap_engine))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Keypair;

    #[tokio::test]
    async fn test_initialization() {
        let config = Config::default();
        let (agent, engine) = init(config).await.unwrap();
        
        // Verify initialization
        assert!(engine.get_best_quote(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            1_000_000
        ).await.is_ok());
    }

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.max_slippage_bps, 100);
        assert_eq!(config.max_price_impact_bps, 300);
        assert_eq!(config.min_amount_usdc, 1_000_000);
        assert!(config.use_ai_optimization);
    }
}