pub mod categories;

use std::collections::HashMap;
use std::sync::LazyLock;

/// Domain-specific synonym map for prediction market queries.
/// Maps a term to related terms that prediction markets might use.
static SYNONYMS: LazyLock<HashMap<&'static str, &'static [&'static str]>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Technology & AI
    m.insert("ai", &["artificial intelligence", "machine learning", "deep learning", "llm", "gpt", "neural", "openai", "anthropic"][..]);
    m.insert("artificial intelligence", &["ai", "machine learning", "deep learning", "llm"][..]);
    m.insert("machine learning", &["ai", "artificial intelligence", "ml", "deep learning"][..]);
    m.insert("llm", &["ai", "artificial intelligence", "gpt", "language model"][..]);
    m.insert("gpt", &["ai", "openai", "chatgpt", "llm"][..]);
    m.insert("tech", &["technology", "software", "hardware", "silicon valley"][..]);
    m.insert("ipo", &["public offering", "stock market debut", "go public"][..]);

    // Crypto
    m.insert("crypto", &["bitcoin", "ethereum", "blockchain", "btc", "eth", "cryptocurrency", "defi"][..]);
    m.insert("bitcoin", &["btc", "crypto", "cryptocurrency"][..]);
    m.insert("btc", &["bitcoin", "crypto"][..]);
    m.insert("ethereum", &["eth", "crypto", "blockchain"][..]);
    m.insert("eth", &["ethereum", "crypto"][..]);
    m.insert("defi", &["decentralized finance", "crypto", "blockchain"][..]);
    m.insert("nft", &["non-fungible token", "crypto", "digital art"][..]);

    // Politics
    m.insert("election", &["presidential", "vote", "ballot", "candidate", "primary", "nominee"][..]);
    m.insert("presidential", &["president", "election", "white house"][..]);
    m.insert("president", &["presidential", "white house", "oval office"][..]);
    m.insert("congress", &["senate", "house", "legislature", "capitol"][..]);
    m.insert("senate", &["congress", "senator", "legislature"][..]);
    m.insert("democrat", &["democratic", "dem", "liberal", "left"][..]);
    m.insert("republican", &["gop", "conservative", "right"][..]);
    m.insert("gop", &["republican", "conservative"][..]);
    m.insert("trump", &["donald trump", "maga"][..]);
    m.insert("biden", &["joe biden"][..]);

    // Economics & Finance
    m.insert("recession", &["downturn", "gdp", "economic contraction", "depression"][..]);
    m.insert("inflation", &["cpi", "consumer prices", "price increase"][..]);
    m.insert("fed", &["federal reserve", "interest rate", "monetary policy", "fomc"][..]);
    m.insert("federal reserve", &["fed", "fomc", "interest rate"][..]);
    m.insert("interest rate", &["fed", "federal reserve", "rate cut", "rate hike"][..]);
    m.insert("tariff", &["trade war", "import tax", "trade policy", "duty"][..]);
    m.insert("trade war", &["tariff", "sanctions", "trade policy"][..]);
    m.insert("stock", &["equity", "shares", "market", "s&p", "dow"][..]);
    m.insert("gdp", &["economic growth", "economy", "recession"][..]);
    m.insert("budget", &["spending", "fiscal", "deficit", "debt"][..]);

    // Regulation & Policy
    m.insert("regulation", &["policy", "law", "legislation", "ban", "rule", "act"][..]);
    m.insert("policy", &["regulation", "law", "legislation", "rule"][..]);
    m.insert("ban", &["prohibition", "outlaw", "block", "restrict"][..]);
    m.insert("legislation", &["law", "bill", "act", "regulation"][..]);

    // Geopolitics & War
    m.insert("war", &["conflict", "military", "invasion", "ceasefire", "troops"][..]);
    m.insert("conflict", &["war", "military", "fighting", "hostilities"][..]);
    m.insert("nato", &["alliance", "military", "defense"][..]);
    m.insert("sanctions", &["embargo", "trade restrictions", "penalties"][..]);
    m.insert("ceasefire", &["peace", "truce", "armistice"][..]);
    m.insert("ukraine", &["russia", "kyiv", "zelenskyy"][..]);
    m.insert("china", &["beijing", "chinese", "prc"][..]);
    m.insert("taiwan", &["taipei", "china", "strait"][..]);

    // Science & Space
    m.insert("mars", &["space", "nasa", "spacex", "rocket"][..]);
    m.insert("space", &["nasa", "spacex", "mars", "moon", "rocket", "orbit"][..]);
    m.insert("nasa", &["space", "spacex", "mars"][..]);
    m.insert("climate", &["global warming", "carbon", "emissions", "temperature"][..]);

    // Sports
    m.insert("super bowl", &["nfl", "football", "championship"][..]);
    m.insert("nfl", &["football", "super bowl"][..]);
    m.insert("nba", &["basketball", "finals"][..]);
    m.insert("world cup", &["fifa", "soccer", "football"][..]);

    // Culture & Media
    m.insert("oscar", &["academy award", "film", "movie"][..]);
    m.insert("pope", &["vatican", "catholic", "papal", "conclave"][..]);

    m
});

/// Expand a query into the original plus synonym-based alternatives.
/// Returns a list of query strings to search for.
pub fn expand_query(query: &str) -> Vec<String> {
    let lower = query.to_lowercase();
    let mut expanded = vec![lower.clone()];

    // Try the full query as a key first
    if let Some(syns) = SYNONYMS.get(lower.as_str()) {
        for syn in *syns {
            expanded.push(syn.to_string());
        }
    }

    // Try individual words
    let words: Vec<&str> = lower.split_whitespace().collect();
    for word in &words {
        if let Some(syns) = SYNONYMS.get(*word) {
            for syn in *syns {
                // Create alternative query with this word replaced
                let alt = words
                    .iter()
                    .map(|w| if w == word { *syn } else { w })
                    .collect::<Vec<&str>>()
                    .join(" ");
                if !expanded.contains(&alt) {
                    expanded.push(alt);
                }
            }
        }
    }

    expanded
}

/// Score how relevant a market title is to a query.
/// Returns 0.0 (irrelevant) to 1.0 (exact match).
pub fn relevance_score(query: &str, title: &str) -> f64 {
    let q_lower = query.to_lowercase();
    let t_lower = title.to_lowercase();

    // Tokenize both query and title into whole words
    let q_words: Vec<&str> = q_lower.split_whitespace().collect();
    let t_words: Vec<String> = t_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect();

    // Strategy 1: Full multi-word phrase containment (strongest signal)
    // For single words, require whole-word match to avoid substring false positives
    if q_words.len() > 1 && t_lower.contains(&q_lower) {
        return 1.0;
    }

    // Strategy 2: All query words present as whole words
    let all_present = q_words
        .iter()
        .all(|qw| t_words.iter().any(|tw| tw == qw));
    if all_present {
        return if q_words.len() > 1 { 0.95 } else { 1.0 };
    }

    // Strategy 3: Word overlap ratio (partial matches)
    let matched = q_words
        .iter()
        .filter(|qw| t_words.iter().any(|tw| tw == **qw))
        .count();
    let overlap_ratio = matched as f64 / q_words.len().max(1) as f64;

    if overlap_ratio > 0.0 {
        return overlap_ratio * 0.7;
    }

    // Strategy 4: Jaro-Winkler on the full strings (catches typos/abbreviations)
    let jw = strsim::jaro_winkler(&q_lower, &t_lower);
    if jw > 0.85 {
        return jw * 0.5;
    }

    0.0
}

/// Score a title against a query and all its expanded variants.
/// Returns the maximum score across all variants.
pub fn expanded_relevance_score(query: &str, title: &str) -> f64 {
    let variants = expand_query(query);
    variants
        .iter()
        .map(|q| relevance_score(q, title))
        .fold(0.0_f64, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_query_ai() {
        let expanded = expand_query("AI");
        assert!(expanded.len() > 1);
        assert!(expanded.iter().any(|q| q.contains("artificial intelligence")));
    }

    #[test]
    fn test_expand_query_multi_word() {
        let expanded = expand_query("AI regulation");
        assert!(expanded.iter().any(|q| q.contains("policy") || q.contains("legislation")));
    }

    #[test]
    fn test_relevance_exact_match() {
        let score = relevance_score("bitcoin", "Will Bitcoin hit 200k?");
        assert!(score > 0.8, "Expected high score, got {}", score);
    }

    #[test]
    fn test_relevance_no_match() {
        let score = relevance_score("basketball", "Will Bitcoin hit 200k?");
        assert!(score < 0.1, "Expected low score, got {}", score);
    }

    #[test]
    fn test_expanded_relevance_synonym() {
        // "AI regulation" should match "artificial intelligence policy" via expansion
        let score = expanded_relevance_score("AI regulation", "Will Congress pass artificial intelligence policy?");
        assert!(score > 0.3, "Expected synonym match, got {}", score);
    }

    #[test]
    fn test_expanded_relevance_no_false_positive() {
        // "AI" should NOT match "villain" (the old substring bug)
        let score = expanded_relevance_score("AI", "Will Adam Driver perform as Villain?");
        assert!(score < 0.1, "Expected no match for villain, got {}", score);
    }
}
