use anyhow::Result;
use serde::Serialize;

/// Render any serializable data as JSON to stdout.
pub fn render_json<T: Serialize>(data: &T, raw: bool) -> Result<()> {
    let output = if raw {
        serde_json::to_string(data)?
    } else {
        serde_json::to_string_pretty(data)?
    };
    println!("{}", output);
    Ok(())
}
