mod adapters;
mod analysis;
mod cli;
mod config;
mod llm;
mod output;
mod search;
mod sentiment;

use clap::Parser;
use cli::{Cli, Commands};
use output::OutputFormat;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let format = OutputFormat::from_str_opt(&cli.format);
    let config_path = cli.config.as_deref();

    let result = match cli.command {
        Commands::Search {
            ref query,
            ref sources,
            ref category,
            limit,
            ref sort,
        } => {
            cli::search::run(
                query,
                sources,
                category.as_deref(),
                limit,
                sort,
                format,
                cli.quiet,
                cli.raw,
                cli.smart,
                config_path,
            )
            .await
        }

        Commands::Enrich {
            ref query,
            ref sources,
        } => {
            cli::enrich::run(query, sources, format, cli.quiet, cli.raw, config_path).await
        }

        Commands::Divergence {
            ref query,
            ref sentiment,
            min_score,
            limit,
            explain,
        } => {
            cli::divergence::run(
                query,
                sentiment,
                min_score,
                limit,
                explain,
                format,
                cli.quiet,
                cli.raw,
                config_path,
            )
            .await
        }

        Commands::Signals {
            ref timeframe,
            min_volume,
            limit,
        } => cli::signals::run(timeframe, min_volume, limit, format, cli.quiet, cli.raw).await,

        Commands::Arbitrage {
            ref query,
            ref sources,
            min_spread,
            similarity,
            limit,
        } => {
            cli::arbitrage::run(
                query.as_deref(),
                sources,
                min_spread,
                similarity,
                limit,
                format,
                cli.quiet,
                cli.raw,
            )
            .await
        }

        Commands::Compare {
            ref query,
            ref sources,
            similarity,
            limit,
        } => {
            cli::compare::run(query, sources, similarity, limit, format, cli.quiet, cli.raw)
                .await
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
