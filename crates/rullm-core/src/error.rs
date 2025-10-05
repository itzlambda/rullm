use std::collections::HashMap;
use thiserror::Error;

/// Main error type for the LLM library
#[derive(Error, Debug)]
pub enum LlmError {
    /// Network-related errors
    #[error("Network error: {message}")]
    Network {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Authentication errors
    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    /// Rate limiting errors
    #[error("Rate limit exceeded: {message}. Retry after: {retry_after:?}")]
    RateLimit {
        message: String,
        retry_after: Option<std::time::Duration>,
    },

    /// Provider-specific API errors
    #[error("API error from {provider}: {message} (code: {code:?})")]
    Api {
        provider: String,
        message: String,
        code: Option<String>,
        details: Option<HashMap<String, serde_json::Value>>,
    },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Validation errors for requests
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Timeout errors
    #[error("Request timed out after {duration:?}")]
    Timeout { duration: std::time::Duration },

    /// Serialization/deserialization errors
    #[error("Serialization error: {message}")]
    Serialization {
        message: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Model-specific errors (model not found, unsupported, etc.)
    #[error("Model error: {message}")]
    Model { message: String },

    /// Resource errors (quota exceeded, insufficient credits, etc.)
    #[error("Resource error: {message}")]
    Resource { message: String },

    /// Provider service unavailable
    #[error("Service unavailable: {provider} is currently unavailable")]
    ServiceUnavailable { provider: String },

    /// Generic errors for cases not covered above
    #[error("Unexpected error: {message}")]
    Unknown {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl LlmError {
    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
            source: None,
        }
    }

    /// Create a network error with source
    pub fn network_with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Network {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// Create an authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    /// Create a rate limit error
    pub fn rate_limit(
        message: impl Into<String>,
        retry_after: Option<std::time::Duration>,
    ) -> Self {
        Self::RateLimit {
            message: message.into(),
            retry_after,
        }
    }

    /// Create an API error
    pub fn api(
        provider: impl Into<String>,
        message: impl Into<String>,
        code: Option<String>,
        details: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self::Api {
            provider: provider.into(),
            message: message.into(),
            code,
            details,
        }
    }

    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(duration: std::time::Duration) -> Self {
        Self::Timeout { duration }
    }

    /// Create a serialization error
    pub fn serialization(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Serialization {
            message: message.into(),
            source: source.into(),
        }
    }

    /// Create a model error
    pub fn model(message: impl Into<String>) -> Self {
        Self::Model {
            message: message.into(),
        }
    }

    /// Create a resource error
    pub fn resource(message: impl Into<String>) -> Self {
        Self::Resource {
            message: message.into(),
        }
    }

    /// Create a service unavailable error
    pub fn service_unavailable(provider: impl Into<String>) -> Self {
        Self::ServiceUnavailable {
            provider: provider.into(),
        }
    }

    /// Create an unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::Unknown {
            message: message.into(),
            source: None,
        }
    }

    /// Create an unknown error with source
    pub fn unknown_with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Unknown {
            message: message.into(),
            source: Some(source.into()),
        }
    }
}

/// Convert from reqwest errors
impl From<reqwest::Error> for LlmError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            LlmError::timeout(std::time::Duration::from_secs(30)) // Default timeout
        } else if err.is_connect() {
            LlmError::network_with_source("Connection failed", err)
        } else if err.is_request() {
            LlmError::validation(format!("Invalid request: {err}"))
        } else {
            LlmError::network_with_source("HTTP request failed", err)
        }
    }
}

/// Convert from serde_json errors
impl From<serde_json::Error> for LlmError {
    fn from(err: serde_json::Error) -> Self {
        LlmError::serialization("JSON serialization failed", err)
    }
}
