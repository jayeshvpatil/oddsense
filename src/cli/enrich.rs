use anyhow::Result;

use crate::config;
use crate::output::{self, OutputFormat};
use crate::sentiment;

pub async fn run(
    query: &str,
    sources: &str,
    format: OutputFormat,
    quiet: bool,
    raw: bool,
    config_path: Option<&str>,
) -> Result<()> {
    if !quiet {
        eprintln!("Analyzing sentiment for \"{}\"...", query);
    }

    let cfg = config::load_config(config_path)?;
    let newsapi_key = cfg.api_keys.newsapi.as_deref();

    let use_news = sources == "all" || sources == "news";
    let use_reddit = sources == "all" || sources == "reddit";

    let result = sentiment::aggregate_sentiment(query, use_news, use_reddit, newsapi_key).await?;

    output::render(&result, format, quiet, raw)?;
    Ok(())
}
