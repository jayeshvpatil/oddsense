use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MarketCategory {
    Politics,
    Economics,
    Technology,
    Crypto,
    Sports,
    Science,
    Geopolitics,
    Culture,
    Other,
}

impl fmt::Display for MarketCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarketCategory::Politics => write!(f, "politics"),
            MarketCategory::Economics => write!(f, "economics"),
            MarketCategory::Technology => write!(f, "technology"),
            MarketCategory::Crypto => write!(f, "crypto"),
            MarketCategory::Sports => write!(f, "sports"),
            MarketCategory::Science => write!(f, "science"),
            MarketCategory::Geopolitics => write!(f, "geopolitics"),
            MarketCategory::Culture => write!(f, "culture"),
            MarketCategory::Other => write!(f, "other"),
        }
    }
}

impl MarketCategory {
    pub fn from_str_loose(s: &str) -> Option<MarketCategory> {
        match s.to_lowercase().as_str() {
            "politics" | "political" => Some(MarketCategory::Politics),
            "economics" | "economy" | "finance" | "financial" => Some(MarketCategory::Economics),
            "technology" | "tech" | "ai" => Some(MarketCategory::Technology),
            "crypto" | "cryptocurrency" | "blockchain" => Some(MarketCategory::Crypto),
            "sports" | "sport" => Some(MarketCategory::Sports),
            "science" | "space" | "health" => Some(MarketCategory::Science),
            "geopolitics" | "geopolitical" | "war" | "military" => Some(MarketCategory::Geopolitics),
            "culture" | "entertainment" | "media" => Some(MarketCategory::Culture),
            _ => None,
        }
    }
}

/// Categorize a market based on its title and description using keyword rules.
/// Ordered by specificity — most specific categories first.
pub fn categorize(title: &str, description: &str) -> MarketCategory {
    let text = format!("{} {}", title, description).to_lowercase();

    let rules: &[(&[&str], MarketCategory)] = &[
        // Crypto (most specific — check before tech/economics)
        (
            &[
                "bitcoin", "btc", "ethereum", "eth", "crypto", "blockchain",
                "defi", "nft", "stablecoin", "solana", "dogecoin", "altcoin",
                "token", "mining",
            ],
            MarketCategory::Crypto,
        ),
        // Sports
        (
            &[
                "super bowl", "nfl", "nba", "mlb", "nhl", "world cup", "fifa",
                "olympics", "championship", "playoffs", "quarterback", "touchdown",
                "mvp", "wimbledon", "grand slam",
            ],
            MarketCategory::Sports,
        ),
        // Geopolitics (before politics — more specific)
        (
            &[
                "war", "invasion", "ceasefire", "troops", "military", "nato",
                "sanctions", "nuclear", "missile", "territory", "occupied",
                "ukraine", "russia", "taiwan", "israel", "hamas", "hezbollah",
            ],
            MarketCategory::Geopolitics,
        ),
        // Politics
        (
            &[
                "election", "president", "congress", "senate", "governor",
                "vote", "democrat", "republican", "gop", "nominee", "cabinet",
                "impeach", "pardon", "executive order", "primary", "ballot",
                "speaker", "veto",
            ],
            MarketCategory::Politics,
        ),
        // Economics
        (
            &[
                "gdp", "inflation", "cpi", "fed", "federal reserve", "interest rate",
                "recession", "unemployment", "tariff", "trade war", "deficit",
                "debt ceiling", "budget", "stock market", "s&p", "dow", "nasdaq",
                "treasury", "bond", "yield",
            ],
            MarketCategory::Economics,
        ),
        // Technology (after crypto to avoid overlap)
        (
            &[
                "ai", "artificial intelligence", "openai", "anthropic", "google",
                "apple", "microsoft", "meta", "amazon", "tesla", "spacex",
                "gpt", "llm", "model", "chip", "semiconductor", "software",
                "hardware", "tech", "ipo",
            ],
            MarketCategory::Technology,
        ),
        // Science
        (
            &[
                "mars", "moon", "space", "nasa", "climate", "vaccine",
                "disease", "pandemic", "earthquake", "hurricane", "temperature",
                "carbon", "emissions", "species", "research",
            ],
            MarketCategory::Science,
        ),
        // Culture
        (
            &[
                "oscar", "academy award", "grammy", "emmy", "tony", "movie",
                "film", "album", "concert", "pope", "vatican", "royal",
                "celebrity", "viral", "tiktok", "netflix",
            ],
            MarketCategory::Culture,
        ),
    ];

    for (keywords, category) in rules {
        // Check for whole-word matches to avoid substring false positives
        let text_words: Vec<&str> = text
            .split(|c: char| !c.is_alphanumeric() && c != '&')
            .filter(|w| !w.is_empty())
            .collect();

        for kw in *keywords {
            if kw.contains(' ') {
                // Multi-word keyword: check substring
                if text.contains(kw) {
                    return category.clone();
                }
            } else {
                // Single-word keyword: check whole word
                if text_words.iter().any(|w| *w == *kw) {
                    return category.clone();
                }
            }
        }
    }

    MarketCategory::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto() {
        assert_eq!(categorize("Will Bitcoin hit 200k?", ""), MarketCategory::Crypto);
    }

    #[test]
    fn test_politics() {
        assert_eq!(
            categorize("Will Democrats win the 2028 election?", ""),
            MarketCategory::Politics
        );
    }

    #[test]
    fn test_technology() {
        assert_eq!(
            categorize("Will OpenAI release GPT-5?", ""),
            MarketCategory::Technology
        );
    }

    #[test]
    fn test_geopolitics() {
        assert_eq!(
            categorize("Will Russia and Ukraine reach a ceasefire?", ""),
            MarketCategory::Geopolitics
        );
    }

    #[test]
    fn test_sports() {
        assert_eq!(
            categorize("Who will win the Super Bowl?", "NFL championship"),
            MarketCategory::Sports
        );
    }

    #[test]
    fn test_culture() {
        assert_eq!(
            categorize("Who will be the next Pope?", "Vatican conclave"),
            MarketCategory::Culture
        );
    }

    #[test]
    fn test_economics() {
        assert_eq!(
            categorize("Will the Fed cut interest rates?", ""),
            MarketCategory::Economics
        );
    }

    #[test]
    fn test_no_ai_in_villain() {
        // "ai" should not match inside "villain" — whole word matching
        let cat = categorize("Will Adam Driver perform as Villain?", "");
        assert_ne!(cat, MarketCategory::Technology);
    }
}
