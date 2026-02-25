use anyhow::Result;

use crate::adapters::polymarket::PolymarketAdapter;
use crate::adapters::schema::MarketResponse;
use crate::adapters::MarketSource;
use crate::output::{self, OutputFormat};

pub async fn run(
    _timeframe: &str,
    min_volume: f64,
    limit: usize,
    format: OutputFormat,
    quiet: bool,
    raw: bool,
) -> Result<()> {
    if !quiet {
        eprintln!("Fetching market signals...");
    }

    let adapter = PolymarketAdapter::new();
    if !adapter.is_available() {
        anyhow::bail!(
            "polymarket-cli not found in PATH.\n\
             Install: cargo install --git https://github.com/Polymarket/polymarket-cli.git"
        );
    }

    // Fetch top markets by volume — these are the "movers"
    let mut markets = adapter.list_top(limit * 2, "volume_num").await?;

    // Filter by minimum volume
    markets.retain(|m| m.volume_24h.unwrap_or(0.0) >= min_volume);
    markets.truncate(limit);

    let response = MarketResponse {
        count: markets.len(),
        query: None,
        source: "polymarket".to_string(),
        markets,
    };

    output::render(&response, format, quiet, raw)?;
    Ok(())
}
