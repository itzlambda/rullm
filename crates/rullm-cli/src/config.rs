use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::constants::CONFIG_FILE_NAME;

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Default model to use when none is specified
    pub default_model: Option<String>,
    /// Enable vim mode in interactive chat
    pub vi_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_model: Some("openai/gpt-4o-mini".to_string()),
            vi_mode: false,
        }
    }
}

impl Config {
    pub fn load(base_path: &Path) -> Result<Config> {
        let config_path = base_path.join(CONFIG_FILE_NAME);

        if !config_path.exists() {
            // Create default config
            let default_config = Config::default();
            default_config.save(base_path)?;
            return Ok(default_config);
        }

        let content = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)?;

        Ok(config)
    }

    pub fn save(&self, base_path: &Path) -> Result<()> {
        let config_path = base_path.join(CONFIG_FILE_NAME);

        // Create directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }
}
