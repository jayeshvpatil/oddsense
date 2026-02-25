use serde::{Deserialize, Serialize};
use serde_json::Value;

fn is_null_or_empty(v: &Value) -> bool {
    v.is_null() || (v.is_object() && v.as_object().map_or(false, |o| o.is_empty()))
}

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
    /// Skipped in JSON output when empty/null to reduce bloat.
    #[serde(skip_serializing_if = "is_null_or_empty")]
    pub source_data: Value,
}

impl NormalizedMarket {
    /// Returns true if this market's end date has already passed.
    pub fn is_expired(&self) -> bool {
        if let Some(ref end) = self.end_date {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(end) {
                return dt < chrono::Utc::now();
            }
            // Try parsing date-only formats like "2025-12-31"
            if let Ok(date) = chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d") {
                let end_dt = date
                    .and_hms_opt(23, 59, 59)
                    .map(|ndt| ndt.and_utc());
                if let Some(end_dt) = end_dt {
                    return end_dt < chrono::Utc::now();
                }
            }
        }
        false
    }
}

/// Wrapper for JSON output with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketResponse {
    pub count: usize,
    pub query: Option<String>,
    pub source: String,
    pub markets: Vec<NormalizedMarket>,
}
