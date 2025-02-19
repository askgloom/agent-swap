//! Agent-Swap: An intelligent DEX aggregator for Solana powered by Gloom
//! 
//! This library provides a framework for automated trading on Solana DEXes
//! using AI-powered decision making through the Gloom framework.

pub mod agent;
pub mod swap;
pub mod utils;

use anchor_client::Client;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    transaction::Transaction,
};
use thiserror::Error;

/// Core error types for agent-swap operations
#[derive(Error, Debug)]
pub enum AgentSwapError {
    #[error("Solana client error: {0}")]
    ClientError(#[from] solana_client::client_error::ClientError),

    #[error("Insufficient funds: required {required} but found {available}")]
    InsufficientFunds {
        required: u64,
        available: u64,
    },

    #[error("Route not found between {from} and {to}")]
    RouteNotFound {
        from: Pubkey,
        to: Pubkey,
    },

    #[error("Slippage exceeded: expected {expected}, got {actual}")]
    SlippageExceeded {
        expected: f64,
        actual: f64,
    },

    #[error("Agent error: {0}")]
    AgentError(String),

    #[error("DEX error: {0}")]
    DexError(String),
}

/// Result type for agent-swap operations
pub type Result<T> = std::result::Result<T, AgentSwapError>;

/// Configuration for swap operations
#[derive(Debug, Clone)]
pub struct SwapConfig {
    /// Maximum acceptable slippage (in basis points)
    pub slippage_bps: u16,
    /// Minimum amount to swap
    pub min_amount: u64,
    /// Timeout for swap execution (in seconds)
    pub timeout_seconds: u64,
    /// Whether to use AI optimization
    pub use_ai: bool,
}

impl Default for SwapConfig {
    fn default() -> Self {
        Self {
            slippage_bps: 100, // 1%
            min_amount: 1000,
            timeout_seconds: 60,
            use_ai: true,
        }
    }
}

/// Represents a swap route with price and impact information
#[derive(Debug, Clone)]
pub struct SwapRoute {
    /// Source token
    pub token_in: Pubkey,
    /// Destination token
    pub token_out: Pubkey,
    /// Amount to swap
    pub amount_in: u64,
    /// Expected output amount
    pub amount_out: u64,
    /// Price impact (in basis points)
    pub price_impact_bps: u16,
    /// DEX to use for the swap
    pub dex_type: swap::DexType,
    /// Prepared transaction
    pub transaction: Transaction,
}

/// Core trait for swap execution
#[async_trait::async_trait]
pub trait SwapExecutor {
    /// Execute a swap following the given route
    async fn execute_swap(
        &self,
        route: &SwapRoute,
        wallet: &Keypair,
    ) -> Result<String>; // Returns transaction signature
}

/// Statistics for swap operations
#[derive(Debug, Default)]
pub struct SwapStats {
    /// Total number of swaps executed
    pub total_swaps: u64,
    /// Number of successful swaps
    pub successful_swaps: u64,
    /// Total volume (in USDC)
    pub total_volume: f64,
    /// Average success rate
    pub success_rate: f64,
    /// Average execution time (in seconds)
    pub avg_execution_time: f64,
}

/// Metrics collection for monitoring
#[derive(Debug)]
pub struct Metrics {
    /// Swap statistics
    pub stats: SwapStats,
    /// Performance metrics
    pub performance: std::collections::HashMap<String, f64>,
}

impl Metrics {
    /// Record a successful swap
    pub fn record_success(&mut self, volume: f64, execution_time: f64) {
        self.stats.total_swaps += 1;
        self.stats.successful_swaps += 1;
        self.stats.total_volume += volume;
        self.stats.success_rate = self.stats.successful_swaps as f64 
            / self.stats.total_swaps as f64;
        self.stats.avg_execution_time = 
            (self.stats.avg_execution_time * (self.stats.total_swaps - 1) as f64
            + execution_time) / self.stats.total_swaps as f64;
    }

    /// Record a failed swap
    pub fn record_failure(&mut self) {
        self.stats.total_swaps += 1;
        self.stats.success_rate = self.stats.successful_swaps as f64 
            / self.stats.total_swaps as f64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_config_default() {
        let config = SwapConfig::default();
        assert_eq!(config.slippage_bps, 100);
        assert_eq!(config.min_amount, 1000);
        assert_eq!(config.timeout_seconds, 60);
        assert!(config.use_ai);
    }

    #[test]
    fn test_metrics_recording() {
        let mut metrics = Metrics {
            stats: SwapStats::default(),
            performance: std::collections::HashMap::new(),
        };

        metrics.record_success(1000.0, 2.0);
        assert_eq!(metrics.stats.total_swaps, 1);
        assert_eq!(metrics.stats.successful_swaps, 1);
        assert_eq!(metrics.stats.total_volume, 1000.0);
        assert_eq!(metrics.stats.success_rate, 1.0);
        assert_eq!(metrics.stats.avg_execution_time, 2.0);

        metrics.record_failure();
        assert_eq!(metrics.stats.total_swaps, 2);
        assert_eq!(metrics.stats.successful_swaps, 1);
        assert_eq!(metrics.stats.success_rate, 0.5);
    }
}