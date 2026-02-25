use serde::{Deserialize, Serialize};

use crate::adapters::schema::NormalizedMarket;
use crate::sentiment::SentimentResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Divergence {
    pub market: NormalizedMarket,
    pub sentiment: SentimentResult,
    pub divergence_score: f64,
    pub direction: DivergenceDirection,
    pub signal_strength: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DivergenceDirection {
    MarketHigher,
    SentimentHigher,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceResponse {
    pub count: usize,
    pub query: String,
    pub divergences: Vec<Divergence>,
}

/// Compute divergence between market probability and sentiment score.
///
/// Algorithm:
///   1. Normalize sentiment (-1..1) to probability-like scale (0..1)
///   2. Raw divergence = abs(market_prob - sentiment_prob)
///   3. Weight by sentiment confidence
///   4. Scale to 0-100
pub fn compute_divergence(market: &NormalizedMarket, sentiment: &SentimentResult) -> Divergence {
    // Step 1: sentiment [-1,1] → probability [0,1]
    let sentiment_prob = (sentiment.score + 1.0) / 2.0;

    // Step 2: raw difference
    let raw_diff = (market.probability - sentiment_prob).abs();

    // Step 3: weight by confidence
    let weighted_diff = raw_diff * sentiment.confidence;

    // Step 4: scale to 0-100
    let divergence_score = (weighted_diff * 100.0).min(100.0);

    // Step 5: direction
    let direction = if market.probability > sentiment_prob {
        DivergenceDirection::MarketHigher
    } else {
        DivergenceDirection::SentimentHigher
    };

    // Step 6: signal strength
    let signal_strength = match divergence_score {
        s if s >= 50.0 => "strong",
        s if s >= 25.0 => "moderate",
        _ => "weak",
    }
    .to_string();

    let dir_label = match &direction {
        DivergenceDirection::MarketHigher => "overpriced by market",
        DivergenceDirection::SentimentHigher => "underpriced by market",
    };

    let summary = format!(
        "\"{}\" — Market: {:.0}% | Sentiment: {:.0}% ({}) | Divergence: {:.0}/100 ({}) — {}",
        market.title,
        market.probability * 100.0,
        sentiment_prob * 100.0,
        if sentiment.score > 0.0 {
            "bullish"
        } else if sentiment.score < 0.0 {
            "bearish"
        } else {
            "neutral"
        },
        divergence_score,
        signal_strength,
        dir_label,
    );

    Divergence {
        market: market.clone(),
        sentiment: sentiment.clone(),
        divergence_score,
        direction,
        signal_strength,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_market(prob: f64) -> NormalizedMarket {
        NormalizedMarket {
            id: "test".to_string(),
            source: "polymarket".to_string(),
            title: "Test market".to_string(),
            description: String::new(),
            probability: prob,
            volume_24h: None,
            volume_total: None,
            price_change_24h: None,
            end_date: None,
            category: None,
            url: String::new(),
            source_data: serde_json::Value::Null,
        }
    }

    fn make_sentiment(score: f64, confidence: f64) -> SentimentResult {
        SentimentResult {
            query: "test".to_string(),
            source: "test".to_string(),
            score,
            confidence,
            signal_count: 10,
            sample_signals: vec![],
            analyzed_at: String::new(),
        }
    }

    #[test]
    fn test_high_divergence() {
        // Market says 80%, sentiment is very bearish (-0.8 → prob 0.1)
        let d = compute_divergence(&make_market(0.8), &make_sentiment(-0.8, 1.0));
        assert!(d.divergence_score > 50.0);
        assert_eq!(d.signal_strength, "strong");
        assert!(matches!(d.direction, DivergenceDirection::MarketHigher));
    }

    #[test]
    fn test_low_divergence() {
        // Market says 50%, sentiment neutral (0.0 → prob 0.5)
        let d = compute_divergence(&make_market(0.5), &make_sentiment(0.0, 1.0));
        assert!(d.divergence_score < 5.0);
        assert_eq!(d.signal_strength, "weak");
    }

    #[test]
    fn test_confidence_weighting() {
        // Same raw divergence, but low confidence should reduce score
        let high_conf = compute_divergence(&make_market(0.8), &make_sentiment(-0.5, 1.0));
        let low_conf = compute_divergence(&make_market(0.8), &make_sentiment(-0.5, 0.2));
        assert!(high_conf.divergence_score > low_conf.divergence_score);
    }

    #[test]
    fn test_sentiment_higher() {
        // Market says 20%, sentiment very bullish (0.8 → prob 0.9)
        let d = compute_divergence(&make_market(0.2), &make_sentiment(0.8, 1.0));
        assert!(matches!(d.direction, DivergenceDirection::SentimentHigher));
    }
}
