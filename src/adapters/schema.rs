use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Normalized market data across all prediction market sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMarket {
    pub id: String,
    pub source: String,
    pub title: String,
    pub description: String,
    pub probability: f64,
    pub volume_24h: Option<f64>,
    pub volume_total: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub end_date: Option<String>,
    pub category: Option<String>,
    pub url: String,
    /// Raw JSON from the source, preserved for passthrough.
    pub source_data: Value,
}

/// Wrapper for JSON output with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketResponse {
    pub count: usize,
    pub query: Option<String>,
    pub source: String,
    pub markets: Vec<NormalizedMarket>,
}
