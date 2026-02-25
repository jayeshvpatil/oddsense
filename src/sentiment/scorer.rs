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

const NEGATION_WORDS: &[&str] = &[
    "not", "no", "never", "neither", "nobody", "nothing",
    "nor", "nowhere", "hardly", "barely", "scarcely",
    "doesn't", "doesnt", "don't", "dont", "didn't", "didnt",
    "won't", "wont", "wouldn't", "wouldnt", "can't", "cant",
    "couldn't", "couldnt", "shouldn't", "shouldnt",
    "isn't", "isnt", "aren't", "arent", "wasn't", "wasnt",
    "weren't", "werent", "hasn't", "hasnt", "haven't", "havent",
    "unlikely", "fails", "unable",
];

/// Score a piece of text for sentiment.
/// Returns a value in [-1.0, 1.0] where positive = bullish, negative = bearish.
/// Handles negation: "not approved" counts as negative, "not rejected" as positive.
pub fn score_text(text: &str) -> f64 {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    if words.is_empty() {
        return 0.0;
    }

    let mut pos_count = 0;
    let mut neg_count = 0;

    for (i, word) in words.iter().enumerate() {
        let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric());

        // Check if any of the preceding 2 words is a negation
        let negated = (1..=2).any(|offset| {
            i >= offset && {
                let prev = words[i - offset].trim_matches(|c: char| !c.is_alphanumeric());
                NEGATION_WORDS.contains(&prev)
            }
        });

        if POSITIVE_WORDS.contains(&cleaned) {
            if negated {
                neg_count += 1; // "not approved" → negative
            } else {
                pos_count += 1;
            }
        }
        if NEGATIVE_WORDS.contains(&cleaned) {
            if negated {
                pos_count += 1; // "not rejected" → positive
            } else {
                neg_count += 1;
            }
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

    #[test]
    fn test_negation_flips_positive() {
        let score = score_text("Bill was not approved by the senate");
        assert!(score < 0.0, "Expected negative score for 'not approved', got {}", score);
    }

    #[test]
    fn test_negation_flips_negative() {
        let score = score_text("The proposal hasn't failed yet");
        assert!(score > 0.0, "Expected positive score for 'hasn't failed', got {}", score);
    }

    #[test]
    fn test_double_negation_still_works() {
        // "not" + "crisis" → flipped to positive
        let score = score_text("There is not a crisis in the economy");
        assert!(score > 0.0, "Expected positive for negated negative, got {}", score);
    }
}
