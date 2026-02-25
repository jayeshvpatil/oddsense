use anyhow::Result;
use tabled::{Table, Tabled};

use crate::adapters::schema::{MarketResponse, NormalizedMarket};
use crate::analysis::arbitrage::{ArbitrageResponse, CompareResponse};
use crate::analysis::divergence::{DivergenceDirection, DivergenceResponse};
use crate::sentiment::SentimentResult;

/// Trait for types that can render as a table.
pub trait TableRenderable {
    fn render_table_output(&self) -> Result<()>;
}

// --- Market table ---

#[derive(Tabled)]
struct MarketRow {
    #[tabled(rename = "Title")]
    title: String,
    #[tabled(rename = "Prob %")]
    probability: String,
    #[tabled(rename = "24h Vol")]
    volume_24h: String,
    #[tabled(rename = "Total Vol")]
    volume_total: String,
    #[tabled(rename = "Change")]
    change: String,
    #[tabled(rename = "Source")]
    source: String,
}

impl From<&NormalizedMarket> for MarketRow {
    fn from(m: &NormalizedMarket) -> Self {
        Self {
            title: truncate(&m.title, 50),
            probability: format!("{:.1}%", m.probability * 100.0),
            volume_24h: format_volume(m.volume_24h),
            volume_total: format_volume(m.volume_total),
            change: format_change(m.price_change_24h),
            source: m.source.clone(),
        }
    }
}

impl TableRenderable for MarketResponse {
    fn render_table_output(&self) -> Result<()> {
        if self.markets.is_empty() {
            eprintln!("No markets found.");
            return Ok(());
        }
        let rows: Vec<MarketRow> = self.markets.iter().map(MarketRow::from).collect();
        println!("{}", Table::new(rows));
        eprintln!("\n{} market(s) found", self.count);
        Ok(())
    }
}

// --- Sentiment table ---

#[derive(Tabled)]
struct SentimentRow {
    #[tabled(rename = "Title")]
    title: String,
    #[tabled(rename = "Source")]
    source: String,
    #[tabled(rename = "Sentiment")]
    sentiment: String,
}

impl TableRenderable for SentimentResult {
    fn render_table_output(&self) -> Result<()> {
        if self.sample_signals.is_empty() {
            eprintln!("No signals found.");
            return Ok(());
        }
        let rows: Vec<SentimentRow> = self
            .sample_signals
            .iter()
            .map(|s| SentimentRow {
                title: truncate(&s.title, 60),
                source: s.source_name.clone(),
                sentiment: format_sentiment(s.sentiment),
            })
            .collect();
        println!("{}", Table::new(rows));
        eprintln!(
            "\nAggregate: {} | Confidence: {:.0}% | {} signal(s)",
            format_sentiment(self.score),
            self.confidence * 100.0,
            self.signal_count
        );
        Ok(())
    }
}

// --- Divergence table ---

#[derive(Tabled)]
struct DivergenceRow {
    #[tabled(rename = "Market")]
    title: String,
    #[tabled(rename = "Mkt %")]
    market_prob: String,
    #[tabled(rename = "Sent %")]
    sentiment_prob: String,
    #[tabled(rename = "Div")]
    divergence: String,
    #[tabled(rename = "Strength")]
    strength: String,
    #[tabled(rename = "Direction")]
    direction: String,
}

impl TableRenderable for DivergenceResponse {
    fn render_table_output(&self) -> Result<()> {
        if self.divergences.is_empty() {
            eprintln!("No divergences found above threshold.");
            return Ok(());
        }
        let rows: Vec<DivergenceRow> = self
            .divergences
            .iter()
            .map(|d| {
                let sentiment_prob = (d.sentiment.score + 1.0) / 2.0;
                DivergenceRow {
                    title: truncate(&d.market.title, 45),
                    market_prob: format!("{:.0}%", d.market.probability * 100.0),
                    sentiment_prob: format!("{:.0}%", sentiment_prob * 100.0),
                    divergence: format!("{:.0}", d.divergence_score),
                    strength: d.signal_strength.clone(),
                    direction: match d.direction {
                        DivergenceDirection::MarketHigher => "Mkt > Sent".to_string(),
                        DivergenceDirection::SentimentHigher => "Sent > Mkt".to_string(),
                    },
                }
            })
            .collect();
        println!("{}", Table::new(rows));
        eprintln!("\n{} divergence(s) found", self.count);
        Ok(())
    }
}

// --- Arbitrage table ---

#[derive(Tabled)]
struct ArbitrageRow {
    #[tabled(rename = "Topic")]
    topic: String,
    #[tabled(rename = "Highest")]
    highest: String,
    #[tabled(rename = "Lowest")]
    lowest: String,
    #[tabled(rename = "Spread")]
    spread: String,
    #[tabled(rename = "Similarity")]
    similarity: String,
}

impl TableRenderable for ArbitrageResponse {
    fn render_table_output(&self) -> Result<()> {
        if self.opportunities.is_empty() {
            eprintln!("No arbitrage opportunities found above {:.0}pp spread.", self.min_spread);
            return Ok(());
        }
        let rows: Vec<ArbitrageRow> = self
            .opportunities
            .iter()
            .map(|o| ArbitrageRow {
                topic: truncate(&o.topic, 40),
                highest: format!(
                    "{} {:.0}%",
                    o.highest.source,
                    o.highest.probability * 100.0
                ),
                lowest: format!(
                    "{} {:.0}%",
                    o.lowest.source,
                    o.lowest.probability * 100.0
                ),
                spread: format!("{:.1}pp", o.spread),
                similarity: format!("{:.0}%", o.similarity * 100.0),
            })
            .collect();
        println!("{}", Table::new(rows));
        eprintln!("\n{} arbitrage opportunity(ies) found", self.count);
        Ok(())
    }
}

// --- Compare table ---

#[derive(Tabled)]
struct CompareRow {
    #[tabled(rename = "Source")]
    source: String,
    #[tabled(rename = "Title")]
    title: String,
    #[tabled(rename = "Prob %")]
    probability: String,
    #[tabled(rename = "24h Vol")]
    volume_24h: String,
    #[tabled(rename = "End Date")]
    end_date: String,
}

impl TableRenderable for CompareResponse {
    fn render_table_output(&self) -> Result<()> {
        if self.markets.is_empty() {
            eprintln!("No markets found for \"{}\".", self.query);
            return Ok(());
        }
        let rows: Vec<CompareRow> = self
            .markets
            .iter()
            .map(|m| CompareRow {
                source: m.source.clone(),
                title: truncate(&m.title, 45),
                probability: format!("{:.1}%", m.probability * 100.0),
                volume_24h: format_volume(m.volume_24h),
                end_date: m
                    .end_date
                    .as_deref()
                    .map(|d| truncate(d, 10))
                    .unwrap_or_else(|| "-".to_string()),
            })
            .collect();
        println!("{}", Table::new(rows));
        eprintln!(
            "\n{} market(s) across {} source(s)",
            self.markets.len(),
            self.sources.len()
        );
        Ok(())
    }
}

/// Render any TableRenderable type.
pub fn render_table<T: TableRenderable>(data: &T) -> Result<()> {
    data.render_table_output()
}

// --- Helpers ---

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 3).collect();
        format!("{}...", truncated)
    }
}

fn format_volume(v: Option<f64>) -> String {
    match v {
        Some(v) if v >= 1_000_000.0 => format!("${:.1}M", v / 1_000_000.0),
        Some(v) if v >= 1_000.0 => format!("${:.1}K", v / 1_000.0),
        Some(v) if v > 0.0 => format!("${:.0}", v),
        _ => "-".to_string(),
    }
}

fn format_change(c: Option<f64>) -> String {
    match c {
        Some(c) if c > 0.0 => format!("+{:.1}%", c * 100.0),
        Some(c) if c < 0.0 => format!("{:.1}%", c * 100.0),
        _ => "-".to_string(),
    }
}

fn format_sentiment(s: f64) -> String {
    if s > 0.1 {
        format!("+{:.2} bullish", s)
    } else if s < -0.1 {
        format!("{:.2} bearish", s)
    } else {
        format!("{:.2} neutral", s)
    }
}
