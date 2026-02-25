pub mod arbitrage;
pub mod compare;
pub mod divergence;
pub mod enrich;
pub mod search;
pub mod signals;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "vibe-dash",
    version,
    about = "Prediction market intelligence — sentiment, divergences, and signals",
    long_about = "Agent-native CLI that composes with polymarket-cli to add sentiment analysis, \
                  divergence detection, and cross-platform intelligence on top of prediction market data."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format: json or table
    #[arg(long, global = true, default_value = "table")]
    pub format: String,

    /// Suppress non-data output
    #[arg(long, short, global = true)]
    pub quiet: bool,

    /// Raw JSON output (no pretty-printing)
    #[arg(long, global = true)]
    pub raw: bool,

    /// Custom config file path
    #[arg(long, global = true)]
    pub config: Option<String>,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Verbose logging to stderr
    #[arg(long, short, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search prediction markets (passthrough to polymarket-cli + normalization)
    Search {
        /// Search query
        query: String,

        /// Max results
        #[arg(long, default_value_t = 10)]
        limit: usize,

        /// Sort by: volume_num, created_at (default: volume_num)
        #[arg(long, default_value = "volume_num")]
        sort: String,
    },

    /// Fetch sentiment signals for a topic
    Enrich {
        /// Topic to analyze
        query: String,

        /// Sentiment sources: news, reddit, all
        #[arg(long, default_value = "all")]
        sources: String,
    },

    /// Find markets where odds diverge from real-world sentiment
    Divergence {
        /// Topic to analyze
        query: String,

        /// Sentiment sources: news, reddit, all
        #[arg(long, default_value = "all")]
        sentiment: String,

        /// Minimum divergence score 0-100
        #[arg(long, default_value_t = 20.0)]
        min_score: f64,

        /// Max results
        #[arg(long, default_value_t = 10)]
        limit: usize,

        /// Include human-readable explanations
        #[arg(long)]
        explain: bool,
    },

    /// Surface trending markets and momentum signals
    Signals {
        /// Time window: 1h, 24h, 7d
        #[arg(long, default_value = "24h")]
        timeframe: String,

        /// Minimum 24h volume filter (USD)
        #[arg(long, default_value_t = 10000.0)]
        min_volume: f64,

        /// Max results
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },

    /// Find cross-platform arbitrage opportunities
    Arbitrage {
        /// Optional topic to focus on
        query: Option<String>,

        /// Comma-separated sources: polymarket,kalshi,metaculus,all
        #[arg(long, default_value = "all")]
        sources: String,

        /// Minimum spread in percentage points to surface
        #[arg(long, default_value_t = 5.0)]
        min_spread: f64,

        /// Title similarity threshold (0.0 to 1.0)
        #[arg(long, default_value_t = 0.7)]
        similarity: f64,

        /// Max results
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },

    /// Compare the same question across platforms (side-by-side)
    Compare {
        /// Topic to compare
        query: String,

        /// Comma-separated sources: polymarket,kalshi,metaculus,all
        #[arg(long, default_value = "all")]
        sources: String,

        /// Title similarity threshold (0.0 to 1.0)
        #[arg(long, default_value_t = 0.6)]
        similarity: f64,

        /// Max results per source
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
}
