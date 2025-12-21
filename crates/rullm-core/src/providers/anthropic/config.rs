use crate::config::ProviderConfig;
use crate::error::LlmError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Anthropic-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub timeout_seconds: u64,
    /// Whether to use OAuth authentication (Bearer token) instead of API key (x-api-key)
    #[serde(default)]
    pub use_oauth: bool,
}

impl AnthropicConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            timeout_seconds: 30,
            use_oauth: false,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn with_oauth(mut self, use_oauth: bool) -> Self {
        self.use_oauth = use_oauth;
        self
    }
}

impl ProviderConfig for AnthropicConfig {
    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn base_url(&self) -> &str {
        self.base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com")
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }

    fn headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        if self.use_oauth {
            // OAuth: use Bearer token + required beta headers
            headers.insert(
                "Authorization".to_string(),
                format!("Bearer {}", self.api_key),
            );
            headers.insert(
                "anthropic-beta".to_string(),
                "oauth-2025-04-20,claude-code-20250219,interleaved-thinking-2025-05-14,fine-grained-tool-streaming-2025-05-14".to_string(),
            );
            headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        } else {
            // API key: use x-api-key header
            headers.insert("x-api-key".to_string(), self.api_key.clone());
            headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        }

        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers
    }

    fn validate(&self) -> Result<(), LlmError> {
        if self.api_key.is_empty() {
            return Err(LlmError::configuration("Anthropic API key is required"));
        }

        Ok(())
    }
}
