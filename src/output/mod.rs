pub mod json;
pub mod table;

use serde::Serialize;

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Table,
}

impl OutputFormat {
    pub fn from_str_opt(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            _ => OutputFormat::Table,
        }
    }
}

/// Render data to stdout in the requested format.
pub fn render<T: Serialize + table::TableRenderable>(
    data: &T,
    format: OutputFormat,
    quiet: bool,
    raw: bool,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => json::render_json(data, raw),
        OutputFormat::Table => {
            if quiet {
                // In quiet mode, output nothing for table format
                Ok(())
            } else {
                table::render_table(data)
            }
        }
    }
}
