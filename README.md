# vibe-dash

**Agent-native CLI for prediction market intelligence.**

Aggregates data from Polymarket, Kalshi, and Metaculus. Cross-references with real-time sentiment from news and Reddit. Surfaces divergences where crowds disagree with real-world signals and arbitrage opportunities across platforms.

Built for agents. Every command outputs structured JSON, is composable via unix pipes, and ships with a SKILL.md so AI agents (Claude Code, Codex, etc.) can use it out of the box.

## Install

### From crates.io

```bash
cargo install vibe-dash
```

### From source

```bash
git clone https://github.com/jayeshvpatil/vibe-dash.git
cd vibe-dash
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
vibe-dash search "bitcoin" --limit 5

# Get sentiment signals
vibe-dash enrich "AI regulation" --sources reddit

# Find where market odds diverge from sentiment
vibe-dash divergence "Trump" --explain

# Surface trending markets by volume
vibe-dash signals --min-volume 50000

# Find cross-platform arbitrage (Polymarket vs Kalshi)
vibe-dash arbitrage --min-spread 5

# Compare the same question across platforms
vibe-dash compare "Harvey Weinstein" --sources polymarket,kalshi
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

Create `~/.config/vibe-dash/config.toml` (macOS: `~/Library/Application Support/com.vibe-dash.vibe-dash/config.toml`):

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
vibe-dash divergence "AI" --format json --quiet --raw | \
  jq '.divergences[] | select(.divergence_score > 50)'

# Agent pipeline: cross-platform spread analysis
vibe-dash arbitrage --format json --quiet --raw | \
  jq '.opportunities[] | {topic, spread, highest: .highest.source, lowest: .lowest.source}'

# Combine with other CLIs
vibe-dash search "crypto" --format json --quiet --raw | \
  jq -r '.markets[].title' | head -5
```

See [SKILL.md](SKILL.md) for full agent instructions.

## Architecture

vibe-dash follows a **composition over reimplementation** pattern:

- **Polymarket**: Shells out to `polymarket-cli` (subprocess), parses JSON output
- **Kalshi**: Direct REST API client (`https://api.elections.kalshi.com/trade-api/v2`)
- **Metaculus**: Direct REST API client (requires auth)
- **Sentiment**: NewsAPI + Reddit public API with keyword-based scoring
- **Arbitrage**: Jaro-Winkler fuzzy title matching across platforms

```
vibe-dash
  |- adapters/        # Market source adapters (polymarket, kalshi, metaculus)
  |- analysis/        # Divergence detection, arbitrage matching
  |- sentiment/       # News + Reddit sentiment scoring
  |- cli/             # Command handlers
  |- output/          # JSON + table formatters
  '- config.rs        # Config file management
```

## License

MIT
