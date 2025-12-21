use crate::config::ProviderConfig;
use crate::error::LlmError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Google AI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleAiConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub timeout_seconds: u64,
}

impl GoogleAiConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            timeout_seconds: 30,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }
}

impl ProviderConfig for GoogleAiConfig {
    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn base_url(&self) -> &str {
        self.base_url
            .as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com/v1beta")
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }

    fn headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers
    }

    fn validate(&self) -> Result<(), LlmError> {
        if self.api_key.is_empty() {
            return Err(LlmError::configuration("Google AI API key is required"));
        }

        Ok(())
    }
}
