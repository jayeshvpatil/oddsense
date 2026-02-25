use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

use super::{scorer, SentimentResult, SignalItem};

#[derive(Debug, Deserialize)]
struct RedditListing {
    data: RedditListingData,
}

#[derive(Debug, Deserialize)]
struct RedditListingData {
    children: Vec<RedditChild>,
}

#[derive(Debug, Deserialize)]
struct RedditChild {
    data: RedditPost,
}

#[derive(Debug, Deserialize)]
struct RedditPost {
    title: String,
    selftext: Option<String>,
    subreddit: Option<String>,
    created_utc: Option<f64>,
    permalink: Option<String>,
    _score: Option<i64>,
}

/// Fetch Reddit sentiment for a query using the public JSON API.
pub async fn fetch_reddit_sentiment(query: &str) -> Result<SentimentResult> {
    let client = Client::builder()
        .user_agent("oddsense/0.1.0 (prediction market intelligence)")
        .build()?;

    let resp = client
        .get("https://www.reddit.com/search.json")
        .query(&[
            ("q", query),
            ("sort", "new"),
            ("limit", "25"),
            ("t", "week"),
        ])
        .send()
        .await
        .context("Failed to query Reddit")?;

    if !resp.status().is_success() {
        anyhow::bail!("Reddit API returned status {}", resp.status());
    }

    let listing: RedditListing = resp
        .json()
        .await
        .context("Failed to parse Reddit response")?;

    let mut signals = Vec::new();
    let mut total_score = 0.0;

    for child in &listing.data.children {
        let post = &child.data;
        let desc = post.selftext.as_deref().unwrap_or("");
        let sentiment = scorer::score_article(&post.title, desc);

        total_score += sentiment;

        let published_at = post
            .created_utc
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts as i64, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let url = post
            .permalink
            .as_deref()
            .map(|p| format!("https://reddit.com{}", p));

        signals.push(SignalItem {
            title: post.title.clone(),
            source_name: format!("r/{}", post.subreddit.as_deref().unwrap_or("unknown")),
            published_at,
            sentiment,
            url,
        });
    }

    let count = signals.len() as f64;
    let avg_score = if count > 0.0 {
        total_score / count
    } else {
        0.0
    };

    Ok(SentimentResult {
        query: query.to_string(),
        source: "reddit".to_string(),
        score: avg_score,
        confidence: (count / 25.0).min(1.0),
        signal_count: signals.len() as u32,
        sample_signals: signals,
        analyzed_at: chrono::Utc::now().to_rfc3339(),
    })
}
