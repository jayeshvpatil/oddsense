use std::collections::HashMap;

use anyhow::Result;

use crate::adapters::polymarket::PolymarketAdapter;
use crate::adapters::schema::NormalizedMarket;
use crate::adapters::MarketSource;
use crate::analysis::arbitrage::title_similarity;
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
    let raw_markets = adapter.search(query, limit * 5).await?;

    // Deduplicate: multi-outcome events return many markets with the same URL.
    // Keep only the highest-probability outcome per event to get diverse results.
    let markets = dedup_markets(raw_markets);

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
    let base_sent =
        sentiment::aggregate_sentiment(query, use_news, use_reddit, newsapi_key).await?;

    // Step 3: Compute per-market sentiment by re-scoring signals against each market title.
    // This avoids extra API calls while giving each market a distinct sentiment score.
    let mut divergences: Vec<_> = markets
        .iter()
        .map(|m| {
            let market_sent = sentiment::rescore_for_market(&base_sent, &m.title);
            compute_divergence(m, &market_sent)
        })
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

/// Two-stage deduplication:
/// 1. URL-based: multi-outcome events share a URL — keep highest-probability outcome.
/// 2. Title-similarity: markets like "Will Trump nominate X as Fed chair?" are separate
///    events but essentially the same theme — keep only the highest-volume variant.
fn dedup_markets(markets: Vec<NormalizedMarket>) -> Vec<NormalizedMarket> {
    // Stage 1: URL-based dedup (same event, different outcomes)
    let mut best_by_url: HashMap<String, NormalizedMarket> = HashMap::new();
    for m in markets {
        let key = m.url.clone();
        let entry = best_by_url.entry(key);
        entry
            .and_modify(|existing| {
                if m.probability > existing.probability {
                    *existing = m.clone();
                }
            })
            .or_insert(m);
    }
    let mut after_url: Vec<NormalizedMarket> = best_by_url.into_values().collect();
    // Sort by volume descending so the highest-volume market wins in stage 2
    after_url.sort_by(|a, b| {
        b.volume_24h
            .unwrap_or(0.0)
            .partial_cmp(&a.volume_24h.unwrap_or(0.0))
            .unwrap()
    });

    // Stage 2: Template dedup (same question structure, different entity)
    // e.g., "Will X win the 2028 US Presidential Election?" are all variants
    // of the same template. Group by template, keep highest-volume per template.
    let mut best_by_template: HashMap<String, NormalizedMarket> = HashMap::new();
    for m in &after_url {
        if let Some(tmpl) = extract_template(&m.title) {
            let entry = best_by_template.entry(tmpl);
            entry
                .and_modify(|existing| {
                    let m_vol = m.volume_24h.unwrap_or(0.0);
                    let ex_vol = existing.volume_24h.unwrap_or(0.0);
                    if m_vol > ex_vol {
                        *existing = m.clone();
                    }
                })
                .or_insert(m.clone());
        }
    }
    // Collect: markets that matched a template use the best-per-template,
    // markets that didn't match any template pass through as-is.
    let template_winners: Vec<NormalizedMarket> = best_by_template.into_values().collect();
    let non_template: Vec<NormalizedMarket> = after_url
        .into_iter()
        .filter(|m| extract_template(&m.title).is_none())
        .collect();

    let mut combined: Vec<NormalizedMarket> = template_winners;
    combined.extend(non_template);
    combined.sort_by(|a, b| {
        b.volume_24h
            .unwrap_or(0.0)
            .partial_cmp(&a.volume_24h.unwrap_or(0.0))
            .unwrap()
    });

    // Stage 3: Jaro-Winkler fallback for any remaining similar titles
    const SIMILARITY_THRESHOLD: f64 = 0.80;
    let mut unique: Vec<NormalizedMarket> = Vec::new();
    for candidate in combined {
        let dominated = unique.iter().any(|existing| {
            title_similarity(&existing.title, &candidate.title) >= SIMILARITY_THRESHOLD
        });
        if !dominated {
            unique.push(candidate);
        }
    }
    unique
}

/// Extract a template from a market title by replacing the variable entity with "{}".
/// Returns Some(template) if the title matches a known pattern, None otherwise.
///
/// Examples:
///   "Will Elon Musk win the 2028 US Presidential Election?" -> "Will {} win the 2028 US Presidential Election?"
///   "Will OpenAI have the best AI model at the end of February 2026?" -> "Will {} have the best AI model at the end of {} 2026?"
fn extract_template(title: &str) -> Option<String> {
    let lower = title.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.len() < 5 {
        return None;
    }

    // Pattern: "Will <entity> <verb> the <rest>?"
    // The entity is 1-3 words between "will" and a common verb.
    if words[0] == "will" {
        let verbs = [
            "win", "have", "be", "get", "reach", "hit", "pass", "become", "nominate", "sign",
            "announce", "release",
        ];
        for (i, word) in words.iter().enumerate().skip(1) {
            if i > 4 {
                break; // entity shouldn't be more than 3 words
            }
            let clean = word.trim_end_matches('?');
            if verbs.contains(&clean) {
                // Everything after the entity is the template suffix
                let suffix: String = words[i..].join(" ");
                return Some(format!("will {{}} {}", suffix));
            }
        }
    }

    None
}
