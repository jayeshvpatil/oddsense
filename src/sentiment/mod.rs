pub mod news;
pub mod reddit;
pub mod scorer;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentResult {
    pub query: String,
    pub source: String,
    pub score: f64,
    pub confidence: f64,
    pub signal_count: u32,
    pub sample_signals: Vec<SignalItem>,
    pub analyzed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalItem {
    pub title: String,
    pub source_name: String,
    pub published_at: String,
    pub sentiment: f64,
    pub url: Option<String>,
}

/// Re-score existing sentiment signals for a specific market title.
/// Instead of calling APIs again, we filter/weight the already-fetched signals
/// by relevance to the market title, giving each market a distinct score.
pub fn rescore_for_market(base: &SentimentResult, market_title: &str) -> SentimentResult {
    let title_words: Vec<String> = extract_keywords(market_title);

    if title_words.is_empty() || base.sample_signals.is_empty() {
        return base.clone();
    }

    // Score each signal by keyword overlap with the market title
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;

    for signal in &base.sample_signals {
        let signal_text = signal.title.to_lowercase();
        let overlap = title_words
            .iter()
            .filter(|w| signal_text.contains(w.as_str()))
            .count();

        // Weight: base relevance (0.1) + keyword overlap bonus
        let relevance = 0.1 + (overlap as f64 * 0.3);
        weighted_sum += signal.sentiment * relevance;
        weight_total += relevance;
    }

    let score = if weight_total > 0.0 {
        (weighted_sum / weight_total).clamp(-1.0, 1.0)
    } else {
        base.score
    };

    // Confidence scales with how many signals were relevant
    let relevant_count = base
        .sample_signals
        .iter()
        .filter(|s| {
            let st = s.title.to_lowercase();
            title_words.iter().any(|w| st.contains(w.as_str()))
        })
        .count();
    let confidence = if relevant_count > 0 {
        (relevant_count as f64 / 10.0).min(1.0) * base.confidence
    } else {
        base.confidence * 0.5 // low confidence if no direct matches
    };

    SentimentResult {
        query: market_title.to_string(),
        source: base.source.clone(),
        score,
        confidence,
        signal_count: base.signal_count,
        sample_signals: base.sample_signals.clone(),
        analyzed_at: base.analyzed_at.clone(),
    }
}

/// Extract meaningful keywords from a market title (skip stop words).
fn extract_keywords(title: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "will", "the", "a", "an", "in", "of", "at", "to", "for", "by", "on", "is", "be",
        "and", "or", "not", "than", "as", "this", "that", "with", "from", "its", "it",
        "more", "less", "before", "after", "during", "next", "new", "win", "have", "has",
        "does", "do", "been", "was", "were", "are", "about",
    ];
    title
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2 && !STOP_WORDS.contains(w))
        .map(String::from)
        .collect()
}

/// Aggregate sentiment from multiple sources.
pub async fn aggregate_sentiment(
    query: &str,
    use_news: bool,
    use_reddit: bool,
    newsapi_key: Option<&str>,
) -> Result<SentimentResult> {
    let mut all_signals: Vec<SignalItem> = Vec::new();
    let mut weighted_score = 0.0;
    let mut total_weight = 0.0;

    if use_news {
        match news::fetch_news_sentiment(query, newsapi_key).await {
            Ok(result) => {
                let weight = 0.6;
                weighted_score += result.score * weight;
                total_weight += weight;
                all_signals.extend(result.sample_signals);
            }
            Err(e) => {
                eprintln!("Warning: news sentiment unavailable: {}", e);
            }
        }
    }

    if use_reddit {
        match reddit::fetch_reddit_sentiment(query).await {
            Ok(result) => {
                let weight = 0.4;
                weighted_score += result.score * weight;
                total_weight += weight;
                all_signals.extend(result.sample_signals);
            }
            Err(e) => {
                eprintln!("Warning: reddit sentiment unavailable: {}", e);
            }
        }
    }

    let final_score = if total_weight > 0.0 {
        weighted_score / total_weight
    } else {
        0.0
    };

    let confidence = if all_signals.is_empty() {
        0.0
    } else {
        (total_weight * (all_signals.len() as f64).min(20.0) / 20.0).min(1.0)
    };

    Ok(SentimentResult {
        query: query.to_string(),
        source: "aggregate".to_string(),
        score: final_score,
        confidence,
        signal_count: all_signals.len() as u32,
        sample_signals: all_signals,
        analyzed_at: chrono::Utc::now().to_rfc3339(),
    })
}
