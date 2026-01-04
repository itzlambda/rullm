use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::error::ClientError;

/// Configuration for the Chat Completions client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL for the API (e.g., "https://api.openai.com/v1")
    pub base_url: Arc<str>,
    /// Authentication configuration
    pub auth: AuthConfig,
    /// Default headers to include in all requests
    pub default_headers: HeaderMap,
    /// Request timeout
    pub timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: Arc::from("https://api.openai.com/v1"),
            auth: AuthConfig::None,
            default_headers: HeaderMap::new(),
            timeout: Duration::from_secs(60),
        }
    }
}

impl ClientConfig {
    /// Create a new builder for ClientConfig.
    pub fn builder() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
    }
}

/// Builder for creating a [`ClientConfig`].
#[derive(Debug, Default)]
pub struct ClientConfigBuilder {
    base_url: Option<Arc<str>>,
    auth: Option<AuthConfig>,
    default_headers: HeaderMap,
    timeout: Option<Duration>,
}

impl ClientConfigBuilder {
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base URL for the API.
    pub fn base_url(mut self, url: impl Into<Arc<str>>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the authentication configuration.
    pub fn auth(mut self, auth: AuthConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Set a bearer token for authentication.
    pub fn bearer_token(mut self, token: impl Into<Arc<str>>) -> Self {
        self.auth = Some(AuthConfig::BearerToken(token.into()));
        self
    }

    /// Add a default header that will be included in all requests.
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.default_headers.insert(name, value);
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the [`ClientConfig`].
    pub fn build(self) -> Result<ClientConfig, ClientError> {
        Ok(ClientConfig {
            base_url: self
                .base_url
                .unwrap_or_else(|| Arc::from("https://api.openai.com/v1")),
            auth: self.auth.unwrap_or(AuthConfig::None),
            default_headers: self.default_headers,
            timeout: self.timeout.unwrap_or(Duration::from_secs(60)),
        })
    }
}

/// Authentication configuration for the client.
#[derive(Debug, Clone)]
pub enum AuthConfig {
    /// No authentication
    None,
    /// Bearer token authentication (Authorization: Bearer <token>)
    BearerToken(Arc<str>),
    /// Custom header authentication
    Header {
        name: HeaderName,
        value: HeaderValue,
    },
    /// Query parameter authentication
    QueryParam { name: Arc<str>, value: Arc<str> },
}

impl AuthConfig {
    /// Create a bearer token auth config.
    pub fn bearer(token: impl Into<Arc<str>>) -> Self {
        Self::BearerToken(token.into())
    }

    /// Create a custom header auth config.
    pub fn header(name: HeaderName, value: HeaderValue) -> Self {
        Self::Header { name, value }
    }

    /// Create a query parameter auth config.
    pub fn query_param(name: impl Into<Arc<str>>, value: impl Into<Arc<str>>) -> Self {
        Self::QueryParam {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// Rate limit information from response headers.
#[derive(Debug, Clone, Default)]
pub struct RateLimitInfo {
    /// Maximum requests allowed in the window
    pub limit_requests: Option<u32>,
    /// Maximum tokens allowed in the window
    pub limit_tokens: Option<u32>,
    /// Remaining requests in the current window
    pub remaining_requests: Option<u32>,
    /// Remaining tokens in the current window
    pub remaining_tokens: Option<u32>,
    /// Time until the request limit resets (in seconds)
    pub reset_requests_secs: Option<f64>,
    /// Time until the token limit resets (in seconds)
    pub reset_tokens_secs: Option<f64>,
}

impl RateLimitInfo {
    /// Parse rate limit info from response headers.
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            limit_requests: parse_header_u32(headers, "x-ratelimit-limit-requests"),
            limit_tokens: parse_header_u32(headers, "x-ratelimit-limit-tokens"),
            remaining_requests: parse_header_u32(headers, "x-ratelimit-remaining-requests"),
            remaining_tokens: parse_header_u32(headers, "x-ratelimit-remaining-tokens"),
            reset_requests_secs: parse_reset_header(headers, "x-ratelimit-reset-requests"),
            reset_tokens_secs: parse_reset_header(headers, "x-ratelimit-reset-tokens"),
        }
    }
}

fn parse_header_u32(headers: &HeaderMap, name: &str) -> Option<u32> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
}

fn parse_reset_header(headers: &HeaderMap, name: &str) -> Option<f64> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_reset_duration)
}

/// Parse a reset duration string like "1s", "1m30s", "500ms", etc.
fn parse_reset_duration(s: &str) -> Option<f64> {
    let s = s.trim();

    // Try parsing as a simple float (seconds)
    if let Ok(secs) = s.parse::<f64>() {
        return Some(secs);
    }

    // Parse duration format like "1m30s", "500ms", "2s"
    let mut total_secs = 0.0;
    let mut current_num = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' {
            current_num.push(c);
        } else {
            if current_num.is_empty() {
                continue;
            }
            let num: f64 = current_num.parse().ok()?;
            current_num.clear();

            match c {
                'h' => total_secs += num * 3600.0,
                'm' if s.contains("ms") => {} // handled separately
                'm' => total_secs += num * 60.0,
                's' => total_secs += num,
                _ => {}
            }
        }
    }

    // Handle milliseconds specially
    if s.ends_with("ms") {
        if let Some(ms_str) = s.strip_suffix("ms") {
            if let Ok(ms) = ms_str.trim().parse::<f64>() {
                return Some(ms / 1000.0);
            }
        }
    }

    if total_secs > 0.0 {
        Some(total_secs)
    } else {
        None
    }
}

/// Response metadata extracted from HTTP headers.
#[derive(Debug, Clone, Default)]
pub struct ResponseMeta {
    /// Request ID from the x-request-id header
    pub request_id: Option<Arc<str>>,
    /// Rate limit information
    pub ratelimit: Option<RateLimitInfo>,
    /// Processing time in milliseconds
    pub latency_ms: Option<u64>,
}

impl ResponseMeta {
    /// Parse metadata from response headers.
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            request_id: headers
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .map(Arc::from),
            ratelimit: Some(RateLimitInfo::from_headers(headers)),
            latency_ms: headers
                .get("openai-processing-ms")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok()),
        }
    }
}

/// Wrapper for API responses that includes metadata.
#[derive(Debug, Clone)]
pub struct ApiResponse<T> {
    /// The response data
    pub data: T,
    /// Response metadata
    pub meta: ResponseMeta,
    /// Raw JSON body for debugging
    pub raw_json: Option<Arc<str>>,
}

impl<T> ApiResponse<T> {
    /// Create a new API response.
    pub fn new(data: T, meta: ResponseMeta) -> Self {
        Self {
            data,
            meta,
            raw_json: None,
        }
    }

    /// Create an API response with raw JSON preserved.
    pub fn with_raw_json(data: T, meta: ResponseMeta, raw_json: Arc<str>) -> Self {
        Self {
            data,
            meta,
            raw_json: Some(raw_json),
        }
    }

    /// Map the data to a different type.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> ApiResponse<U> {
        ApiResponse {
            data: f(self.data),
            meta: self.meta,
            raw_json: self.raw_json,
        }
    }
}
