# CLAUDE.md — oddsense

## Project Overview

**oddsense** is an agent-native CLI intelligence layer for prediction markets. It does NOT re-implement market data fetching. Instead, it **composes with existing CLIs** (starting with [polymarket-cli](https://github.com/Polymarket/polymarket-cli)) and adds the intelligence layer on top: cross-platform aggregation, real-world sentiment analysis, divergence detection, and arbitrage discovery.

Think of it as: `polymarket-cli` is the data pipe. `oddsense` is the brain.

### Core Philosophy (from Karpathy)

> "CLIs are super exciting precisely because they are a 'legacy' technology, which means AI agents can natively and easily use them, combine them... Even more powerful when you use it as a module of bigger pipelines."

oddsense embodies this. It's a CLI that agents can install, compose, and build dashboards/apps on top of. It treats polymarket-cli (and future CLIs like kalshi, metaculus) as upstream data sources and focuses exclusively on what they don't provide: **analysis, enrichment, signals, and cross-source intelligence.**

---

## Tech Stack

- **Language**: Rust (fast, single binary, no runtime — matches polymarket-cli)
- **CLI Framework**: `clap` v4 (derive API)
- **HTTP Client**: `reqwest` with `tokio` async runtime
- **JSON**: `serde` / `serde_json`
- **TUI**: `ratatui` + `crossterm` for live dashboards
- **Output**: `tabled` for table output, raw JSON for piping
- **Config**: `directories` crate + `toml` for config
- **Errors**: `anyhow` (binary) + `thiserror` (library)
- **Search**: `strsim` for Jaro-Winkler similarity, `std::sync::LazyLock` for static synonym maps
- **LLM**: Anthropic Claude API (via `reqwest`) for smart query expansion & reranking (optional, behind `--smart` flag)
- **External dep**: `polymarket-cli` (invoked via `std::process::Command`, parsed via JSON output)

---

## Architecture: Composition Over Reimplementation

```
┌─────────────────────────────────────────────────────────┐
│                      oddsense                          │
│                  (intelligence layer)                    │
│                                                         │
│  ┌──────────┐  ┌────────────┐  ┌─────────────────────┐ │
│  │ Sentiment│  │ Divergence │  │   Cross-Platform    │ │
│  │ Engine   │  │ Detector   │  │   Arbitrage Finder  │ │
│  └────┬─────┘  └─────┬──────┘  └──────────┬──────────┘ │
│       │              │                     │            │
│  ┌────┴──────────────┴─────────────────────┴──────────┐ │
│  │              Unified Market Schema                  │ │
│  │         (normalize all sources into one)             │ │
│  └────┬──────────────┬─────────────────────┬──────────┘ │
│       │              │                     │            │
│  ┌────┴────┐   ┌─────┴─────┐   ┌──────────┴──────────┐ │
│  │Polymarket│  │  Kalshi   │   │   Metaculus         │ │
│  │ Adapter  │  │  Adapter  │   │   Adapter           │ │
│  │(via CLI) │  │ (via API) │   │   (via API)         │ │
│  └──────────┘  └───────────┘   └─────────────────────┘ │
└─────────────────────────────────────────────────────────┘
        │               │                    │
   polymarket-cli    Kalshi API         Metaculus API
   (subprocess)      (direct HTTP)      (direct HTTP)
```

**Key design decision**: For Polymarket, we shell out to `polymarket-cli` and parse its JSON output. This avoids reimplementing their auth, CLOB interaction, and API wrappers. For other platforms without existing CLIs, we call their APIs directly. This means users get the full power of polymarket-cli for trading/browsing and oddsense for intelligence.

---

## Project Structure

```
oddsense/
├── CLAUDE.md                # This file — build instructions for Claude Code
├── SKILL.md                 # Instructions for OTHER agents consuming this CLI
├── README.md                # Human-facing docs
├── Cargo.toml
├── Cargo.lock
├── config.example.toml
├── src/
│   ├── main.rs              # Entry point, clap CLI definition
│   ├── lib.rs               # Re-exports
│   │
│   ├── cli/                 # Command handlers
│   │   ├── mod.rs           # Subcommand routing + CLI arg structs
│   │   ├── search.rs        # `search` — multi-source market search with --smart and --category
│   │   ├── enrich.rs        # `enrich` — add sentiment to market data
│   │   ├── divergence.rs    # `divergence` — find market vs reality gaps
│   │   ├── arbitrage.rs     # `arbitrage` — cross-platform odds comparison
│   │   ├── signals.rs       # `signals` — trending topics across sources
│   │   ├── dashboard.rs     # `dashboard` — live TUI
│   │   └── compare.rs       # `compare` — side-by-side platform comparison
│   │
│   ├── adapters/            # Data source adapters
│   │   ├── mod.rs           # MarketSource trait + registry
│   │   ├── schema.rs        # Unified NormalizedMarket struct + is_expired()
│   │   ├── polymarket.rs    # Wraps polymarket-cli subprocess
│   │   ├── kalshi.rs        # Direct Kalshi API client (semantic search via expanded_relevance_score)
│   │   └── metaculus.rs     # Direct Metaculus API client
│   │
│   ├── search/              # Search intelligence (v0.2.0)
│   │   ├── mod.rs           # Synonym expansion, relevance scoring, expanded_relevance_score()
│   │   └── categories.rs    # Rule-based market categorization (MarketCategory enum)
│   │
│   ├── llm/                 # LLM integration (v0.2.0, behind --smart flag)
│   │   ├── mod.rs           # QueryExpansion, RankedResult, RerankResponse structs
│   │   └── provider.rs      # AnthropicProvider — query expansion + result reranking
│   │
│   ├── sentiment/           # Sentiment analysis engines
│   │   ├── mod.rs           # SentimentSource trait + aggregator
│   │   ├── news.rs          # NewsAPI / RSS feed analysis
│   │   ├── reddit.rs        # Reddit API sentiment
│   │   └── scorer.rs        # Keyword + negation-aware sentiment scorer
│   │
│   ├── analysis/            # Core intelligence algorithms
│   │   ├── mod.rs
│   │   ├── divergence.rs    # Divergence detection + scoring
│   │   ├── arbitrage.rs     # Cross-platform spread detection
│   │   ├── momentum.rs      # Volume/price momentum signals
│   │   └── correlation.rs   # Event correlation (related markets moving together)
│   │
│   ├── output/              # Output formatters
│   │   ├── mod.rs           # Format dispatcher
│   │   ├── json.rs          # JSON (default for agents)
│   │   ├── table.rs         # Pretty tables (default for humans)
│   │   └── tui.rs           # Ratatui live dashboard
│   │
│   └── config.rs            # Config loading, API key management, LLM config
│
└── tests/
    ├── integration/
    │   ├── enrich_test.rs
    │   ├── divergence_test.rs
    │   └── arbitrage_test.rs
    └── fixtures/
        ├── polymarket_cli_output.json
        ├── kalshi_response.json
        └── metaculus_response.json
```

---

## Build Phases

### Phase 1: Polymarket Adapter + Unified Schema

**Goal**: Shell out to `polymarket-cli`, parse JSON output, normalize into unified schema.

1. Initialize Rust project: `cargo init --name oddsense`
2. Add dependencies to `Cargo.toml`:
   ```toml
   [dependencies]
   clap = { version = "4", features = ["derive"] }
   tokio = { version = "1", features = ["full"] }
   reqwest = { version = "0.12", features = ["json"] }
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   anyhow = "1"
   thiserror = "2"
   tabled = "0.17"
   chrono = { version = "0.4", features = ["serde"] }
   toml = "0.8"
   directories = "6"
   ```
3. Define `adapters/schema.rs` — the unified market type all sources normalize into:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct NormalizedMarket {
       pub id: String,
       pub source: String,            // "polymarket", "kalshi", "metaculus"
       pub title: String,
       pub description: String,
       pub probability: f64,          // 0.0 - 1.0 (normalized!)
       pub volume_24h: Option<f64>,   // in USD
       pub volume_total: Option<f64>,
       pub price_change_24h: Option<f64>,
       pub end_date: Option<String>,  // ISO 8601
       pub category: Option<String>,
       pub url: String,
       pub source_data: Value,        // raw JSON from source, for passthrough
   }
   ```
4. Define `MarketSource` trait in `adapters/mod.rs`:
   ```rust
   #[async_trait]
   pub trait MarketSource: Send + Sync {
       fn name(&self) -> &str;
       async fn search(&self, query: &str, limit: usize) -> Result<Vec<NormalizedMarket>>;
       async fn list_top(&self, limit: usize, sort_by: &str) -> Result<Vec<NormalizedMarket>>;
       async fn get_market(&self, id: &str) -> Result<NormalizedMarket>;
       fn is_available(&self) -> bool; // check if CLI/API is accessible
   }
   ```
5. Implement `adapters/polymarket.rs`:
   - Check that `polymarket` binary exists in PATH (print helpful install message if not)
   - Shell out via `tokio::process::Command`:
     ```rust
     let output = Command::new("polymarket")
         .args(&["-o", "json", "markets", "search", query, "--limit", &limit.to_string()])
         .output()
         .await
         .context("Failed to run polymarket-cli. Is it installed? Run: curl -sSL https://raw.githubusercontent.com/Polymarket/polymarket-cli/main/install.sh | sh")?;
     ```
   - Parse JSON output into `Vec<serde_json::Value>`
   - Map to `NormalizedMarket` (key mapping: `outcomePrices[0]` → `probability`, `volume` → `volume_total`, etc.)
   - Handle polymarket-cli errors (non-zero exit, JSON parse failures)
   - **Important**: Polymarket prices are in cents (0-100), normalize to 0.0-1.0
6. Create basic CLI skeleton with a `search` passthrough command to verify the pipeline works:
   ```
   oddsense search "bitcoin" --limit 5 --format json
   ```

**Milestone**: `oddsense search "bitcoin"` calls polymarket-cli under the hood and returns normalized data.

**Testing**: Save a real `polymarket -o json markets search "bitcoin"` output to `tests/fixtures/polymarket_cli_output.json`. Write unit tests that parse this fixture through the adapter.

---

### Phase 2: Sentiment Engine

**Goal**: Build a sentiment analysis pipeline that scores real-world signals for a given topic.

1. Define `SentimentResult` struct in `sentiment/mod.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct SentimentResult {
       pub query: String,
       pub source: String,            // "newsapi", "reddit", "aggregate"
       pub score: f64,                // -1.0 (very bearish) to 1.0 (very bullish)
       pub confidence: f64,           // 0.0 - 1.0, how confident in the score
       pub signal_count: u32,         // number of articles/posts analyzed
       pub sample_signals: Vec<SignalItem>,
       pub analyzed_at: String,       // ISO 8601
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct SignalItem {
       pub title: String,
       pub source_name: String,       // "CNN", "Reuters", etc.
       pub published_at: String,
       pub sentiment: f64,            // individual item score
       pub url: Option<String>,
   }
   ```
2. Implement `sentiment/scorer.rs` — a keyword/heuristic sentiment scorer:
   - Maintain word lists for positive/negative market-relevant terms
   - Positive: "approved", "passed", "surged", "confirmed", "deal", "breakthrough", "wins"
   - Negative: "rejected", "failed", "crashed", "blocked", "scandal", "collapses", "loses"
   - Score = (positive_count - negative_count) / total_words, clamped to [-1, 1]
   - Weight title words higher than description words (3x)
   - This is deliberately simple — a good heuristic beats a slow ML model for a CLI
3. Implement `sentiment/news.rs`:
   - Use NewsAPI.org (`https://newsapi.org/v2/everything`)
   - Query parameters: `q=<topic>&sortBy=publishedAt&language=en&pageSize=20`
   - Requires API key (stored in config.toml)
   - For each article: extract title + description, run through scorer
   - Aggregate into `SentimentResult`
   - Handle rate limits (free tier: 100 req/day) — cache results for 15 min
4. Implement `sentiment/reddit.rs`:
   - Use Reddit JSON API (no auth needed): `https://www.reddit.com/search.json?q=<query>&sort=new&limit=25`
   - Parse `data.children[].data.title` and `selftext`
   - Run through scorer
   - Reddit is good for grassroots/retail sentiment vs news for institutional
5. Implement aggregate sentiment in `sentiment/mod.rs`:
   - Combine news + reddit scores with configurable weights (default: news=0.6, reddit=0.4)
   - Return both individual and aggregate `SentimentResult`s
6. Add `enrich` command in `cli/enrich.rs`:
   ```
   oddsense enrich "bitcoin" --sources news,reddit --format json
   ```
   - Takes a topic, fetches sentiment from all sources, returns enriched data
   - Can also accept piped input: `polymarket -o json markets search "bitcoin" | oddsense enrich --stdin`

**Milestone**: `oddsense enrich "AI regulation"` returns sentiment scores from news and reddit.

**Testing**: Mock HTTP responses for NewsAPI and Reddit. Test scorer with known positive/negative headlines.

---

### Phase 3: Divergence Detection (The Killer Feature)

**Goal**: Compare prediction market odds against real-world sentiment and surface actionable divergences.

1. Define `Divergence` struct in `analysis/divergence.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Divergence {
       pub market: NormalizedMarket,
       pub sentiment: SentimentResult,
       pub divergence_score: f64,     // 0-100, higher = more divergent
       pub direction: DivergenceDirection,
       pub summary: String,           // human-readable explanation
       pub signal_strength: String,   // "weak", "moderate", "strong"
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum DivergenceDirection {
       MarketHigher,   // market says likely, sentiment says unlikely
       SentimentHigher, // sentiment says likely, market says unlikely
   }
   ```
2. Implement divergence scoring algorithm:
   ```
   INPUT: market.probability (0.0-1.0), sentiment.score (-1.0 to 1.0)

   Step 1: Normalize sentiment to probability-like scale
     sentiment_prob = (sentiment.score + 1.0) / 2.0  // maps [-1,1] to [0,1]

   Step 2: Calculate raw divergence
     raw_diff = abs(market.probability - sentiment_prob)

   Step 3: Weight by confidence
     weighted_diff = raw_diff * sentiment.confidence

   Step 4: Scale to 0-100
     divergence_score = weighted_diff * 100

   Step 5: Determine direction
     if market.probability > sentiment_prob → MarketHigher
     else → SentimentHigher

   Step 6: Signal strength
     0-25: "weak"
     25-50: "moderate"
     50+: "strong"
   ```
3. Implement `cli/divergence.rs` command:
   ```
   oddsense divergence <query>
     --sources polymarket,news,reddit
     --min-score 20
     --limit 10
     --format json|table
   ```
   - Workflow:
     1. Search Polymarket (via polymarket-cli) for query → get markets
     2. For each market, extract the core topic from the title
     3. Run sentiment analysis on the topic
     4. Compute divergence score for each market
     5. Sort by divergence score descending
     6. Filter by min-score
     7. Output results
4. Add `--explain` flag that includes a human-readable explanation:
   ```
   "The market puts 'Will Bitcoin hit $150k by July 2026' at 23% probability,
    but news sentiment is strongly bullish (score: 0.72). Divergence: 68/100 (strong).
    This could mean the market is underpricing the event relative to current news flow."
   ```

**Milestone**: `oddsense divergence "bitcoin" --min-score 30` returns markets where odds significantly diverge from real-world sentiment.

**Testing**: Create fixture data with known divergences. Test that scoring algorithm produces expected results for edge cases (0 probability, neutral sentiment, max divergence, etc.)

---

### Phase 4: Cross-Platform Arbitrage

**Goal**: Find the same question priced differently across prediction market platforms.

1. Implement `adapters/kalshi.rs`:
   - Kalshi API: `https://api.elections.kalshi.com/trade-api/v2`
   - `GET /events?status=open&with_nested_markets=true` — list active events with markets
   - `GET /events?status=open&with_nested_markets=true&series_ticker=<ticker>` — filter by series
   - Market data: `yes_price` (0-100) → divide by 100 for probability
   - No API key for read-only
   - Rate limit: ~10 req/sec
2. Implement `adapters/metaculus.rs`:
   - Metaculus API: `https://www.metaculus.com/api2/questions/`
   - Supports `?search=<query>` parameter
   - Map community_prediction → probability
   - No API key needed
3. Implement `analysis/arbitrage.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ArbitrageOpportunity {
       pub topic: String,
       pub markets: Vec<NormalizedMarket>,  // same question, different sources
       pub spread: f64,                      // max_prob - min_prob
       pub highest: MarketRef,               // source with highest probability
       pub lowest: MarketRef,                // source with lowest probability
       pub summary: String,
   }
   ```
   - **Matching algorithm**: This is the hard part. Same question appears differently across platforms.
     - Fuzzy string matching on titles (use `strsim` crate for Levenshtein/Jaro-Winkler)
     - Match if similarity > 0.7 AND same rough time window for end date
     - Allow manual overrides via config file for known pairings
   - Calculate spread between highest and lowest probability across platforms
   - Only surface opportunities where spread > threshold (default 5%)
4. Implement `cli/arbitrage.rs` command:
   ```
   oddsense arbitrage
     --sources polymarket,kalshi,metaculus
     --min-spread 5
     --limit 20
     --format json|table
   ```
   - Optionally take a topic: `oddsense arbitrage "AI" --min-spread 10`
5. Implement `cli/compare.rs` for explicit side-by-side:
   ```
   oddsense compare "Will GPT-5 release in 2026"
     --sources polymarket,kalshi,metaculus
     --format json|table
   ```
   - Shows the same (or similar) question across all platforms with odds, volume, and end dates

**Milestone**: `oddsense arbitrage --min-spread 10` finds questions priced >10% differently across platforms.

**Testing**: Create fixtures with intentionally similar/different market titles. Test fuzzy matching accuracy. Test spread calculations.

---

### Phase 5: Live TUI Dashboard

**Goal**: A real-time terminal dashboard combining all intelligence into one view.

1. Add dependencies:
   ```toml
   ratatui = "0.29"
   crossterm = "0.28"
   ```
2. Implement `output/tui.rs` with four panels:
   ```
   ┌─────────────────────────────────────────────────────────┐
   │  ODDSENSE — Prediction Market Intelligence    [q]quit  │
   ├──────────────────────────────┬──────────────────────────┤
   │  🔥 TOP DIVERGENCES          │  📊 ARBITRAGE SPREADS    │
   │                              │                          │
   │  Bitcoin $150k by July       │  "AI regulation 2026"    │
   │  Market: 23% | Sentiment: 72%│  PM: 45% | KL: 38%      │
   │  Divergence: 68 ▲ STRONG    │  Spread: 7%              │
   │                              │                          │
   │  Trump VP pick before Aug    │  "Fed rate cut June"     │
   │  Market: 67% | Sentiment: 41%│  PM: 62% | KL: 71%      │
   │  Divergence: 52 ▼ STRONG    │  Spread: 9%              │
   │                              │                          │
   ├──────────────────────────────┴──────────────────────────┤
   │  📈 MOMENTUM SIGNALS                                    │
   │                                                         │
   │  ▲ +15%  Will OpenAI IPO in 2026      Vol: $2.3M       │
   │  ▲ +12%  Bitcoin above $200k EOY      Vol: $890K       │
   │  ▼ -18%  TikTok ban upheld            Vol: $1.1M       │
   │  ▼ -9%   Fed cuts before July         Vol: $3.2M       │
   │                                                         │
   ├─────────────────────────────────────────────────────────┤
   │  Refreshing in 45s | Sources: PM ✓  KL ✓  MC ✓  News ✓│
   └─────────────────────────────────────────────────────────┘
   ```
3. Keyboard controls:
   - `q` / `Esc` — quit
   - `r` — force refresh
   - `Tab` — cycle focus between panels
   - `Enter` — expand selected item (full detail view)
   - `/` — search filter
   - `1-4` — toggle panels
4. Data fetching:
   - Use `tokio::spawn` for background data fetching (never block the TUI thread)
   - Configurable refresh interval (default 60s)
   - Show loading spinners per-panel while fetching
   - Cache data between refreshes, show stale indicator if fetch fails
5. Launch command:
   ```
   oddsense dashboard
     --refresh 60
     --sources polymarket,kalshi
     --sentiment news,reddit
     --panels divergence,arbitrage,momentum,watchlist
   ```

**Milestone**: `oddsense dashboard` launches a live-updating multi-panel TUI.

---

### Phase 6: Signals & Momentum Analysis

**Goal**: Surface trending markets and momentum shifts.

1. Implement `analysis/momentum.rs`:
   - Track markets with biggest 24h price changes (use `polymarket -o json markets list --order volume_num`)
   - Categorize as momentum signals: direction (up/down), magnitude, volume context
   - **Volume-weighted momentum**: A 10% move on $5M volume is more significant than on $10K
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct MomentumSignal {
       pub market: NormalizedMarket,
       pub price_change: f64,          // percentage
       pub volume_change: f64,         // percentage volume change
       pub momentum_score: f64,        // combined weighted score
       pub direction: String,          // "bullish" or "bearish"
       pub tier: String,               // "whale" (>$1M vol), "active" (>$100K), "quiet"
   }
   ```
2. Implement `analysis/correlation.rs`:
   - Find markets that are moving together (e.g., multiple AI-related markets all shifting up)
   - Group by category/tag, detect when an entire category is trending
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct CorrelationCluster {
       pub theme: String,              // "AI", "Crypto", "US Politics"
       pub markets: Vec<NormalizedMarket>,
       pub avg_movement: f64,
       pub direction: String,
       pub summary: String,
   }
   ```
3. Implement `cli/signals.rs` command:
   ```
   oddsense signals
     --timeframe 24h|7d
     --min-volume 10000
     --format json|table

   oddsense signals --correlated
     # Shows correlated market clusters
   ```

**Milestone**: `oddsense signals` surfaces high-momentum markets with volume context and correlation clusters.

---

### Phase 7: Config, Polish & SKILL.md

**Goal**: Persistent config, caching, and full agent-readiness.

1. Implement `config.rs`:
   - Config at `~/.config/oddsense/config.toml` (macOS: `~/Library/Application Support/com.oddsense.oddsense/config.toml`):
     ```toml
     [api_keys]
     newsapi = "your-key-here"
     anthropic = "sk-ant-..."   # for --smart mode (LLM query expansion + reranking)
     openai = ""                # future: alternative to anthropic
     # kalshi and metaculus don't need keys for read

     [defaults]
     format = "table"
     refresh_seconds = 60
     sources = ["polymarket"]

     [llm]
     provider = "anthropic"                   # or "openai" (future)
     model = "claude-haiku-4-5-20251001"      # optional model override
     ```
   - Environment variable fallback: `ANTHROPIC_API_KEY` is checked if config key is empty
2. Implement basic file-based caching:
   - Cache dir: `~/.cache/oddsense/`
   - Cache key: `{source}_{query}_{timestamp_bucket}.json`
   - TTL-based invalidation
   - `--no-cache` flag to bypass
3. Write `SKILL.md` for agent consumption:
   - Every command with full flag documentation
   - JSON output schemas for each command
   - Example compositions and pipelines
   - Prerequisite: polymarket-cli must be installed
   - Error handling guide (what exit codes mean)
4. Write `README.md` for humans:
   - Install instructions
   - Quick start (5 commands to get value)
   - Screenshots / GIFs of TUI dashboard
   - Architecture diagram
   - Contributing guide
5. Ensure agent-friendliness:
   - Every command supports `--format json` with consistent schemas
   - Exit codes: 0=success, 1=error, 2=no results found
   - stderr for logs/messages, stdout for data only
   - `--quiet` flag suppresses all non-data output
   - `--raw` flag for unformatted JSON (no pretty-printing)
   - `--stdin` flag to accept piped input from other CLIs

---

## CLI Command Reference

### `oddsense enrich <query>`
Fetch sentiment signals for a topic. The foundational command everything else builds on.
```
FLAGS:
  --sources <list>       Sentiment sources: news, reddit, all (default: all)
  --format <format>      json | table (default: table)
  --stdin                Read market data from stdin (piped from polymarket-cli)
  --no-cache             Bypass cache
```
**Example**:
```bash
# Standalone
oddsense enrich "bitcoin ETF"

# Piped from polymarket-cli
polymarket -o json markets search "bitcoin" | oddsense enrich --stdin
```

### `oddsense divergence <query>`
Find markets where odds diverge from real-world sentiment. The killer feature.
```
FLAGS:
  --sources <list>       Market sources: polymarket, kalshi, metaculus, all (default: polymarket)
  --sentiment <list>     Sentiment sources: news, reddit, all (default: all)
  --min-score <n>        Minimum divergence score 0-100 (default: 20)
  --limit <n>            Max results (default: 10)
  --format <format>      json | table (default: table)
  --explain              Include human-readable explanations
```
**Example**:
```bash
oddsense divergence "AI" --min-score 40 --explain --format json
```

### `oddsense arbitrage [query]`
Find the same question priced differently across platforms.
```
FLAGS:
  --sources <list>       Platforms to compare: polymarket, kalshi, metaculus (default: all available)
  --min-spread <pct>     Minimum spread to surface (default: 5.0)
  --limit <n>            Max results (default: 20)
  --format <format>      json | table (default: table)
```
**Example**:
```bash
oddsense arbitrage --min-spread 10 --format json
oddsense arbitrage "election" --sources polymarket,kalshi
```

### `oddsense compare <query>`
Side-by-side comparison of how different platforms price the same topic.
```
FLAGS:
  --sources <list>       Platforms: polymarket, kalshi, metaculus (default: all available)
  --format <format>      json | table (default: table)
```
**Example**:
```bash
oddsense compare "Will GPT-5 launch in 2026"
```

### `oddsense signals`
Surface trending markets, momentum shifts, and correlated clusters.
```
FLAGS:
  --timeframe <t>        1h, 24h, 7d (default: 24h)
  --min-volume <usd>     Minimum volume filter (default: 10000)
  --correlated           Show correlated market clusters
  --limit <n>            Max results (default: 20)
  --format <format>      json | table (default: table)
```
**Example**:
```bash
oddsense signals --timeframe 24h --min-volume 100000
oddsense signals --correlated --format json
```

### `oddsense dashboard`
Launch live TUI dashboard with all intelligence panels.
```
FLAGS:
  --refresh <secs>       Refresh interval (default: 60)
  --sources <list>       Market sources (default: from config)
  --sentiment <list>     Sentiment sources (default: from config)
  --panels <list>        Panels to show: divergence, arbitrage, momentum, all (default: all)
```
**Example**:
```bash
oddsense dashboard --refresh 30 --panels divergence,momentum
```

### `oddsense search <query>`
Search prediction markets across Polymarket, Kalshi, and Metaculus with semantic search.
```
FLAGS:
  --sources <list>       Comma-separated: polymarket,kalshi,metaculus,all (default: all)
  --category <cat>       Filter by category: politics, economics, technology, crypto, sports, science, geopolitics, culture
  --limit <n>            Max results (default: 10)
  --sort <field>         Sort by: volume_num, created_at (default: volume_num)
  --format <format>      json | table (default: table)
```

### Global Flags (all commands)
```
  --format <format>      json | table (default: table)
  --smart, -s            LLM-powered query expansion + result reranking (requires Anthropic API key)
  --quiet, -q            Suppress non-data output
  --raw                  Unformatted JSON (for piping)
  --no-cache             Bypass cache
  --config <path>        Custom config file path
  --no-color             Disable colored output
  --verbose, -v          Verbose logging to stderr
```

---

## Polymarket CLI Integration Details

### How we invoke polymarket-cli

```rust
use tokio::process::Command;

pub async fn polymarket_search(query: &str, limit: usize) -> Result<Vec<Value>> {
    let output = Command::new("polymarket")
        .args(&["-o", "json", "markets", "search", query, "--limit", &limit.to_string()])
        .output()
        .await
        .context("Failed to run polymarket-cli. Is it installed? Run: curl -sSL https://raw.githubusercontent.com/Polymarket/polymarket-cli/main/install.sh | sh")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("polymarket-cli error: {}", stderr);
    }

    let markets: Vec<Value> = serde_json::from_slice(&output.stdout)
        .context("Failed to parse polymarket-cli JSON output")?;

    Ok(markets)
}
```

### Key polymarket-cli commands we rely on

```bash
# Search markets (Phase 1)
polymarket -o json markets search "<query>" --limit N

# List by volume (Phase 1, for movers/signals)
polymarket -o json markets list --limit N --order volume_num

# Market detail (Phase 3, for enrichment)
polymarket -o json markets get <slug-or-id>

# Price history (Phase 6, for momentum)
polymarket -o json clob price-history <token_id> --interval 1d

# Events with tags (Phase 4, for categorization)
polymarket -o json events list --tag <tag> --limit N
polymarket -o json tags list
```

### Field mapping: polymarket output → NormalizedMarket

```
polymarket.question       → title
polymarket.description    → description
polymarket.outcomePrices[0] → probability (parse string to f64, divide by 100 if > 1)
polymarket.volume         → volume_total (parse "$145.2M" → 145200000.0)
polymarket.id             → id
polymarket.slug           → used for URL construction
"polymarket"              → source
```

**Gotcha**: polymarket-cli's volume field may be a formatted string like "$145.2M". Write a parser that handles K/M/B suffixes.

---

## API Endpoints (for non-CLI sources)

### Kalshi (Phase 4)
- Base: `https://api.elections.kalshi.com/trade-api/v2`
- `GET /events?status=open&with_nested_markets=true` — list active events with markets
- `GET /events?status=open&with_nested_markets=true&series_ticker=<ticker>` — filter by series
- Market data: `yes_price` (0-100) → divide by 100 for probability
- No API key for read-only
- Rate limit: ~10 req/sec

### Metaculus (Phase 4)
- Base: `https://www.metaculus.com/api2`
- `GET /questions/?search=<query>&status=open&type=forecast` — search questions
- `GET /questions/<id>/` — question detail
- Uses `community_prediction.full.q2` (median) as probability
- No API key needed
- Rate limit: be respectful, ~1 req/sec

### NewsAPI (Phase 2)
- Base: `https://newsapi.org/v2`
- `GET /everything?q=<query>&sortBy=publishedAt&language=en&pageSize=20`
- Requires API key (free tier: 100 req/day)
- Returns: title, description, source.name, publishedAt, url

### Reddit (Phase 2)
- `GET https://www.reddit.com/search.json?q=<query>&sort=new&limit=25`
- No API key needed for public JSON endpoints
- Set User-Agent header to avoid 429s: `User-Agent: oddsense/0.1.0`
- Returns: data.children[].data.{title, selftext, score, num_comments, created_utc}

---

## Coding Conventions

- Use `anyhow::Result` in binary code, `thiserror` for custom errors in lib code
- All public structs: `#[derive(Debug, Clone, Serialize, Deserialize)]`
- Async everywhere with `tokio` runtime
- Each command handler is an async fn in its own file under `src/cli/`
- Data on stdout, human messages on stderr
- Never hardcode print — always go through the output module with `--format` support
- Use `#[cfg(test)]` for unit tests in each module
- Integration tests in `tests/integration/`
- Fixtures for all external data sources in `tests/fixtures/`
- Run `cargo clippy` and `cargo fmt` before committing

---

## Example Agent Pipelines

These demonstrate Karpathy's "module of bigger pipelines" vision:

```bash
# Agent builds a divergence report
oddsense divergence "AI" --format json --explain | jq '.[] | select(.signal_strength == "strong")'

# Cross-reference with GitHub activity
gh search repos "AI agent" --sort stars --json name,stars | \
  jq -r '.[].name' | head -5 | \
  xargs -I {} oddsense enrich "{}" --format json

# Daily alpha email pipeline
echo "# Daily Vibe Report — $(date)" > /tmp/report.md
echo "## Strong Divergences" >> /tmp/report.md
oddsense divergence "" --min-score 50 --format table >> /tmp/report.md
echo "## Arbitrage Opportunities" >> /tmp/report.md
oddsense arbitrage --min-spread 8 --format table >> /tmp/report.md
echo "## Momentum" >> /tmp/report.md
oddsense signals --timeframe 24h --format table >> /tmp/report.md
cat /tmp/report.md | mail -s "Vibe Report" user@example.com

# Agent creates a web dashboard from CLI data
oddsense divergence "" --min-score 30 --format json > divergences.json
oddsense signals --format json > signals.json
# → agent reads these and builds a React dashboard

# Combine polymarket-cli trading with oddsense intelligence
# "Buy the strongest divergence where sentiment says underpriced"
MARKET=$(oddsense divergence "" --format json --limit 1 | jq -r '.[0].market.id')
polymarket clob market-order --token $MARKET --side buy --amount 5
```

---

## Definition of Done (v0.1.0) — COMPLETE

- [x] `oddsense enrich <query>` returns sentiment scores from news + reddit
- [x] `oddsense divergence <query>` compares polymarket odds vs sentiment
- [x] `oddsense signals` shows momentum and volume-weighted market movers
- [ ] `oddsense dashboard` launches a basic TUI with divergence + signals panels
- [x] Polymarket adapter works via polymarket-cli subprocess with proper error handling
- [x] All commands support `--format json` with documented schemas
- [ ] All commands support `--stdin` for piped composition
- [x] Proper exit codes (0=success, 1=error)
- [x] stderr/stdout separation works correctly
- [x] Config file with API key management
- [ ] Basic TTL caching for sentiment results
- [x] SKILL.md is complete
- [x] README.md with install, quickstart, and pipeline examples

## Definition of Done (v0.2.0) — COMPLETE

- [x] Kalshi adapter (direct API) with semantic search via expanded_relevance_score()
- [x] Metaculus adapter (direct API)
- [x] `oddsense arbitrage` cross-platform spread detection
- [x] `oddsense compare` side-by-side platform comparison
- [x] Fuzzy matching for cross-platform question pairing (Jaro-Winkler)
- [x] Semantic search with synonym expansion (~50 domain groups)
- [x] `--category` filter for market categorization
- [x] `--smart` flag for LLM-powered query expansion and result reranking
- [x] Negation-aware sentiment scoring
- [x] Expired market filtering
- [x] Published to crates.io + GitHub
- [ ] Correlation cluster detection in signals
- [ ] Full TUI dashboard with all 4 panels

## Definition of Done (v0.3.0)

- [ ] Correlation cluster detection in signals
- [ ] Full TUI dashboard with all 4 panels
- [ ] Basic TTL caching for sentiment results
- [ ] `--stdin` for piped composition
- [ ] OpenAI provider support for `--smart`
- [ ] Prebuilt binaries via GitHub releases
