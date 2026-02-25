/// Simple keyword-based sentiment scorer for market-relevant text.
/// Deliberately simple — a fast heuristic beats a slow ML model in a CLI.

const POSITIVE_WORDS: &[&str] = &[
    "approved", "passed", "surged", "confirmed", "deal", "breakthrough", "wins",
    "bullish", "soars", "gains", "rally", "record", "high", "growth", "boost",
    "success", "agreement", "progress", "optimistic", "upgrade", "positive",
    "rising", "accelerate", "strong", "outperform", "exceed", "launch", "adopt",
    "support", "accept", "increase", "expand", "advance", "achieve", "milestone",
];

const NEGATIVE_WORDS: &[&str] = &[
    "rejected", "failed", "crashed", "blocked", "scandal", "collapses", "loses",
    "bearish", "plunges", "decline", "slump", "low", "crisis", "risk", "fear",
    "failure", "dispute", "pessimistic", "downgrade", "negative", "falling",
    "decelerate", "weak", "underperform", "miss", "delay", "oppose", "ban",
    "resist", "reject", "decrease", "shrink", "retreat", "concern", "warning",
];

/// Score a piece of text for sentiment.
/// Returns a value in [-1.0, 1.0] where positive = bullish, negative = bearish.
pub fn score_text(text: &str) -> f64 {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    if words.is_empty() {
        return 0.0;
    }

    let mut pos_count = 0;
    let mut neg_count = 0;

    for word in &words {
        let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric());
        if POSITIVE_WORDS.contains(&cleaned) {
            pos_count += 1;
        }
        if NEGATIVE_WORDS.contains(&cleaned) {
            neg_count += 1;
        }
    }

    let diff = pos_count as f64 - neg_count as f64;
    let total = words.len() as f64;

    // Normalize and clamp to [-1, 1]
    // Multiply by scaling factor since keyword density is typically low
    (diff / total * 10.0).clamp(-1.0, 1.0)
}

/// Score title + description with title weighted 3x.
pub fn score_article(title: &str, description: &str) -> f64 {
    let title_score = score_text(title);
    let desc_score = score_text(description);
    // Title weight 3x, description 1x
    ((title_score * 3.0 + desc_score) / 4.0).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positive_text() {
        let score = score_text("Bitcoin surged to record high after ETF approved");
        assert!(score > 0.0, "Expected positive score, got {}", score);
    }

    #[test]
    fn test_negative_text() {
        let score = score_text("Market crashed amid scandal and crisis fears");
        assert!(score < 0.0, "Expected negative score, got {}", score);
    }

    #[test]
    fn test_neutral_text() {
        let score = score_text("The weather today is cloudy with a chance of rain");
        assert!(
            score.abs() < 0.01,
            "Expected neutral score, got {}",
            score
        );
    }

    #[test]
    fn test_empty_text() {
        assert!((score_text("") - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_article_scoring() {
        let score = score_article(
            "Bitcoin surged past record high",
            "Analysts remain cautious about further gains",
        );
        // Title is very positive, description slightly positive — overall positive
        assert!(score > 0.0);
    }
}
