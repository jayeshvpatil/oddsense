use anyhow::Result;

use crate::adapters::polymarket::PolymarketAdapter;
use crate::adapters::MarketSource;
use crate::analysis::divergence::{compute_divergence, DivergenceResponse};
use crate::config;
use crate::output::{self, OutputFormat};
use crate::sentiment;

pub async fn run(
    query: &str,
    sentiment_sources: &str,
    min_score: f64,
    limit: usize,
    explain: bool,
    format: OutputFormat,
    quiet: bool,
    raw: bool,
    config_path: Option<&str>,
) -> Result<()> {
    if !quiet {
        eprintln!("Finding divergences for \"{}\"...", query);
    }

    let cfg = config::load_config(config_path)?;
    let newsapi_key = cfg.api_keys.newsapi.as_deref();

    // Step 1: Get markets from polymarket-cli
    let adapter = PolymarketAdapter::new();
    if !adapter.is_available() {
        anyhow::bail!(
            "polymarket-cli not found in PATH.\n\
             Install: cargo install --git https://github.com/Polymarket/polymarket-cli.git"
        );
    }

    if !quiet {
        eprintln!("  Fetching markets...");
    }
    let markets = adapter.search(query, limit * 2).await?;

    if markets.is_empty() {
        if !quiet {
            eprintln!("No markets found for \"{}\".", query);
        }
        let response = DivergenceResponse {
            count: 0,
            query: query.to_string(),
            divergences: vec![],
        };
        output::render(&response, format, quiet, raw)?;
        return Ok(());
    }

    // Step 2: Get sentiment for the query
    let use_news = sentiment_sources == "all" || sentiment_sources == "news";
    let use_reddit = sentiment_sources == "all" || sentiment_sources == "reddit";

    if !quiet {
        eprintln!("  Analyzing sentiment...");
    }
    let sent = sentiment::aggregate_sentiment(query, use_news, use_reddit, newsapi_key).await?;

    // Step 3: Compute divergences
    let mut divergences: Vec<_> = markets
        .iter()
        .map(|m| compute_divergence(m, &sent))
        .filter(|d| d.divergence_score >= min_score)
        .collect();

    divergences.sort_by(|a, b| {
        b.divergence_score
            .partial_cmp(&a.divergence_score)
            .unwrap()
    });
    divergences.truncate(limit);

    if explain && !quiet {
        for d in &divergences {
            eprintln!("\n  {}", d.summary);
        }
        eprintln!();
    }

    let response = DivergenceResponse {
        count: divergences.len(),
        query: query.to_string(),
        divergences,
    };

    output::render(&response, format, quiet, raw)?;
    Ok(())
}
