use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub api_keys: ApiKeys,
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeys {
    pub polymarket: Option<String>,
    pub newsapi: Option<String>,
    pub anthropic: Option<String>,
    pub openai: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmConfig {
    /// LLM provider: "anthropic" or "openai"
    pub provider: Option<String>,
    /// Model override (e.g. "claude-haiku-4-5-20251001")
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    pub format: String,
    pub refresh_seconds: u64,
    pub sources: Vec<String>,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            format: "table".to_string(),
            refresh_seconds: 60,
            sources: vec!["polymarket".to_string()],
        }
    }
}

/// Get the config file path (~/.config/oddsense/config.toml).
pub fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "oddsense", "oddsense")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}

/// Load config from disk, or return defaults if not found.
pub fn load_config(custom_path: Option<&str>) -> Result<Config> {
    let path = match custom_path {
        Some(p) => PathBuf::from(p),
        None => match config_path() {
            Some(p) => p,
            None => return Ok(Config::default()),
        },
    };

    if !path.exists() {
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}
