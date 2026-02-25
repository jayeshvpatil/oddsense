pub mod kalshi;
pub mod metaculus;
pub mod polymarket;
pub mod schema;

use anyhow::Result;
use async_trait::async_trait;
use schema::NormalizedMarket;

/// Trait that all prediction market source adapters implement.
#[async_trait]
pub trait MarketSource: Send + Sync {
    fn name(&self) -> &str;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<NormalizedMarket>>;
    async fn list_top(&self, limit: usize, sort_by: &str) -> Result<Vec<NormalizedMarket>>;
    async fn get_market(&self, id: &str) -> Result<NormalizedMarket>;
    /// Check if the underlying CLI/API is accessible.
    fn is_available(&self) -> bool;
}
