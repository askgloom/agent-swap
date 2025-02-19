//! Memory system for the swap agent
//! 
//! Stores and manages historical swap data and performance metrics
//! to inform future decision making.

use solana_sdk::pubkey::Pubkey;
use crate::{
    swap::DexType,
    SwapRoute,
    Result,
    AgentSwapError,
};

use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

/// Represents a historical swap record
#[derive(Debug, Clone)]
pub struct SwapRecord {
    /// Timestamp of the swap
    pub timestamp: u64,
    /// Source token
    pub token_in: Pubkey,
    /// Destination token
    pub token_out: Pubkey,
    /// Amount swapped
    pub amount_in: u64,
    /// Amount received
    pub amount_out: u64,
    /// DEX used
    pub dex_type: DexType,
    /// Whether the swap was successful
    pub success: bool,
    /// Price impact in basis points
    pub price_impact_bps: u16,
    /// Transaction signature
    pub signature: String,
}

/// Historical performance metrics for a specific route
#[derive(Debug, Default)]
pub struct RouteMetrics {
    /// Total number of swaps
    pub total_swaps: u64,
    /// Number of successful swaps
    pub successful_swaps: u64,
    /// Average price impact
    pub avg_price_impact: f64,
    /// Best historical rate
    pub best_rate: f64,
    /// Worst historical rate
    pub worst_rate: f64,
    /// Last update timestamp
    pub last_update: u64,
}

/// Memory system for storing swap history
#[derive(Debug, Default)]
pub struct Memory {
    /// Historical swap records
    records: Vec<SwapRecord>,
    /// Cached metrics per route
    metrics: HashMap<(Pubkey, Pubkey, DexType), RouteMetrics>,
    /// Maximum records to keep
    max_records: usize,
}

impl Memory {
    /// Create a new memory system with specified capacity
    pub fn new(max_records: usize) -> Self {
        Self {
            records: Vec::with_capacity(max_records),
            metrics: HashMap::new(),
            max_records,
        }
    }

    /// Add a new swap record
    pub fn add_swap(&mut self, route: SwapRoute, success: bool) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AgentSwapError::AgentError(e.to_string()))?
            .as_secs();

        let record = SwapRecord {
            timestamp,
            token_in: route.token_in,
            token_out: route.token_out,
            amount_in: route.amount_in,
            amount_out: route.amount_out,
            dex_type: route.dex_type,
            success,
            price_impact_bps: route.price_impact_bps,
            signature: String::new(), // Set this when available
        };

        // Update metrics
        self.update_metrics(&record);

        // Add record and maintain size limit
        if self.records.len() >= self.max_records {
            self.records.remove(0);
        }
        self.records.push(record);

        Ok(())
    }

    /// Get relevant swap history for a route
    pub fn get_relevant_swaps(
        &self,
        token_in: Pubkey,
        token_out: Pubkey,
        dex_type: DexType,
    ) -> RouteMetrics {
        self.metrics
            .get(&(token_in, token_out, dex_type))
            .cloned()
            .unwrap_or_default()
    }

    /// Get recent swaps within a time window
    pub fn get_recent_swaps(&self, seconds: u64) -> Vec<&SwapRecord> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.records
            .iter()
            .filter(|r| now - r.timestamp <= seconds)
            .collect()
    }

    /// Calculate success rate for a specific route
    pub fn get_success_rate(
        &self,
        token_in: Pubkey,
        token_out: Pubkey,
        dex_type: DexType,
    ) -> f64 {
        let metrics = self.get_relevant_swaps(token_in, token_out, dex_type);
        if metrics.total_swaps == 0 {
            return 0.0;
        }
        metrics.successful_swaps as f64 / metrics.total_swaps as f64
    }

    // Private helper methods
    fn update_metrics(&mut self, record: &SwapRecord) {
        let key = (record.token_in, record.token_out, record.dex_type);
        let metrics = self.metrics.entry(key).or_default();

        metrics.total_swaps += 1;
        if record.success {
            metrics.successful_swaps += 1;
        }

        let rate = record.amount_out as f64 / record.amount_in as f64;
        metrics.best_rate = metrics.best_rate.max(rate);
        metrics.worst_rate = if metrics.worst_rate == 0.0 {
            rate
        } else {
            metrics.worst_rate.min(rate)
        };

        // Update average price impact
        metrics.avg_price_impact = (metrics.avg_price_impact * (metrics.total_swaps - 1) as f64
            + record.price_impact_bps as f64) / metrics.total_swaps as f64;

        metrics.last_update = record.timestamp;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Keypair;

    fn create_test_route() -> SwapRoute {
        SwapRoute {
            token_in: Keypair::new().pubkey(),
            token_out: Keypair::new().pubkey(),
            amount_in: 1000000,
            amount_out: 900000,
            price_impact_bps: 100,
            dex_type: DexType::Raydium,
            transaction: Transaction::default(),
        }
    }

    #[test]
    fn test_memory_capacity() {
        let mut memory = Memory::new(2);
        let route = create_test_route();

        memory.add_swap(route.clone(), true).unwrap();
        memory.add_swap(route.clone(), true).unwrap();
        memory.add_swap(route.clone(), true).unwrap();

        assert_eq!(memory.records.len(), 2);
    }

    #[test]
    fn test_success_rate() {
        let mut memory = Memory::new(10);
        let route = create_test_route();

        memory.add_swap(route.clone(), true).unwrap();
        memory.add_swap(route.clone(), false).unwrap();

        let rate = memory.get_success_rate(
            route.token_in,
            route.token_out,
            route.dex_type,
        );
        assert_eq!(rate, 0.5);
    }
}