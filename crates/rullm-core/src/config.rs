use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

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

/// OpenAI-compatible configuration (supports OpenAI, Groq, OpenRouter, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompatibleConfig {
    pub api_key: String,
    pub organization: Option<String>,
    pub project: Option<String>,
    pub base_url: Option<String>,
    pub timeout_seconds: u64,
}

/// Type alias for backwards compatibility
pub type OpenAIConfig = OpenAICompatibleConfig;

impl OpenAICompatibleConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            organization: None,
            project: None,
            base_url: None,
            timeout_seconds: 30,
        }
    }

    pub fn groq(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            organization: None,
            project: None,
            base_url: Some("https://api.groq.com/openai/v1".to_string()),
            timeout_seconds: 30,
        }
    }

    pub fn openrouter(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            organization: None,
            project: None,
            base_url: Some("https://openrouter.ai/api/v1".to_string()),
            timeout_seconds: 30,
        }
    }

    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }

    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }
}

impl ProviderConfig for OpenAICompatibleConfig {
    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn base_url(&self) -> &str {
        self.base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1")
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }

    fn headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.api_key),
        );
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        if let Some(org) = &self.organization {
            headers.insert("OpenAI-Organization".to_string(), org.clone());
        }

        if let Some(project) = &self.project {
            headers.insert("OpenAI-Project".to_string(), project.clone());
        }

        headers
    }

    fn validate(&self) -> Result<(), crate::error::LlmError> {
        if self.api_key.is_empty() {
            return Err(crate::error::LlmError::configuration("API key is required"));
        }

        // Relaxed validation: don't require 'sk-' prefix since Groq and OpenRouter use different formats
        // OpenAI keys start with 'sk-', Groq uses 'gsk_', OpenRouter uses different format

        Ok(())
    }
}

/// Anthropic-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub timeout_seconds: u64,
}

impl AnthropicConfig {
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
        headers.insert("x-api-key".to_string(), self.api_key.clone());
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        headers
    }

    fn validate(&self) -> Result<(), crate::error::LlmError> {
        if self.api_key.is_empty() {
            return Err(crate::error::LlmError::configuration(
                "Anthropic API key is required",
            ));
        }

        Ok(())
    }
}

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

    fn validate(&self) -> Result<(), crate::error::LlmError> {
        if self.api_key.is_empty() {
            return Err(crate::error::LlmError::configuration(
                "Google AI API key is required",
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

    /// Create Google AI config from environment
    pub fn google_ai_from_env() -> Result<GoogleAiConfig, crate::error::LlmError> {
        let api_key = std::env::var("GOOGLE_AI_API_KEY").map_err(|_| {
            crate::error::LlmError::configuration("GOOGLE_AI_API_KEY environment variable not set")
        })?;

        let mut config = GoogleAiConfig::new(api_key);

        if let Ok(base_url) = std::env::var("GOOGLE_AI_BASE_URL") {
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
