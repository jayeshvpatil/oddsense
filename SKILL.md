# SKILL.md — oddsense

> Instructions for AI agents to use oddsense CLI.

## What is oddsense?

A CLI tool for prediction market intelligence. It aggregates data from Polymarket, Kalshi, and Metaculus, cross-references with news/Reddit sentiment, and detects divergences and arbitrage opportunities.

## Installation

```bash
cargo install oddsense
cargo install --git https://github.com/Polymarket/polymarket-cli.git
```

## Commands

### `oddsense search <query>`
Search prediction markets for a topic.
```
--limit <n>       Max results (default: 10)
--sort <field>    Sort by: volume_num, created_at (default: volume_num)
--format json     JSON output
--quiet           Suppress stderr
--raw             No pretty-printing
```

**JSON schema:**
```json
{
  "count": 5,
  "query": "bitcoin",
  "source": "polymarket",
  "markets": [
    {
      "id": "string",
      "source": "polymarket",
      "title": "Will Bitcoin hit $200k?",
      "description": "string",
      "probability": 0.35,
      "volume_24h": 100000.0,
      "volume_total": 5000000.0,
      "price_change_24h": 0.05,
      "end_date": "2026-12-31T00:00:00Z",
      "category": "crypto",
      "url": "https://polymarket.com/event/...",
      "source_data": {}
    }
  ]
}
```

### `oddsense enrich <query>`
Fetch sentiment signals for a topic from news and Reddit.
```
--sources <s>     news, reddit, all (default: all)
--format json
--quiet
--raw
```

**JSON schema:**
```json
{
  "query": "bitcoin",
  "source": "aggregate",
  "score": 0.25,
  "confidence": 0.8,
  "signal_count": 15,
  "sample_signals": [
    {
      "title": "Bitcoin surges past $100k",
      "source_name": "news",
      "sentiment": 0.6,
      "url": "https://..."
    }
  ],
  "analyzed_at": "2026-02-25T10:00:00Z"
}
```

- `score`: -1.0 (bearish) to 1.0 (bullish)
- `confidence`: 0.0 to 1.0

### `oddsense divergence <query>`
Find markets where odds diverge from real-world sentiment.
```
--sentiment <s>   news, reddit, all (default: all)
--min-score <n>   Minimum divergence score 0-100 (default: 20)
--limit <n>       Max results (default: 10)
--explain         Print human-readable explanations to stderr
--format json
--quiet
--raw
```

**JSON schema:**
```json
{
  "count": 3,
  "query": "AI",
  "divergences": [
    {
      "market": { "...NormalizedMarket..." },
      "sentiment": { "...SentimentResult..." },
      "divergence_score": 65.0,
      "direction": "MarketHigher",
      "signal_strength": "strong",
      "summary": "\"Will AI...\" — Market: 80% | Sentiment: 30% | Divergence: 65/100"
    }
  ]
}
```

- `direction`: "MarketHigher" or "SentimentHigher"
- `signal_strength`: "weak" (<25), "moderate" (25-50), "strong" (>50)

### `oddsense signals`
Surface trending markets by volume.
```
--timeframe <t>   1h, 24h, 7d (default: 24h)
--min-volume <n>  Minimum 24h volume in USD (default: 10000)
--limit <n>       Max results (default: 20)
--format json
--quiet
--raw
```

### `oddsense arbitrage [query]`
Find cross-platform arbitrage opportunities.
```
--sources <s>     polymarket,kalshi,metaculus,all (default: all)
--min-spread <n>  Minimum spread in percentage points (default: 5)
--similarity <n>  Title matching threshold 0.0-1.0 (default: 0.7)
--limit <n>       Max results (default: 20)
--format json
--quiet
--raw
```

**JSON schema:**
```json
{
  "count": 2,
  "query": null,
  "min_spread": 5.0,
  "opportunities": [
    {
      "topic": "Will Jesus Christ return before GTA VI?",
      "markets": [
        { "source": "kalshi", "id": "...", "title": "...", "probability": 0.88, "url": "..." },
        { "source": "polymarket", "id": "...", "title": "...", "probability": 0.48, "url": "..." }
      ],
      "spread": 39.5,
      "highest": { "source": "kalshi", "probability": 0.88, "..." },
      "lowest": { "source": "polymarket", "probability": 0.48, "..." },
      "similarity": 0.7,
      "summary": "\"...\" — kalshi 88% vs polymarket 48% — spread 39.5pp"
    }
  ]
}
```

### `oddsense compare <query>`
Compare the same question across platforms side-by-side.
```
--sources <s>     polymarket,kalshi,metaculus,all (default: all)
--similarity <n>  Title matching threshold 0.0-1.0 (default: 0.6)
--limit <n>       Max results per source (default: 10)
--format json
--quiet
--raw
```

**JSON schema:**
```json
{
  "query": "bitcoin",
  "sources": ["polymarket", "kalshi"],
  "markets": [
    { "...NormalizedMarket with source field..." }
  ]
}
```

## Agent Pipelines

```bash
# Find high-confidence divergences
oddsense divergence "AI" -q --raw --format json | jq '.divergences[] | select(.divergence_score > 50)'

# Get arbitrage spreads > 10pp
oddsense arbitrage --min-spread 10 -q --raw --format json | jq '.opportunities[].summary'

# Daily market scan
for topic in "AI" "crypto" "elections"; do
  echo "=== $topic ==="
  oddsense divergence "$topic" --format json -q --raw
done

# Combine search + sentiment
QUERY="bitcoin"
oddsense search "$QUERY" --format json -q --raw > /tmp/markets.json
oddsense enrich "$QUERY" --format json -q --raw > /tmp/sentiment.json
jq -s '{ markets: .[0], sentiment: .[1] }' /tmp/markets.json /tmp/sentiment.json

# Cross-platform comparison piped to analysis
oddsense arbitrage --format json -q --raw | \
  jq '.opportunities[] | {topic, spread, buy: .lowest.source, sell: .highest.source}'
```

## Exit Codes

- `0`: Success (even if no results — check `count` field)
- `1`: Error (API failure, missing dependency, etc.)

## Notes

- Data goes to stdout, human messages to stderr
- `--quiet` suppresses all stderr output
- `--raw` disables JSON pretty-printing (for piping)
- Requires `polymarket-cli` in PATH for Polymarket commands
- NewsAPI key needed for news sentiment (set in config.toml)
- Kalshi and Reddit work without API keys
- Metaculus requires auth — skipped gracefully if unavailable
