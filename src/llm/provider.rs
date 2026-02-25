use anyhow::{Context, Result};
use serde::Deserialize;

use super::{QueryExpansion, RerankResponse, RankedResult};

/// Anthropic Claude provider for LLM-powered search features.
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("oddsense/0.1.0")
            .build()
            .expect("Failed to build HTTP client");
        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| "claude-haiku-4-5-20251001".to_string()),
        }
    }

    /// Expand a query into alternative phrasings using the LLM.
    pub async fn expand_query(&self, query: &str) -> Result<QueryExpansion> {
        let prompt = format!(
            "You are a prediction market search assistant. Given a user query, output JSON with:\n\
             - expanded_queries: 3-5 alternative phrasings that prediction markets might use\n\
             - category: one of [politics, economics, technology, crypto, sports, science, geopolitics, culture]\n\
             - intent: one sentence describing what the user wants\n\n\
             Query: \"{}\"\n\n\
             Respond with only valid JSON, no explanation.",
            query
        );

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 200,
            "messages": [{"role": "user", "content": prompt}]
        });

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to call Anthropic API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API returned {}: {}", status, text);
        }

        let api_resp: AnthropicResponse = resp
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        let text = api_resp
            .content
            .first()
            .and_then(|c| c.text.as_deref())
            .unwrap_or("{}");

        // Parse the JSON from the LLM response
        let expansion: QueryExpansion = serde_json::from_str(text).unwrap_or(QueryExpansion {
            original: query.to_string(),
            expanded_queries: vec![query.to_string()],
            category: None,
            intent: format!("Find prediction markets about {}", query),
        });

        Ok(QueryExpansion {
            original: query.to_string(),
            ..expansion
        })
    }

    /// Rerank search results by relevance using the LLM.
    pub async fn rerank_results(
        &self,
        query: &str,
        titles: &[String],
    ) -> Result<RerankResponse> {
        if titles.is_empty() {
            return Ok(RerankResponse { rankings: vec![] });
        }

        let numbered: String = titles
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{}. {}", i, t))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Rate the relevance (0.0-1.0) of each prediction market to the query \"{}\".\n\
             Markets:\n{}\n\n\
             Respond with a JSON array: [{{\"index\": N, \"relevance\": 0.X, \"category\": \"...\"}}]\n\
             Categories: politics, economics, technology, crypto, sports, science, geopolitics, culture\n\
             Only include markets with relevance > 0.2. Respond with only valid JSON.",
            query, numbered
        );

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 500,
            "messages": [{"role": "user", "content": prompt}]
        });

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to call Anthropic API for reranking")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API returned {}: {}", status, text);
        }

        let api_resp: AnthropicResponse = resp
            .json()
            .await
            .context("Failed to parse Anthropic rerank response")?;

        let text = api_resp
            .content
            .first()
            .and_then(|c| c.text.as_deref())
            .unwrap_or("[]");

        let rankings: Vec<RankedResult> = serde_json::from_str(text).unwrap_or_default();

        Ok(RerankResponse { rankings })
    }
}

/// Create an LLM provider from config, checking config file then environment variables.
pub fn create_provider(
    config_key: Option<&str>,
    provider_name: Option<&str>,
    model: Option<&str>,
) -> Option<AnthropicProvider> {
    // Priority: config file key > ANTHROPIC_API_KEY env var
    let api_key = config_key
        .filter(|k| !k.is_empty())
        .map(String::from)
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok());

    let api_key = api_key?;

    // For now, only Anthropic is supported
    let _ = provider_name; // future: support openai

    Some(AnthropicProvider::new(api_key, model.map(String::from)))
}
