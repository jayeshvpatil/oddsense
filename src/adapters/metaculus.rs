use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use super::schema::NormalizedMarket;
use super::MarketSource;

const BASE_URL: &str = "https://www.metaculus.com/api2";

/// Metaculus adapter using their public API.
/// Note: As of 2026-02, the API may require authentication.
/// This adapter degrades gracefully — returns empty results if unavailable.
pub struct MetaculusAdapter {
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct MetaculusResponse {
    results: Vec<MetaculusQuestion>,
}

#[derive(Debug, Deserialize)]
struct MetaculusQuestion {
    id: i64,
    title: String,
    #[serde(default)]
    description: Option<String>,
    url: Option<String>,
    community_prediction: Option<CommunityPrediction>,
    nr_forecasters: Option<i64>,
    resolve_time: Option<String>,
    #[serde(default)]
    group: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CommunityPrediction {
    full: Option<PredictionValue>,
}

#[derive(Debug, Deserialize)]
struct PredictionValue {
    q2: Option<f64>, // median prediction
}

impl MetaculusAdapter {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("vibe-dash/0.1.0")
            .build()
            .expect("Failed to build HTTP client");
        Self { client }
    }

    fn normalize(q: &MetaculusQuestion) -> NormalizedMarket {
        let probability = q
            .community_prediction
            .as_ref()
            .and_then(|cp| cp.full.as_ref())
            .and_then(|f| f.q2)
            .unwrap_or(0.0);

        NormalizedMarket {
            id: q.id.to_string(),
            source: "metaculus".to_string(),
            title: q.title.clone(),
            description: q.description.clone().unwrap_or_default(),
            probability,
            volume_24h: None,                // Metaculus doesn't have volume
            volume_total: q.nr_forecasters.map(|n| n as f64), // use forecaster count as proxy
            price_change_24h: None,
            end_date: q.resolve_time.clone(),
            category: q.group.clone(),
            url: q
                .url
                .clone()
                .unwrap_or_else(|| format!("https://www.metaculus.com/questions/{}/", q.id)),
            source_data: Value::Null,
        }
    }

    async fn fetch_questions(
        &self,
        params: &[(&str, &str)],
    ) -> Result<Vec<MetaculusQuestion>> {
        let url = format!("{}/questions/", BASE_URL);
        let resp = self
            .client
            .get(&url)
            .query(params)
            .send()
            .await
            .context("Failed to fetch Metaculus questions")?;

        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            // Metaculus API requires auth — degrade gracefully
            eprintln!("  [metaculus] API requires authentication, skipping");
            return Ok(vec![]);
        }

        if !resp.status().is_success() {
            anyhow::bail!("Metaculus API returned {}", resp.status());
        }

        let data: MetaculusResponse =
            resp.json().await.context("Failed to parse Metaculus response")?;
        Ok(data.results)
    }
}

#[async_trait]
impl MarketSource for MetaculusAdapter {
    fn name(&self) -> &str {
        "metaculus"
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<NormalizedMarket>> {
        let limit_str = limit.to_string();
        let questions = self
            .fetch_questions(&[
                ("search", query),
                ("limit", &limit_str),
                ("status", "open"),
                ("type", "forecast"),
            ])
            .await?;

        let markets: Vec<NormalizedMarket> =
            questions.iter().map(Self::normalize).collect();
        Ok(markets)
    }

    async fn list_top(&self, limit: usize, _sort_by: &str) -> Result<Vec<NormalizedMarket>> {
        let limit_str = limit.to_string();
        let questions = self
            .fetch_questions(&[
                ("limit", &limit_str),
                ("status", "open"),
                ("type", "forecast"),
                ("order_by", "-activity"),
            ])
            .await?;

        let markets: Vec<NormalizedMarket> =
            questions.iter().map(Self::normalize).collect();
        Ok(markets)
    }

    async fn get_market(&self, id: &str) -> Result<NormalizedMarket> {
        let url = format!("{}/questions/{}/", BASE_URL, id);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch Metaculus question")?;

        if !resp.status().is_success() {
            anyhow::bail!("Metaculus API returned {} for question {}", resp.status(), id);
        }

        let q: MetaculusQuestion =
            resp.json().await.context("Failed to parse Metaculus question")?;
        Ok(Self::normalize(&q))
    }

    fn is_available(&self) -> bool {
        true // HTTP API — errors handled at request time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_metaculus() {
        let q = MetaculusQuestion {
            id: 12345,
            title: "Will AGI be achieved by 2030?".to_string(),
            description: Some("A question about AGI timelines".to_string()),
            url: Some("https://www.metaculus.com/questions/12345/".to_string()),
            community_prediction: Some(CommunityPrediction {
                full: Some(PredictionValue { q2: Some(0.25) }),
            }),
            nr_forecasters: Some(500),
            resolve_time: Some("2030-12-31T00:00:00Z".to_string()),
            group: Some("AI".to_string()),
        };

        let m = MetaculusAdapter::normalize(&q);
        assert_eq!(m.source, "metaculus");
        assert_eq!(m.id, "12345");
        assert!((m.probability - 0.25).abs() < f64::EPSILON);
        assert_eq!(m.volume_total, Some(500.0)); // forecaster count
        assert_eq!(m.volume_24h, None);
    }

    #[test]
    fn test_normalize_missing_prediction() {
        let q = MetaculusQuestion {
            id: 99,
            title: "Test question".to_string(),
            description: None,
            url: None,
            community_prediction: None,
            nr_forecasters: None,
            resolve_time: None,
            group: None,
        };

        let m = MetaculusAdapter::normalize(&q);
        assert!((m.probability - 0.0).abs() < f64::EPSILON);
        assert_eq!(m.url, "https://www.metaculus.com/questions/99/");
    }
}
