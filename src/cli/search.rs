use anyhow::Result;

use crate::adapters::polymarket::PolymarketAdapter;
use crate::adapters::schema::MarketResponse;
use crate::adapters::MarketSource;
use crate::output::{self, OutputFormat};

pub async fn run(
    query: &str,
    limit: usize,
    _sort: &str,
    format: OutputFormat,
    quiet: bool,
    raw: bool,
) -> Result<()> {
    if !quiet {
        eprintln!("Searching polymarket for \"{}\"...", query);
    }

    let adapter = PolymarketAdapter::new();

    if !adapter.is_available() {
        anyhow::bail!(
            "polymarket-cli not found in PATH.\n\
             Install: cargo install --git https://github.com/Polymarket/polymarket-cli.git"
        );
    }

    let markets = adapter.search(query, limit).await?;

    let response = MarketResponse {
        count: markets.len(),
        query: Some(query.to_string()),
        source: "polymarket".to_string(),
        markets,
    };

    output::render(&response, format, quiet, raw)?;
    Ok(())
}
