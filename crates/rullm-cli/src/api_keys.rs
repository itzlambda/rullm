use crate::provider::Provider;
use rullm_core::error::LlmError;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Default, Deserialize, Serialize, Debug, Clone)]
pub struct ApiKeys {
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub google_ai_api_key: Option<String>,
}

impl ApiKeys {
    /// Load API keys from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, LlmError> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| LlmError::validation(format!("Failed to read API keys config: {e}")))?;

        // Handle empty files gracefully
        if content.trim().is_empty() {
            return Ok(Self::default());
        }

        toml::from_str(&content)
            .map_err(|e| LlmError::validation(format!("Failed to parse API keys config: {e}")))
    }

    /// Save API keys to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), LlmError> {
        let path = path.as_ref();

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                LlmError::validation(format!("Failed to create data directory: {e}"))
            })?;
        }

        let content = toml::to_string_pretty(self).map_err(|e| {
            LlmError::validation(format!("Failed to serialize API keys config: {e}"))
        })?;

        std::fs::write(path, content)
            .map_err(|e| LlmError::validation(format!("Failed to write API keys config: {e}")))
    }

    pub fn get_api_key(provider: &Provider, api_keys: &ApiKeys) -> Option<String> {
        let key = match provider {
            Provider::OpenAI => api_keys.openai_api_key.as_ref(),
            Provider::Anthropic => api_keys.anthropic_api_key.as_ref(),
            Provider::Google => api_keys.google_ai_api_key.as_ref(),
        };

        key.cloned()
            .or_else(|| std::env::var(provider.env_key()).ok())
    }

    pub fn set_api_key_for_provider(provider: &Provider, api_keys: &mut ApiKeys, key: &str) {
        match provider {
            Provider::OpenAI => api_keys.openai_api_key = Some(key.to_string()),
            Provider::Anthropic => api_keys.anthropic_api_key = Some(key.to_string()),
            Provider::Google => api_keys.google_ai_api_key = Some(key.to_string()),
        }
    }

    pub fn delete_api_key_for_provider(provider: &Provider, api_keys: &mut ApiKeys) {
        match provider {
            Provider::OpenAI => api_keys.openai_api_key = None,
            Provider::Anthropic => api_keys.anthropic_api_key = None,
            Provider::Google => api_keys.google_ai_api_key = None,
        }
    }
}
