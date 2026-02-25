use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

use super::schema::NormalizedMarket;
use super::MarketSource;

/// Polymarket adapter that shells out to `polymarket-cli`.
pub struct PolymarketAdapter {
    binary: String,
}

impl PolymarketAdapter {
    pub fn new() -> Self {
        Self {
            binary: "polymarket".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn with_binary(path: String) -> Self {
        Self { binary: path }
    }

    /// Run polymarket-cli with given args and parse JSON output.
    async fn run_cli(&self, args: &[&str]) -> Result<Vec<Value>> {
        let output = Command::new(&self.binary)
            .args(&["-o", "json"])
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context(
                "Failed to run polymarket-cli. Is it installed?\n\
                 Install: cargo install --git https://github.com/Polymarket/polymarket-cli.git",
            )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("polymarket-cli error: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Ok(vec![]);
        }

        let parsed: Value = serde_json::from_str(stdout.trim())
            .context("Failed to parse polymarket-cli JSON output")?;

        match parsed {
            Value::Array(arr) => Ok(arr),
            single => Ok(vec![single]),
        }
    }

    /// Convert a raw polymarket-cli JSON market object into our normalized schema.
    fn normalize(raw: &Value) -> NormalizedMarket {
        let question = raw["question"].as_str().unwrap_or("").to_string();
        let slug = raw["slug"].as_str().unwrap_or("");

        NormalizedMarket {
            id: raw["id"]
                .as_str()
                .or_else(|| raw["id"].as_u64().map(|_| ""))
                .unwrap_or("")
                .to_string(),
            source: "polymarket".to_string(),
            title: question,
            description: raw["description"].as_str().unwrap_or("").to_string(),
            probability: parse_probability(raw),
            volume_24h: parse_string_f64(&raw["volume24hr"]),
            volume_total: parse_string_f64(&raw["volume"])
                .or_else(|| parse_string_f64(&raw["volumeNum"])),
            price_change_24h: parse_string_f64(&raw["oneDayPriceChange"]),
            end_date: raw["endDate"].as_str().map(String::from),
            category: raw["category"].as_str().map(String::from),
            url: format!("https://polymarket.com/event/{}", slug),
            source_data: Value::Null,
        }
    }
}

/// Parse the "Yes" outcome probability from polymarket's outcomePrices field.
/// Format: "[\"0.85\",\"0.15\"]" — index 0 is the Yes price (0.0 to 1.0).
fn parse_probability(raw: &Value) -> f64 {
    if let Some(prices_str) = raw["outcomePrices"].as_str() {
        if let Ok(prices) = serde_json::from_str::<Vec<String>>(prices_str) {
            if let Some(yes_price) = prices.first() {
                return yes_price.parse::<f64>().unwrap_or(0.0);
            }
        }
    }
    // Fallback: try lastTradePrice
    parse_string_f64(&raw["lastTradePrice"]).unwrap_or(0.0)
}

/// Parse a Value that may be a string-encoded number or a raw number.
fn parse_string_f64(val: &Value) -> Option<f64> {
    match val {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

#[async_trait]
impl MarketSource for PolymarketAdapter {
    fn name(&self) -> &str {
        "polymarket"
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<NormalizedMarket>> {
        let limit_str = limit.to_string();
        let raw = self
            .run_cli(&["markets", "search", query, "--limit", &limit_str])
            .await?;
        let mut markets: Vec<NormalizedMarket> = raw.iter().map(Self::normalize).collect();
        markets.truncate(limit);
        Ok(markets)
    }

    async fn list_top(&self, limit: usize, _sort_by: &str) -> Result<Vec<NormalizedMarket>> {
        let limit_str = limit.to_string();
        let raw = self
            .run_cli(&[
                "markets",
                "list",
                "--limit",
                &limit_str,
                "--active",
                "true",
                "--closed",
                "false",
            ])
            .await?;

        // Sort by 24h volume descending on our side since the API ordering is unreliable
        let mut markets: Vec<NormalizedMarket> = raw.iter().map(Self::normalize).collect();
        markets.sort_by(|a, b| {
            b.volume_24h
                .unwrap_or(0.0)
                .partial_cmp(&a.volume_24h.unwrap_or(0.0))
                .unwrap()
        });
        markets.truncate(limit);
        Ok(markets)
    }

    async fn get_market(&self, id: &str) -> Result<NormalizedMarket> {
        let raw = self.run_cli(&["markets", "get", id]).await?;
        raw.first()
            .map(Self::normalize)
            .context("No market returned")
    }

    fn is_available(&self) -> bool {
        std::process::Command::new(&self.binary)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_probability() {
        let raw = serde_json::json!({
            "outcomePrices": "[\"0.85\",\"0.15\"]"
        });
        assert!((parse_probability(&raw) - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_probability_missing() {
        let raw = serde_json::json!({});
        assert!((parse_probability(&raw) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_string_f64() {
        assert_eq!(
            parse_string_f64(&Value::String("1234.56".to_string())),
            Some(1234.56)
        );
        assert_eq!(parse_string_f64(&serde_json::json!(42.0)), Some(42.0));
        assert_eq!(parse_string_f64(&Value::Null), None);
    }

    #[test]
    fn test_normalize_market() {
        let raw = serde_json::json!({
            "id": "12345",
            "question": "Will Bitcoin hit 200k?",
            "description": "Test market",
            "slug": "bitcoin-200k",
            "outcomePrices": "[\"0.35\",\"0.65\"]",
            "volume": "5000000.50",
            "volume24hr": "100000.25",
            "oneDayPriceChange": "0.05",
            "endDate": "2026-12-31T00:00:00Z",
            "category": "crypto"
        });
        let m = PolymarketAdapter::normalize(&raw);
        assert_eq!(m.id, "12345");
        assert_eq!(m.title, "Will Bitcoin hit 200k?");
        assert!((m.probability - 0.35).abs() < f64::EPSILON);
        assert_eq!(m.volume_24h, Some(100000.25));
        assert_eq!(m.volume_total, Some(5000000.50));
        assert_eq!(m.price_change_24h, Some(0.05));
        assert_eq!(m.url, "https://polymarket.com/event/bitcoin-200k");
    }
}
