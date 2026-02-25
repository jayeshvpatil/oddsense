use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::Deserialize;

use super::{scorer, SentimentResult, SignalItem};

const NEWSAPI_BASE: &str = "https://newsapi.org/v2";

#[derive(Debug, Deserialize)]
struct NewsApiResponse {
    status: String,
    #[serde(rename = "totalResults")]
    _total_results: Option<u32>,
    articles: Option<Vec<NewsArticle>>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NewsArticle {
    title: Option<String>,
    description: Option<String>,
    source: Option<NewsSource>,
    #[serde(rename = "publishedAt")]
    published_at: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NewsSource {
    name: Option<String>,
}

/// Fetch news sentiment for a query using NewsAPI.
pub async fn fetch_news_sentiment(
    query: &str,
    api_key: Option<&str>,
) -> Result<SentimentResult> {
    let key = match api_key {
        Some(k) if !k.is_empty() => k,
        _ => bail!(
            "NewsAPI key required. Get a free key at https://newsapi.org \
             and set it in ~/.config/vibe-dash/config.toml under [api_keys] newsapi = \"...\""
        ),
    };

    let client = Client::builder()
        .user_agent("vibe-dash/0.1.0")
        .build()?;

    let resp = client
        .get(format!("{}/everything", NEWSAPI_BASE))
        .query(&[
            ("q", query),
            ("sortBy", "publishedAt"),
            ("language", "en"),
            ("pageSize", "20"),
            ("apiKey", key),
        ])
        .send()
        .await
        .context("Failed to query NewsAPI")?;

    let data: NewsApiResponse = resp.json().await.context("Failed to parse NewsAPI response")?;

    if data.status != "ok" {
        bail!(
            "NewsAPI error: {}",
            data.message.unwrap_or_else(|| "unknown error".to_string())
        );
    }

    let articles = data.articles.unwrap_or_default();
    let mut signals = Vec::new();
    let mut total_score = 0.0;

    for article in &articles {
        let title = article.title.as_deref().unwrap_or("");
        let desc = article.description.as_deref().unwrap_or("");
        let sentiment = scorer::score_article(title, desc);

        total_score += sentiment;
        signals.push(SignalItem {
            title: title.to_string(),
            source_name: article
                .source
                .as_ref()
                .and_then(|s| s.name.as_deref())
                .unwrap_or("unknown")
                .to_string(),
            published_at: article.published_at.clone().unwrap_or_default(),
            sentiment,
            url: article.url.clone(),
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
        source: "news".to_string(),
        score: avg_score,
        confidence: (count / 20.0).min(1.0),
        signal_count: signals.len() as u32,
        sample_signals: signals,
        analyzed_at: chrono::Utc::now().to_rfc3339(),
    })
}
