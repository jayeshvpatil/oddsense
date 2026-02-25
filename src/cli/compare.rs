use anyhow::Result;

use crate::adapters::kalshi::KalshiAdapter;
use crate::adapters::metaculus::MetaculusAdapter;
use crate::adapters::polymarket::PolymarketAdapter;
use crate::adapters::schema::NormalizedMarket;
use crate::adapters::MarketSource;
use crate::analysis::arbitrage::{title_similarity, CompareResponse};
use crate::output::{self, OutputFormat};

pub async fn run(
    query: &str,
    sources: &str,
    similarity: f64,
    limit: usize,
    format: OutputFormat,
    quiet: bool,
    raw: bool,
) -> Result<()> {
    if !quiet {
        eprintln!("Comparing \"{}\" across platforms...", query);
    }

    let source_list: Vec<&str> = sources.split(',').map(|s| s.trim()).collect();
    let mut all_markets: Vec<NormalizedMarket> = Vec::new();
    let mut used_sources: Vec<String> = Vec::new();

    let use_polymarket = source_list.contains(&"polymarket") || source_list.contains(&"all");
    let use_kalshi = source_list.contains(&"kalshi") || source_list.contains(&"all");
    let use_metaculus = source_list.contains(&"metaculus") || source_list.contains(&"all");

    let poly_fut = async {
        if !use_polymarket {
            return Ok(vec![]);
        }
        let adapter = PolymarketAdapter::new();
        if !adapter.is_available() {
            return Ok(vec![]);
        }
        if !quiet {
            eprintln!("  Searching polymarket...");
        }
        adapter.search(query, limit).await
    };

    let kalshi_fut = async {
        if !use_kalshi {
            return Ok(vec![]);
        }
        let adapter = KalshiAdapter::new();
        if !quiet {
            eprintln!("  Searching kalshi...");
        }
        adapter.search(query, limit).await
    };

    let metaculus_fut = async {
        if !use_metaculus {
            return Ok(vec![]);
        }
        let adapter = MetaculusAdapter::new();
        if !quiet {
            eprintln!("  Searching metaculus...");
        }
        adapter.search(query, limit).await
    };

    let (poly_result, kalshi_result, metaculus_result) =
        tokio::join!(poly_fut, kalshi_fut, metaculus_fut);

    if let Ok(markets) = poly_result {
        if !markets.is_empty() {
            used_sources.push("polymarket".to_string());
            all_markets.extend(markets);
        }
    }
    if let Ok(markets) = kalshi_result {
        if !markets.is_empty() {
            used_sources.push("kalshi".to_string());
            all_markets.extend(markets);
        }
    }
    if let Ok(markets) = metaculus_result {
        if !markets.is_empty() {
            used_sources.push("metaculus".to_string());
            all_markets.extend(markets);
        }
    }

    // Filter to markets that are relevant to the query (by title similarity)
    let query_norm = query.to_lowercase();
    let mut relevant: Vec<NormalizedMarket> = all_markets
        .into_iter()
        .filter(|m| {
            let title_lower = m.title.to_lowercase();
            title_lower.contains(&query_norm)
                || title_similarity(&m.title, query) >= similarity
        })
        .collect();

    // Sort by source then probability for a clean side-by-side view
    relevant.sort_by(|a, b| {
        a.source
            .cmp(&b.source)
            .then(b.probability.partial_cmp(&a.probability).unwrap())
    });
    relevant.truncate(limit);

    // Recompute sources based on what actually made it through the filter
    let actual_sources: Vec<String> = {
        let mut s: Vec<String> = relevant.iter().map(|m| m.source.clone()).collect();
        s.sort();
        s.dedup();
        s
    };

    let response = CompareResponse {
        query: query.to_string(),
        sources: actual_sources,
        markets: relevant,
    };

    output::render(&response, format, quiet, raw)?;
    Ok(())
}
