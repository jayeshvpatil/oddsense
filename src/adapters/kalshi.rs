use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use super::schema::NormalizedMarket;
use super::MarketSource;

const BASE_URL: &str = "https://api.elections.kalshi.com/trade-api/v2";

/// Kalshi adapter using their public REST API (no auth for read-only).
pub struct KalshiAdapter {
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct EventsResponse {
    events: Vec<KalshiEvent>,
}

#[derive(Debug, Deserialize)]
struct KalshiEvent {
    event_ticker: String,
    title: String,
    category: Option<String>,
    #[serde(default)]
    markets: Vec<KalshiMarket>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct KalshiMarket {
    ticker: String,
    title: Option<String>,
    subtitle: Option<String>,
    yes_sub_title: Option<String>,
    yes_bid: Option<f64>,
    yes_ask: Option<f64>,
    last_price: Option<f64>,
    volume_24h: Option<f64>,
    volume: Option<f64>,
    close_time: Option<String>,
    status: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MarketsResponse {
    markets: Vec<KalshiMarket>,
}

impl KalshiAdapter {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("oddsense/0.1.0")
            .build()
            .expect("Failed to build HTTP client");
        Self { client }
    }

    /// Normalize a Kalshi market into our schema.
    fn normalize(event: &KalshiEvent, market: &KalshiMarket) -> NormalizedMarket {
        // Kalshi prices are in cents (0-100), convert to probability (0.0-1.0)
        let probability = market
            .yes_bid
            .or(market.last_price)
            .map(|p| p / 100.0)
            .unwrap_or(0.0);

        // Build a descriptive title: use market title (or event title as fallback),
        // and append yes_sub_title so multi-outcome events are distinguishable
        // (e.g. "Who will the next Pope be? — Pietro Parolin").
        let base_title = market
            .title
            .as_deref()
            .unwrap_or(&event.title);
        let title = match &market.yes_sub_title {
            Some(sub) if !sub.is_empty() => format!("{} — {}", base_title, sub),
            _ => base_title.to_string(),
        };

        NormalizedMarket {
            id: market.ticker.clone(),
            source: "kalshi".to_string(),
            title,
            description: market.subtitle.clone().unwrap_or_default(),
            probability,
            volume_24h: market.volume_24h,
            volume_total: market.volume,
            price_change_24h: None, // Kalshi API doesn't provide this directly
            end_date: market.close_time.clone(),
            category: event.category.clone(),
            url: format!(
                "https://kalshi.com/markets/{}",
                event.event_ticker.to_lowercase()
            ),
            source_data: Value::Null,
        }
    }
}

#[async_trait]
impl MarketSource for KalshiAdapter {
    fn name(&self) -> &str {
        "kalshi"
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<NormalizedMarket>> {
        let url = format!("{}/events", BASE_URL);
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("status", "open"),
                ("with_nested_markets", "true"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await
            .context("Failed to fetch Kalshi events")?;

        if !resp.status().is_success() {
            anyhow::bail!("Kalshi API returned {}", resp.status());
        }

        let data: EventsResponse = resp.json().await.context("Failed to parse Kalshi response")?;

        let query_lower = query.to_lowercase();
        let mut markets: Vec<NormalizedMarket> = data
            .events
            .iter()
            .filter(|e| e.title.to_lowercase().contains(&query_lower))
            .flat_map(|e| {
                e.markets
                    .iter()
                    .map(move |m| Self::normalize(e, m))
            })
            .collect();

        markets.truncate(limit);
        Ok(markets)
    }

    async fn list_top(&self, limit: usize, _sort_by: &str) -> Result<Vec<NormalizedMarket>> {
        let url = format!("{}/events", BASE_URL);
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("status", "open"),
                ("with_nested_markets", "true"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await
            .context("Failed to fetch Kalshi events")?;

        if !resp.status().is_success() {
            anyhow::bail!("Kalshi API returned {}", resp.status());
        }

        let data: EventsResponse = resp.json().await.context("Failed to parse Kalshi response")?;

        let mut markets: Vec<NormalizedMarket> = data
            .events
            .iter()
            .flat_map(|e| {
                e.markets
                    .iter()
                    .map(move |m| Self::normalize(e, m))
            })
            .collect();

        // Sort by volume descending
        markets.sort_by(|a, b| {
            b.volume_24h
                .unwrap_or(0.0)
                .partial_cmp(&a.volume_24h.unwrap_or(0.0))
                .unwrap()
        });
        markets.truncate(limit);
        Ok(markets)
    }

    async fn get_market(&self, ticker: &str) -> Result<NormalizedMarket> {
        let url = format!("{}/markets/{}", BASE_URL, ticker);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch Kalshi market")?;

        if !resp.status().is_success() {
            anyhow::bail!("Kalshi API returned {} for market {}", resp.status(), ticker);
        }

        let data: MarketsResponse = resp.json().await?;
        let market = data
            .markets
            .first()
            .context("No market found")?;

        // For single market fetch, we don't have the event context
        let dummy_event = KalshiEvent {
            event_ticker: ticker.to_string(),
            title: String::new(),
            category: None,
            markets: vec![],
        };

        Ok(Self::normalize(&dummy_event, market))
    }

    fn is_available(&self) -> bool {
        true // HTTP API, always "available" — errors happen at request time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_kalshi_market() {
        let event = KalshiEvent {
            event_ticker: "PRES-2028".to_string(),
            title: "2028 Presidential Election".to_string(),
            category: Some("politics".to_string()),
            markets: vec![],
        };
        let market = KalshiMarket {
            ticker: "PRES-2028-DEM".to_string(),
            title: Some("Will Democrats win the 2028 election?".to_string()),
            subtitle: Some("Binary market".to_string()),
            yes_sub_title: Some("Democrats".to_string()),
            yes_bid: Some(45.0),
            yes_ask: Some(47.0),
            last_price: Some(46.0),
            volume_24h: Some(50000.0),
            volume: Some(2000000.0),
            close_time: Some("2028-11-05T00:00:00Z".to_string()),
            status: Some("open".to_string()),
        };

        let normalized = KalshiAdapter::normalize(&event, &market);
        assert_eq!(normalized.source, "kalshi");
        assert_eq!(normalized.id, "PRES-2028-DEM");
        assert_eq!(
            normalized.title,
            "Will Democrats win the 2028 election? — Democrats"
        );
        assert!((normalized.probability - 0.45).abs() < f64::EPSILON);
        assert_eq!(normalized.volume_24h, Some(50000.0));
        assert_eq!(normalized.category, Some("politics".to_string()));
    }

    #[test]
    fn test_normalize_fallback_to_last_price() {
        let event = KalshiEvent {
            event_ticker: "TEST".to_string(),
            title: "Test Event".to_string(),
            category: None,
            markets: vec![],
        };
        let market = KalshiMarket {
            ticker: "TEST-MKT".to_string(),
            title: None,
            subtitle: None,
            yes_sub_title: None,
            yes_bid: None,
            yes_ask: None,
            last_price: Some(70.0),
            volume_24h: None,
            volume: None,
            close_time: None,
            status: None,
        };

        let normalized = KalshiAdapter::normalize(&event, &market);
        assert_eq!(normalized.title, "Test Event"); // falls back to event title
        assert!((normalized.probability - 0.70).abs() < f64::EPSILON);
    }
}
