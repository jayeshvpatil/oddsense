pub mod provider;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExpansion {
    pub original: String,
    pub expanded_queries: Vec<String>,
    pub category: Option<String>,
    pub intent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedResult {
    pub index: usize,
    pub relevance: f64,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResponse {
    pub rankings: Vec<RankedResult>,
}
