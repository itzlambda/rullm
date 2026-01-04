//! Error types for the Anthropic client

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Main error type for the Anthropic client
#[derive(Error, Debug)]
pub enum AnthropicError {
    /// API error returned by Anthropic
    #[error("API error ({status}): {error}")]
    Api {
        status: StatusCode,
        request_id: Option<Arc<str>>,
        error: ErrorObject,
    },

    /// HTTP transport error
    #[error("Transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// JSON serialization/deserialization error
    #[error("Serialization error: {message}")]
    Serialization {
        message: Arc<str>,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Request timeout
    #[error("Request timed out")]
    Timeout,

    /// Invalid request configuration
    #[error("Invalid request: {0}")]
    InvalidRequest(Arc<str>),

    /// Configuration error (missing API key, etc.)
    #[error("Configuration error: {0}")]
    Configuration(Arc<str>),
}

impl AnthropicError {
    /// Create a serialization error
    pub fn serialization(
        message: impl Into<Arc<str>>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Serialization {
            message: message.into(),
            source: source.into(),
        }
    }

    /// Create an invalid request error
    pub fn invalid_request(message: impl Into<Arc<str>>) -> Self {
        Self::InvalidRequest(message.into())
    }

    /// Create a configuration error
    pub fn configuration(message: impl Into<Arc<str>>) -> Self {
        Self::Configuration(message.into())
    }

    /// Create an API error
    pub fn api(status: StatusCode, request_id: Option<Arc<str>>, error: ErrorObject) -> Self {
        Self::Api {
            status,
            request_id,
            error,
        }
    }

    /// Get the request ID if available
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::Api { request_id, .. } => request_id.as_deref(),
            _ => None,
        }
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Api { status, error, .. } => {
                // Rate limit (429) and overloaded (529) are retryable
                *status == StatusCode::TOO_MANY_REQUESTS
                    || status.as_u16() == 529
                    || error.error_type.as_ref() == "overloaded_error"
            }
            Self::Transport(e) => e.is_timeout() || e.is_connect(),
            Self::Timeout => true,
            _ => false,
        }
    }
}

/// Error object returned by the Anthropic API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject {
    /// Error type (e.g., "invalid_request_error", "authentication_error")
    #[serde(rename = "type")]
    pub error_type: Arc<str>,

    /// Human-readable error message
    pub message: Arc<str>,

    /// Parameter that caused the error (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<Arc<str>>,
}

impl std::fmt::Display for ErrorObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(param) = &self.param {
            write!(
                f,
                "{}: {} (param: {})",
                self.error_type, self.message, param
            )
        } else {
            write!(f, "{}: {}", self.error_type, self.message)
        }
    }
}

/// API error response wrapper
#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorResponse {
    /// Always "error" for error responses
    #[serde(rename = "type")]
    pub response_type: Arc<str>,

    /// The error details
    pub error: ErrorObject,
}

/// Result type alias for Anthropic operations
pub type Result<T> = std::result::Result<T, AnthropicError>;
