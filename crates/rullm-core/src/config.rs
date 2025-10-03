use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryPolicy {
    /// Fixed delay between retries
    Fixed { delay_ms: u64 },
    /// Exponential backoff with optional jitter
    ExponentialBackoff {
        initial_delay_ms: u64,
        max_delay_ms: u64,
        multiplier: f64,
        jitter: bool,
    },
    /// Respect API-provided retry timing from response headers, with fallback policy
    ApiGuided {
        /// Fallback policy when no API guidance is available
        fallback: Box<RetryPolicy>,
        /// Maximum time to wait based on API guidance (prevents indefinite waits)
        max_api_delay_ms: u64,
        /// Headers to check for retry timing (in order of preference)
        retry_headers: Vec<String>,
    },
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy::ApiGuided {
            fallback: Box::new(RetryPolicy::ExponentialBackoff {
                initial_delay_ms: 1000,
                max_delay_ms: 30000,
                multiplier: 2.0,
                jitter: true,
            }),
            max_api_delay_ms: 60000, // Max 1 minute wait from API guidance
            retry_headers: vec![
                "retry-after".to_string(),
                "x-ratelimit-reset".to_string(),
                "x-ratelimit-reset-after".to_string(),
                "reset-time".to_string(),
            ],
        }
    }
}

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

    /// Get maximum number of retry attempts
    fn max_retries(&self) -> u32;

    /// Get retry policy configuration
    fn retry_policy(&self) -> RetryPolicy;

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
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl HttpProviderConfig {
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            timeout_seconds: 30,
            headers: HashMap::new(),
            max_retries: 3,
            retry_delay_ms: 1000,
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

    pub fn with_retries(mut self, max_retries: u32, delay_ms: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_delay_ms = delay_ms;
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

    fn max_retries(&self) -> u32 {
        self.max_retries
    }

    fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy::Fixed {
            delay_ms: self.retry_delay_ms,
        }
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
    pub max_retries: u32,
    pub retry_policy: RetryPolicy,
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
            max_retries: 3,
            retry_policy: RetryPolicy::default(),
        }
    }

    pub fn groq(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            organization: None,
            project: None,
            base_url: Some("https://api.groq.com/openai/v1".to_string()),
            timeout_seconds: 30,
            max_retries: 3,
            retry_policy: RetryPolicy::default(),
        }
    }

    pub fn openrouter(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            organization: None,
            project: None,
            base_url: Some("https://openrouter.ai/api/v1".to_string()),
            timeout_seconds: 30,
            max_retries: 3,
            retry_policy: RetryPolicy::default(),
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

    pub fn with_retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    pub fn with_fixed_retry(mut self, max_retries: u32, delay_ms: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_policy = RetryPolicy::Fixed { delay_ms };
        self
    }

    pub fn with_exponential_backoff(
        mut self,
        max_retries: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
        multiplier: f64,
        jitter: bool,
    ) -> Self {
        self.max_retries = max_retries;
        self.retry_policy = RetryPolicy::ExponentialBackoff {
            initial_delay_ms,
            max_delay_ms,
            multiplier,
            jitter,
        };
        self
    }

    pub fn with_api_guided_retry(
        mut self,
        max_retries: u32,
        fallback_policy: RetryPolicy,
        max_api_delay_ms: u64,
    ) -> Self {
        self.max_retries = max_retries;
        self.retry_policy = RetryPolicy::ApiGuided {
            fallback: Box::new(fallback_policy),
            max_api_delay_ms,
            retry_headers: vec![
                "retry-after".to_string(),
                "x-ratelimit-reset".to_string(),
                "x-ratelimit-reset-after".to_string(),
                "reset-time".to_string(),
            ],
        };
        self
    }

    pub fn with_smart_retry(mut self, max_retries: u32) -> Self {
        // Smart retry: API-guided with exponential backoff fallback
        self.max_retries = max_retries;
        self.retry_policy = RetryPolicy::default();
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

    fn max_retries(&self) -> u32 {
        self.max_retries
    }

    fn retry_policy(&self) -> RetryPolicy {
        self.retry_policy.clone()
    }

    fn validate(&self) -> Result<(), crate::error::LlmError> {
        if self.api_key.is_empty() {
            return Err(crate::error::LlmError::configuration(
                "API key is required",
            ));
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
    pub max_retries: u32,
    pub retry_policy: RetryPolicy,
}

impl AnthropicConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            timeout_seconds: 30,
            max_retries: 3,
            retry_policy: RetryPolicy::default(),
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

    fn max_retries(&self) -> u32 {
        self.max_retries
    }

    fn retry_policy(&self) -> RetryPolicy {
        self.retry_policy.clone()
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
    pub max_retries: u32,
    pub retry_policy: RetryPolicy,
}

impl GoogleAiConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: None,
            timeout_seconds: 30,
            max_retries: 3,
            retry_policy: RetryPolicy::default(),
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

    fn max_retries(&self) -> u32 {
        self.max_retries
    }

    fn retry_policy(&self) -> RetryPolicy {
        self.retry_policy.clone()
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

/// Utility functions for parsing retry timing from HTTP headers
pub mod retry_parsing {
    use super::*;
    use std::str::FromStr;

    /// Parse retry delay from HTTP response headers
    pub fn parse_retry_delay(
        headers: &HashMap<String, String>,
        retry_headers: &[String],
        max_delay_ms: u64,
    ) -> Option<Duration> {
        for header_name in retry_headers {
            if let Some(value) = headers.get(header_name) {
                if let Some(delay) = parse_single_header(header_name, value, max_delay_ms) {
                    return Some(delay);
                }
            }
        }
        None
    }

    fn parse_single_header(header_name: &str, value: &str, max_delay_ms: u64) -> Option<Duration> {
        match header_name.to_lowercase().as_str() {
            "retry-after" => parse_retry_after(value, max_delay_ms),
            "x-ratelimit-reset" => parse_reset_timestamp(value, max_delay_ms),
            "x-ratelimit-reset-after" => parse_seconds(value, max_delay_ms),
            "reset-time" => parse_reset_timestamp(value, max_delay_ms),
            _ => None,
        }
    }

    fn parse_retry_after(value: &str, max_delay_ms: u64) -> Option<Duration> {
        // Retry-After can be either seconds or HTTP date
        if let Ok(seconds) = u64::from_str(value.trim()) {
            let delay_ms = seconds * 1000;
            if delay_ms <= max_delay_ms {
                return Some(Duration::from_millis(delay_ms));
            }
        }
        // TODO: Add HTTP date parsing if needed
        None
    }

    fn parse_reset_timestamp(value: &str, max_delay_ms: u64) -> Option<Duration> {
        if let Ok(timestamp) = u64::from_str(value.trim()) {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();

            if timestamp > now {
                let delay_seconds = timestamp - now;
                let delay_ms = delay_seconds * 1000;
                if delay_ms <= max_delay_ms {
                    return Some(Duration::from_millis(delay_ms));
                }
            }
        }
        None
    }

    fn parse_seconds(value: &str, max_delay_ms: u64) -> Option<Duration> {
        if let Ok(seconds) = u64::from_str(value.trim()) {
            let delay_ms = seconds * 1000;
            if delay_ms <= max_delay_ms {
                return Some(Duration::from_millis(delay_ms));
            }
        }
        None
    }
}
