use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// The primary error type for the Chat Completions client.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// HTTP transport error (network, DNS, TLS, etc.)
    #[error("HTTP error: {0}")]
    Http(#[from] HttpError),

    /// API returned an error response
    #[error("API error: {0}")]
    Api(#[from] ApiError),

    /// Failed to deserialize the response
    #[error("Deserialize error: {0}")]
    Deserialize(#[from] DeserializeError),

    /// Stream-specific errors
    #[error("Stream error: {0}")]
    Stream(#[from] StreamError),
}

/// HTTP transport error wrapping reqwest errors.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct HttpError {
    pub message: String,
    pub source: Option<reqwest::Error>,
}

impl From<reqwest::Error> for HttpError {
    fn from(err: reqwest::Error) -> Self {
        Self {
            message: err.to_string(),
            source: Some(err),
        }
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(err: reqwest::Error) -> Self {
        ClientError::Http(HttpError::from(err))
    }
}

/// API error returned by the server.
#[derive(Debug, thiserror::Error)]
#[error("{error}")]
pub struct ApiError {
    /// HTTP status code
    pub status: u16,
    /// The error object from the API
    pub error: ApiErrorBody,
    /// Raw response body for debugging
    pub raw_body: Option<Arc<str>>,
}

impl ApiError {
    /// Returns true if this is a rate limit error (429).
    pub fn is_rate_limit(&self) -> bool {
        self.status == 429
    }

    /// Returns true if this is a server error (5xx).
    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status)
    }

    /// Returns true if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        self.is_rate_limit() || self.is_server_error()
    }
}

/// The error body returned by the OpenAI API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub message: Arc<str>,
    #[serde(rename = "type")]
    pub error_type: Option<Arc<str>>,
    pub param: Option<Arc<str>>,
    pub code: Option<Arc<str>>,
}

impl std::fmt::Display for ApiErrorBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(ref code) = self.code {
            write!(f, " (code: {})", code)?;
        }
        Ok(())
    }
}

/// Deserialization error.
#[derive(Debug, thiserror::Error)]
#[error("Failed to deserialize response: {message}")]
pub struct DeserializeError {
    pub message: String,
    pub source: Option<serde_json::Error>,
    pub raw_body: Option<Arc<str>>,
}

impl From<serde_json::Error> for DeserializeError {
    fn from(err: serde_json::Error) -> Self {
        Self {
            message: err.to_string(),
            source: Some(err),
            raw_body: None,
        }
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(err: serde_json::Error) -> Self {
        ClientError::Deserialize(DeserializeError::from(err))
    }
}

/// Stream-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    /// Invalid SSE line format
    #[error("Invalid SSE line: {0}")]
    InvalidSseLine(Arc<str>),

    /// Error parsing SSE data as JSON
    #[error("Failed to parse SSE data: {0}")]
    ParseError(String),

    /// API error embedded in stream
    #[error("API error in stream: {0}")]
    ApiError(#[from] ApiError),

    /// Stream was closed unexpectedly
    #[error("Stream closed unexpectedly")]
    UnexpectedClose,
}
