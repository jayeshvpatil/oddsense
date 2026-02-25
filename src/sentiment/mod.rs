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
