use serde::{Deserialize, Serialize};
use strsim::jaro_winkler;

use crate::adapters::schema::NormalizedMarket;

/// A reference to a specific market within an arbitrage opportunity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketRef {
    pub source: String,
    pub id: String,
    pub title: String,
    pub probability: f64,
    pub url: String,
}

impl From<&NormalizedMarket> for MarketRef {
    fn from(m: &NormalizedMarket) -> Self {
        Self {
            source: m.source.clone(),
            id: m.id.clone(),
            title: m.title.clone(),
            probability: m.probability,
            url: m.url.clone(),
        }
    }
}

/// An arbitrage opportunity: the same (or very similar) question priced
/// differently across prediction market platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub topic: String,
    pub markets: Vec<MarketRef>,
    pub spread: f64,
    pub highest: MarketRef,
    pub lowest: MarketRef,
    pub similarity: f64,
    pub summary: String,
}

/// Response wrapper for arbitrage output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageResponse {
    pub count: usize,
    pub query: Option<String>,
    pub min_spread: f64,
    pub opportunities: Vec<ArbitrageOpportunity>,
}

/// Response wrapper for the compare command (side-by-side view).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareResponse {
    pub query: String,
    pub sources: Vec<String>,
    pub markets: Vec<NormalizedMarket>,
}

/// Check if two market titles are similar enough to be considered the same question.
/// Uses Jaro-Winkler similarity (0.0 to 1.0) with a configurable threshold.
pub fn titles_match(a: &str, b: &str, threshold: f64) -> bool {
    let sim = title_similarity(a, b);
    sim >= threshold
}

/// Calculate similarity between two market titles.
/// Normalizes titles before comparison (lowercase, strip common prefixes).
pub fn title_similarity(a: &str, b: &str) -> f64 {
    let a_norm = normalize_title(a);
    let b_norm = normalize_title(b);
    jaro_winkler(&a_norm, &b_norm)
}

/// Normalize a title for fuzzy matching:
/// - lowercase
/// - strip "will ", "will the ", etc.
/// - strip trailing "?"
/// - collapse whitespace
fn normalize_title(s: &str) -> String {
    let lower = s.trim().to_lowercase();
    let stripped = lower
        .strip_prefix("will the ")
        .or_else(|| lower.strip_prefix("will "))
        .unwrap_or(&lower);
    let stripped = stripped.strip_suffix('?').unwrap_or(stripped);
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Find arbitrage opportunities across multiple market sources.
///
/// Algorithm:
///   1. Group markets from different sources by fuzzy title matching
///   2. For each group with 2+ sources, compute the spread
///   3. Filter by min_spread and sort by spread descending
pub fn find_arbitrage(
    all_markets: &[NormalizedMarket],
    min_spread: f64,
    similarity_threshold: f64,
) -> Vec<ArbitrageOpportunity> {
    // Group markets by matching titles across different sources
    let mut groups: Vec<Vec<&NormalizedMarket>> = Vec::new();

    for market in all_markets {
        let mut matched = false;
        for group in &mut groups {
            // Check if this market matches any existing group member from a different source
            if let Some(representative) = group.first() {
                // Don't match markets from the same source
                if representative.source == market.source {
                    continue;
                }
                if titles_match(
                    &representative.title,
                    &market.title,
                    similarity_threshold,
                ) {
                    group.push(market);
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            groups.push(vec![market]);
        }
    }

    // Convert groups with 2+ sources into arbitrage opportunities
    let mut opportunities: Vec<ArbitrageOpportunity> = groups
        .into_iter()
        .filter(|g| {
            // Must have markets from at least 2 different sources
            let sources: std::collections::HashSet<&str> =
                g.iter().map(|m| m.source.as_str()).collect();
            sources.len() >= 2
        })
        .filter_map(|group| {
            let highest = group
                .iter()
                .max_by(|a, b| a.probability.partial_cmp(&b.probability).unwrap())?;
            let lowest = group
                .iter()
                .min_by(|a, b| a.probability.partial_cmp(&b.probability).unwrap())?;

            let spread = (highest.probability - lowest.probability) * 100.0; // as percentage

            if spread < min_spread {
                return None;
            }

            let topic = group.first()?.title.clone();
            let similarity = title_similarity(&highest.title, &lowest.title);

            let markets: Vec<MarketRef> = group.iter().map(|m| MarketRef::from(*m)).collect();

            let summary = format!(
                "\"{}\" — {} {:.0}% vs {} {:.0}% — spread {:.1}pp",
                topic,
                highest.source,
                highest.probability * 100.0,
                lowest.source,
                lowest.probability * 100.0,
                spread,
            );

            Some(ArbitrageOpportunity {
                topic,
                markets,
                spread,
                highest: MarketRef::from(*highest),
                lowest: MarketRef::from(*lowest),
                similarity,
                summary,
            })
        })
        .collect();

    opportunities.sort_by(|a, b| b.spread.partial_cmp(&a.spread).unwrap());
    opportunities
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn make_market(source: &str, title: &str, prob: f64) -> NormalizedMarket {
        NormalizedMarket {
            id: format!("{}-{}", source, title.len()),
            source: source.to_string(),
            title: title.to_string(),
            description: String::new(),
            probability: prob,
            volume_24h: None,
            volume_total: None,
            price_change_24h: None,
            end_date: None,
            category: None,
            url: String::new(),
            source_data: Value::Null,
        }
    }

    #[test]
    fn test_title_similarity_identical() {
        let sim = title_similarity(
            "Will Bitcoin hit $200k by 2027?",
            "Will Bitcoin hit $200k by 2027?",
        );
        assert!((sim - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_title_similarity_close() {
        let sim = title_similarity(
            "Will Bitcoin reach $200k by end of 2027?",
            "Will Bitcoin hit $200k by 2027?",
        );
        assert!(sim > 0.7, "Expected similarity > 0.7, got {}", sim);
    }

    #[test]
    fn test_title_similarity_different() {
        let sim = title_similarity(
            "Will the Democrats win in 2028?",
            "Will Bitcoin reach $200k?",
        );
        assert!(sim < 0.7, "Expected similarity < 0.7, got {}", sim);
    }

    #[test]
    fn test_normalize_title() {
        assert_eq!(normalize_title("Will the US default?"), "us default");
        assert_eq!(normalize_title("Will Bitcoin hit 200k?"), "bitcoin hit 200k");
        assert_eq!(
            normalize_title("  Will  GPT-5  release  in  2026?  "),
            "gpt-5 release in 2026"
        );
    }

    #[test]
    fn test_find_arbitrage_basic() {
        let markets = vec![
            make_market("polymarket", "Will Bitcoin hit $200k by 2027?", 0.35),
            make_market("kalshi", "Will Bitcoin hit $200k by 2027?", 0.50),
        ];

        let opps = find_arbitrage(&markets, 5.0, 0.7);
        assert_eq!(opps.len(), 1);
        assert!((opps[0].spread - 15.0).abs() < 0.1);
        assert_eq!(opps[0].highest.source, "kalshi");
        assert_eq!(opps[0].lowest.source, "polymarket");
    }

    #[test]
    fn test_find_arbitrage_below_threshold() {
        let markets = vec![
            make_market("polymarket", "Will Bitcoin hit $200k?", 0.35),
            make_market("kalshi", "Will Bitcoin hit $200k?", 0.36),
        ];

        let opps = find_arbitrage(&markets, 5.0, 0.7);
        assert_eq!(opps.len(), 0); // spread is only 1pp
    }

    #[test]
    fn test_find_arbitrage_same_source_ignored() {
        let markets = vec![
            make_market("polymarket", "Will Bitcoin hit $200k?", 0.35),
            make_market("polymarket", "Will Bitcoin hit $200k?", 0.50),
        ];

        let opps = find_arbitrage(&markets, 5.0, 0.7);
        assert_eq!(opps.len(), 0); // same source, not cross-platform
    }

    #[test]
    fn test_find_arbitrage_three_sources() {
        let markets = vec![
            make_market("polymarket", "Will Bitcoin hit $200k by 2027?", 0.35),
            make_market("kalshi", "Will Bitcoin hit $200k by 2027?", 0.50),
            make_market("metaculus", "Will Bitcoin hit $200k by 2027?", 0.20),
        ];

        let opps = find_arbitrage(&markets, 5.0, 0.7);
        assert_eq!(opps.len(), 1);
        assert!((opps[0].spread - 30.0).abs() < 0.1); // 50% - 20% = 30pp
        assert_eq!(opps[0].highest.source, "kalshi");
        assert_eq!(opps[0].lowest.source, "metaculus");
    }
}
