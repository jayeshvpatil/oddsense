use anyhow::Result;

use crate::adapters::kalshi::KalshiAdapter;
use crate::adapters::metaculus::MetaculusAdapter;
use crate::adapters::polymarket::PolymarketAdapter;
use crate::adapters::schema::{MarketResponse, NormalizedMarket};
use crate::adapters::MarketSource;
use crate::config;
use crate::llm;
use crate::output::{self, OutputFormat};
use crate::search::categories::{self, MarketCategory};

pub async fn run(
    query: &str,
    sources: &str,
    category: Option<&str>,
    limit: usize,
    _sort: &str,
    format: OutputFormat,
    quiet: bool,
    raw: bool,
    smart: bool,
    config_path: Option<&str>,
) -> Result<()> {
    let source_list: Vec<&str> = sources.split(',').map(|s| s.trim()).collect();
    let use_polymarket = source_list.contains(&"polymarket") || source_list.contains(&"all");
    let use_kalshi = source_list.contains(&"kalshi") || source_list.contains(&"all");
    let use_metaculus = source_list.contains(&"metaculus") || source_list.contains(&"all");

    // Parse category filter if provided
    let category_filter: Option<MarketCategory> =
        category.and_then(MarketCategory::from_str_loose);
    if category.is_some() && category_filter.is_none() {
        eprintln!(
            "Warning: unknown category \"{}\". Valid: politics, economics, technology, crypto, sports, science, geopolitics, culture",
            category.unwrap_or("")
        );
    }

    if !quiet {
        let names: Vec<&str> = [
            if use_polymarket { Some("polymarket") } else { None },
            if use_kalshi { Some("kalshi") } else { None },
            if use_metaculus { Some("metaculus") } else { None },
        ]
        .into_iter()
        .flatten()
        .collect();
        let cat_label = category_filter
            .as_ref()
            .map(|c| format!(" [{}]", c))
            .unwrap_or_default();
        eprintln!(
            "Searching {} for \"{}\"{}...",
            names.join(" + "),
            query,
            cat_label
        );
    }

    // --- LLM-powered smart mode ---
    // Try to create an LLM provider if --smart is set
    let llm_provider = if smart {
        let cfg = config::load_config(config_path).unwrap_or_default();
        let provider = llm::provider::create_provider(
            cfg.api_keys.anthropic.as_deref(),
            cfg.llm.provider.as_deref(),
            cfg.llm.model.as_deref(),
        );
        if provider.is_none() && !quiet {
            eprintln!("  Note: --smart requires an Anthropic API key. Falling back to synonym expansion.");
        }
        provider
    } else {
        None
    };

    // If smart mode is on and provider is available, run LLM query expansion in parallel with fetches
    let llm_expansion_fut = async {
        if let Some(ref provider) = llm_provider {
            match provider.expand_query(query).await {
                Ok(expansion) => Some(expansion),
                Err(e) => {
                    if !quiet {
                        eprintln!("  Warning: LLM query expansion failed: {}", e);
                    }
                    None
                }
            }
        } else {
            None
        }
    };

    // Fetch extra results when category filtering to compensate for filtered-out markets
    let fetch_limit = if category_filter.is_some() {
        limit * 5
    } else {
        limit
    };

    let poly_fut = async {
        if !use_polymarket {
            return Ok(vec![]);
        }
        let adapter = PolymarketAdapter::new();
        if !adapter.is_available() {
            if !quiet {
                eprintln!("  Warning: polymarket-cli not found, skipping polymarket");
            }
            return Ok(vec![]);
        }
        adapter.search(query, fetch_limit).await
    };

    let kalshi_fut = async {
        if !use_kalshi {
            return Ok(vec![]);
        }
        let adapter = KalshiAdapter::new();
        adapter.search(query, fetch_limit).await
    };

    let metaculus_fut = async {
        if !use_metaculus {
            return Ok(vec![]);
        }
        let adapter = MetaculusAdapter::new();
        adapter.search(query, fetch_limit).await
    };

    let (poly_result, kalshi_result, metaculus_result, llm_expansion) =
        tokio::join!(poly_fut, kalshi_fut, metaculus_fut, llm_expansion_fut);

    let mut all_markets: Vec<NormalizedMarket> = Vec::new();
    let mut used_sources: Vec<&str> = Vec::new();

    if let Ok(markets) = poly_result {
        if !markets.is_empty() {
            used_sources.push("polymarket");
            all_markets.extend(markets);
        }
    } else if !quiet {
        eprintln!("  Warning: polymarket search failed");
    }

    if let Ok(markets) = kalshi_result {
        if !markets.is_empty() {
            used_sources.push("kalshi");
            all_markets.extend(markets);
        }
    } else if !quiet {
        eprintln!("  Warning: kalshi search failed");
    }

    if let Ok(markets) = metaculus_result {
        if !markets.is_empty() {
            used_sources.push("metaculus");
            all_markets.extend(markets);
        }
    } else if !quiet {
        eprintln!("  Warning: metaculus search failed");
    }

    // If LLM expansion provided a category and user didn't specify one, use it
    let effective_category = if category_filter.is_some() {
        category_filter
    } else {
        llm_expansion
            .as_ref()
            .and_then(|exp| exp.category.as_deref())
            .and_then(MarketCategory::from_str_loose)
    };

    // Log LLM expansion info
    if let Some(ref expansion) = llm_expansion {
        if !quiet {
            eprintln!(
                "  Smart mode: expanded to {} queries{}",
                expansion.expanded_queries.len(),
                expansion
                    .category
                    .as_ref()
                    .map(|c| format!(", detected category: {}", c))
                    .unwrap_or_default()
            );
        }
    }

    // Apply category filter if specified (user-provided or LLM-inferred in smart mode)
    if category.is_some() {
        // Only apply strict category filtering when user explicitly requested it
        if let Some(ref cat) = effective_category {
            all_markets.retain(|m| {
                let market_cat = categories::categorize(&m.title, &m.description);
                &market_cat == cat
            });
        }
    }

    // --- LLM reranking (smart mode) ---
    if let Some(ref provider) = llm_provider {
        if !all_markets.is_empty() {
            let titles: Vec<String> = all_markets.iter().map(|m| m.title.clone()).collect();
            match provider.rerank_results(query, &titles).await {
                Ok(rerank) if !rerank.rankings.is_empty() => {
                    if !quiet {
                        eprintln!("  Smart mode: reranked {} results by LLM relevance", rerank.rankings.len());
                    }
                    // Build a map of index → relevance score
                    let score_map: std::collections::HashMap<usize, f64> = rerank
                        .rankings
                        .iter()
                        .map(|r| (r.index, r.relevance))
                        .collect();

                    // Sort by LLM relevance score (highest first), fallback to volume
                    all_markets.sort_by(|a, b| {
                        let idx_a = titles.iter().position(|t| t == &a.title).unwrap_or(usize::MAX);
                        let idx_b = titles.iter().position(|t| t == &b.title).unwrap_or(usize::MAX);
                        let score_a = score_map.get(&idx_a).copied().unwrap_or(0.0);
                        let score_b = score_map.get(&idx_b).copied().unwrap_or(0.0);
                        score_b
                            .partial_cmp(&score_a)
                            .unwrap()
                            .then_with(|| {
                                b.volume_24h
                                    .unwrap_or(0.0)
                                    .partial_cmp(&a.volume_24h.unwrap_or(0.0))
                                    .unwrap()
                            })
                    });
                }
                Ok(_) => {} // empty rankings, keep default sort
                Err(e) => {
                    if !quiet {
                        eprintln!("  Warning: LLM reranking failed: {}. Using volume sort.", e);
                    }
                }
            }
        }
    }

    // Default sort by volume descending if no LLM reranking was applied
    if llm_provider.is_none() {
        all_markets.sort_by(|a, b| {
            b.volume_24h
                .unwrap_or(0.0)
                .partial_cmp(&a.volume_24h.unwrap_or(0.0))
                .unwrap()
        });
    }
    all_markets.truncate(limit);

    let source_label = if used_sources.len() == 1 {
        used_sources[0].to_string()
    } else {
        used_sources.join("+")
    };

    let response = MarketResponse {
        count: all_markets.len(),
        query: Some(query.to_string()),
        source: source_label,
        markets: all_markets,
    };

    output::render(&response, format, quiet, raw)?;
    Ok(())
}
