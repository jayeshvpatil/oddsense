# oddsense

**Agent-native CLI for prediction market intelligence.**

oddsense is an agent-native CLI intelligence layer for prediction markets. It does NOT re-implement market data fetching. Instead, it composes with existing CLIs (starting with polymarket-cli) and adds the intelligence layer on top: cross-platform aggregation, real-world sentiment analysis, divergence detection, and arbitrage discovery.

Think of it as: **polymarket-cli is the data pipe. oddsense is the brain.**

### Core Philosophy (from Karpathy)

> "CLIs are super exciting precisely because they are a 'legacy' technology, which means AI agents can natively and easily use them, combine them... Even more powerful when you use it as a module of bigger pipelines."

oddsense embodies this. It's a CLI that agents can install, compose, and build dashboards/apps on top of. It treats polymarket-cli (and future CLIs like kalshi, metaculus) as upstream data sources and focuses exclusively on what they don't provide: analysis, enrichment, signals, and cross-source intelligence.

Every command outputs structured JSON, is composable via unix pipes, and ships with a SKILL.md so AI agents (Claude Code, Codex, etc.) can use it out of the box.


## Install

### From crates.io

```bash
cargo install oddsense
```

### From source

```bash
git clone https://github.com/jayeshvpatil/oddsense.git
cd oddsense
cargo install --path .
```

### Prerequisites

- [polymarket-cli](https://github.com/Polymarket/polymarket-cli) — required for Polymarket data:
  ```bash
  cargo install --git https://github.com/Polymarket/polymarket-cli.git
  ```
- (Optional) [NewsAPI key](https://newsapi.org/) — for news sentiment in `enrich` and `divergence` commands

## Quick Start

```bash
# Search prediction markets
oddsense search "bitcoin" --limit 5

# Get sentiment signals
oddsense enrich "AI regulation" --sources reddit

# Find where market odds diverge from sentiment
oddsense divergence "Trump" --explain

# Surface trending markets by volume
oddsense signals --min-volume 50000

# Find cross-platform arbitrage (Polymarket vs Kalshi)
oddsense arbitrage --min-spread 5

# Compare the same question across platforms
oddsense compare "Harvey Weinstein" --sources polymarket,kalshi
```

## Commands

| Command | Description |
|---|---|
| `search <query>` | Search prediction markets via polymarket-cli |
| `enrich <query>` | Fetch sentiment signals (news + Reddit) |
| `divergence <query>` | Find markets where odds diverge from sentiment |
| `signals` | Surface trending markets by volume/momentum |
| `arbitrage [query]` | Find cross-platform pricing differences |
| `compare <query>` | Side-by-side comparison across platforms |

### Global Flags

```
--format json|table   Output format (default: table)
--quiet, -q           Suppress non-data output (stderr)
--raw                 Raw JSON, no pretty-printing
--config <path>       Custom config file path
```

## Configuration

Create `~/.config/oddsense/config.toml` (macOS: `~/Library/Application Support/com.oddsense.oddsense/config.toml`):

```toml
[api_keys]
newsapi = "your-newsapi-key-here"

[defaults]
format = "table"
refresh_seconds = 60
sources = ["polymarket"]
```

### API Keys

| Source | Key Required? | How to get |
|---|---|---|
| Polymarket | No | Free via polymarket-cli |
| Kalshi | No | Free public API |
| Reddit | No | Free public JSON API |
| NewsAPI | Yes | [newsapi.org](https://newsapi.org/) (free tier: 100 req/day) |
| Metaculus | Yes | API requires auth (gracefully skipped if unavailable) |

## Agent Usage

This CLI is designed for AI agents. Every command supports `--format json --quiet --raw` for clean machine-readable output:

```bash
# Agent pipeline: find divergences and extract actionable ones
oddsense divergence "AI" --format json --quiet --raw | \
  jq '.divergences[] | select(.divergence_score > 50)'

# Agent pipeline: cross-platform spread analysis
oddsense arbitrage --format json --quiet --raw | \
  jq '.opportunities[] | {topic, spread, highest: .highest.source, lowest: .lowest.source}'

# Combine with other CLIs
oddsense search "crypto" --format json --quiet --raw | \
  jq -r '.markets[].title' | head -5
```

See [SKILL.md](SKILL.md) for full agent instructions.

## Architecture

oddsense follows a **composition over reimplementation** pattern. For Polymarket, we shell out to `polymarket-cli` and parse its JSON output — no need to reimplement auth, CLOB interaction, or API wrappers. For platforms without existing CLIs (Kalshi, Metaculus), we call their APIs directly.

```
┌─────────────────────────────────────────────────────────┐
│                      oddsense                          │
│                  (intelligence layer)                   │
│                                                         │
│  ┌──────────┐  ┌────────────┐  ┌─────────────────────┐  │
│  │ Sentiment│  │ Divergence │  │   Cross-Platform    │  │
│  │ Engine   │  │ Detector   │  │   Arbitrage Finder  │  │
│  └────┬─────┘  └─────┬──────┘  └──────────┬──────────┘  │
│       │              │                     │             │
│  ┌────┴──────────────┴─────────────────────┴──────────┐  │
│  │              Unified Market Schema                 │  │
│  │         (normalize all sources into one)           │  │
│  └────┬──────────────┬─────────────────────┬──────────┘  │
│       │              │                     │             │
│  ┌────┴────┐   ┌─────┴─────┐   ┌──────────┴──────────┐  │
│  │Polymarket│  │  Kalshi   │   │   Metaculus         │  │
│  │ Adapter  │  │  Adapter  │   │   Adapter           │  │
│  │(via CLI) │  │ (via API) │   │   (via API)         │  │
│  └──────────┘  └───────────┘   └─────────────────────┘  │
└─────────────────────────────────────────────────────────┘
        │               │                    │
   polymarket-cli    Kalshi API         Metaculus API
   (subprocess)      (direct HTTP)      (direct HTTP)
```

### Source Adapters

| Adapter | Method | Auth |
|---|---|---|
| **Polymarket** | Shells out to `polymarket-cli` subprocess, parses JSON | None (public) |
| **Kalshi** | Direct REST API (`api.elections.kalshi.com`) | None (read-only) |
| **Metaculus** | Direct REST API (`metaculus.com/api2`) | Required (skipped if unavailable) |
| **Sentiment** | NewsAPI + Reddit public JSON API | NewsAPI key (optional) |
| **Arbitrage** | Jaro-Winkler fuzzy title matching across all sources | N/A |

### Project Structure

```
src/
  adapters/        # Market source adapters (polymarket, kalshi, metaculus)
  analysis/        # Divergence detection, arbitrage matching
  sentiment/       # News + Reddit sentiment scoring
  cli/             # Command handlers
  output/          # JSON + table formatters
  config.rs        # Config file management
```

## Use Cases

**Developers / Builders**
- *"Build me a dashboard showing all AI-related markets above $1M volume"* — an agent builds it in minutes using `oddsense search` + `signals`
- *"Monitor these 5 markets and Slack me when odds shift more than 10%"* — an agent writes the script with `oddsense` in a cron loop

**Researchers / Analysts**
- *"Pull all election markets, cross-reference with polling data, show me where they diverge"* — `oddsense divergence` does exactly this
- *"Give me a weekly report of the biggest movers in crypto prediction markets"* — pipe `oddsense signals` into a report template

**Traders**
- *"Compare odds across Polymarket and Kalshi for arbitrage"* — `oddsense arbitrage` finds the cross-platform spreads
- *"Watch these markets and alert me when probability drops below 30%"* — combine `oddsense search --format json` with a threshold script

**Content Creators**
- *"Generate a daily Twitter thread of the most interesting prediction market moves"* — an agent composes `oddsense signals` + `enrich` into a thread

## License

MIT
