use crate::config::ProviderConfig;
use crate::error::LlmError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

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

    fn validate(&self) -> Result<(), LlmError> {
        if self.api_key.is_empty() {
            return Err(LlmError::configuration("API key is required"));
        }

        // Relaxed validation: don't require 'sk-' prefix since Groq and OpenRouter use different formats
        // OpenAI keys start with 'sk-', Groq uses 'gsk_', OpenRouter uses different format

        Ok(())
    }
}
