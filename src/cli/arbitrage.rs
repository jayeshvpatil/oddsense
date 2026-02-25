use anyhow::Result;

use crate::adapters::kalshi::KalshiAdapter;
use crate::adapters::metaculus::MetaculusAdapter;
use crate::adapters::polymarket::PolymarketAdapter;
use crate::adapters::schema::NormalizedMarket;
use crate::adapters::MarketSource;
use crate::analysis::arbitrage::{find_arbitrage, ArbitrageResponse};
use crate::output::{self, OutputFormat};

pub async fn run(
    query: Option<&str>,
    sources: &str,
    min_spread: f64,
    similarity: f64,
    limit: usize,
    format: OutputFormat,
    quiet: bool,
    raw: bool,
) -> Result<()> {
    if !quiet {
        if let Some(q) = query {
            eprintln!("Finding arbitrage opportunities for \"{}\"...", q);
        } else {
            eprintln!("Finding arbitrage opportunities across platforms...");
        }
    }

    let source_list: Vec<&str> = sources.split(',').map(|s| s.trim()).collect();
    let fetch_limit = limit * 3; // fetch more to increase matching chances
    let mut all_markets: Vec<NormalizedMarket> = Vec::new();

    // Fetch from each source in parallel
    let use_polymarket = source_list.contains(&"polymarket") || source_list.contains(&"all");
    let use_kalshi = source_list.contains(&"kalshi") || source_list.contains(&"all");
    let use_metaculus = source_list.contains(&"metaculus") || source_list.contains(&"all");

    let poly_fut = async {
        if !use_polymarket {
            return Ok(vec![]);
        }
        let adapter = PolymarketAdapter::new();
        if !adapter.is_available() {
            if !quiet {
                eprintln!("  [polymarket] polymarket-cli not found, skipping");
            }
            return Ok(vec![]);
        }
        if !quiet {
            eprintln!("  Fetching from polymarket...");
        }
        match query {
            Some(q) => adapter.search(q, fetch_limit).await,
            None => adapter.list_top(fetch_limit, "volume").await,
        }
    };

    let kalshi_fut = async {
        if !use_kalshi {
            return Ok(vec![]);
        }
        let adapter = KalshiAdapter::new();
        if !quiet {
            eprintln!("  Fetching from kalshi...");
        }
        match query {
            Some(q) => adapter.search(q, fetch_limit).await,
            None => adapter.list_top(fetch_limit, "volume").await,
        }
    };

    let metaculus_fut = async {
        if !use_metaculus {
            return Ok(vec![]);
        }
        let adapter = MetaculusAdapter::new();
        if !quiet {
            eprintln!("  Fetching from metaculus...");
        }
        match query {
            Some(q) => adapter.search(q, fetch_limit).await,
            None => adapter.list_top(fetch_limit, "volume").await,
        }
    };

    // Run all fetches concurrently
    let (poly_result, kalshi_result, metaculus_result) =
        tokio::join!(poly_fut, kalshi_fut, metaculus_fut);

    // Collect results, logging errors but not failing
    match poly_result {
        Ok(markets) => {
            if !quiet {
                eprintln!("  [polymarket] {} markets fetched", markets.len());
            }
            all_markets.extend(markets);
        }
        Err(e) => {
            if !quiet {
                eprintln!("  [polymarket] error: {}", e);
            }
        }
    }

    match kalshi_result {
        Ok(markets) => {
            if !quiet {
                eprintln!("  [kalshi] {} markets fetched", markets.len());
            }
            all_markets.extend(markets);
        }
        Err(e) => {
            if !quiet {
                eprintln!("  [kalshi] error: {}", e);
            }
        }
    }

    match metaculus_result {
        Ok(markets) => {
            if !quiet {
                eprintln!("  [metaculus] {} markets fetched", markets.len());
            }
            all_markets.extend(markets);
        }
        Err(e) => {
            if !quiet {
                eprintln!("  [metaculus] error: {}", e);
            }
        }
    }

    if !quiet {
        eprintln!(
            "  Total: {} markets across platforms, analyzing...",
            all_markets.len()
        );
    }

    // Find arbitrage opportunities
    let mut opportunities = find_arbitrage(&all_markets, min_spread, similarity);
    opportunities.truncate(limit);

    let response = ArbitrageResponse {
        count: opportunities.len(),
        query: query.map(String::from),
        min_spread,
        opportunities,
    };

    output::render(&response, format, quiet, raw)?;
    Ok(())
}
