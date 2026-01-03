use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::providers::{AnthropicConfig, OpenAICompatibleConfig, OpenAIConfig};

/// Configuration trait for LLM providers
pub trait ProviderConfig: Send + Sync {
    /// Get the API key for this provider
    fn api_key(&self) -> &str;

    /// Get the base URL for API requests
    fn base_url(&self) -> &str;

    /// Get default request timeout
    fn timeout(&self) -> Duration;

    /// Get any additional headers required by the provider
    fn headers(&self) -> HashMap<String, String>;

    /// Validate the configuration
    fn validate(&self) -> Result<(), crate::error::LlmError>;
}

/// Generic configuration for HTTP-based providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpProviderConfig {
    pub api_key: String,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub headers: HashMap<String, String>,
}

impl HttpProviderConfig {
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            timeout_seconds: 30,
            headers: HashMap::new(),
        }
    }

    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

impl ProviderConfig for HttpProviderConfig {
    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }

    fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    fn validate(&self) -> Result<(), crate::error::LlmError> {
        if self.api_key.is_empty() {
            return Err(crate::error::LlmError::configuration("API key is required"));
        }

        if self.base_url.is_empty() {
            return Err(crate::error::LlmError::configuration(
                "Base URL is required",
            ));
        }

        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err(crate::error::LlmError::configuration(
                "Base URL must be a valid HTTP/HTTPS URL",
            ));
        }

        Ok(())
    }
}

/// Configuration builder for creating provider configs from environment variables
pub struct ConfigBuilder;

impl ConfigBuilder {
    /// Create OpenAI config from environment
    pub fn openai_from_env() -> Result<OpenAIConfig, crate::error::LlmError> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            crate::error::LlmError::configuration("OPENAI_API_KEY environment variable not set")
        })?;

        let mut config = OpenAIConfig::new(api_key);

        if let Ok(org) = std::env::var("OPENAI_ORGANIZATION") {
            config = config.with_organization(org);
        }

        if let Ok(project) = std::env::var("OPENAI_PROJECT") {
            config = config.with_project(project);
        }

        if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
            config = config.with_base_url(base_url);
        }

        config.validate()?;
        Ok(config)
    }

    /// Create Anthropic config from environment
    pub fn anthropic_from_env() -> Result<AnthropicConfig, crate::error::LlmError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
            crate::error::LlmError::configuration("ANTHROPIC_API_KEY environment variable not set")
        })?;

        let mut config = AnthropicConfig::new(api_key);

        if let Ok(base_url) = std::env::var("ANTHROPIC_BASE_URL") {
            config = config.with_base_url(base_url);
        }

        config.validate()?;
        Ok(config)
    }

    /// Create Groq config from environment
    pub fn groq_from_env() -> Result<OpenAICompatibleConfig, crate::error::LlmError> {
        let api_key = std::env::var("GROQ_API_KEY").map_err(|_| {
            crate::error::LlmError::configuration("GROQ_API_KEY environment variable not set")
        })?;

        let mut config = OpenAICompatibleConfig::groq(api_key);

        if let Ok(base_url) = std::env::var("GROQ_BASE_URL") {
            config = config.with_base_url(base_url);
        }

        config.validate()?;
        Ok(config)
    }

    /// Create OpenRouter config from environment
    pub fn openrouter_from_env() -> Result<OpenAICompatibleConfig, crate::error::LlmError> {
        let api_key = std::env::var("OPENROUTER_API_KEY").map_err(|_| {
            crate::error::LlmError::configuration("OPENROUTER_API_KEY environment variable not set")
        })?;

        let mut config = OpenAICompatibleConfig::openrouter(api_key);

        if let Ok(base_url) = std::env::var("OPENROUTER_BASE_URL") {
            config = config.with_base_url(base_url);
        }

        config.validate()?;
        Ok(config)
    }
}
