//! Configuration and client builder for the Anthropic client

use crate::error::{AnthropicError, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::env;
use std::sync::Arc;
use std::time::Duration;

/// Default base URL for the Anthropic API
pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

/// Default API version header value
pub const DEFAULT_API_VERSION: &str = "2023-06-01";

/// Default timeout for non-streaming requests (10 minutes as per SDK)
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);

/// Environment variable for API key
pub const ENV_API_KEY: &str = "ANTHROPIC_API_KEY";

/// Environment variable for OAuth token
pub const ENV_AUTH_TOKEN: &str = "ANTHROPIC_AUTH_TOKEN";

/// Environment variable for base URL override
pub const ENV_BASE_URL: &str = "ANTHROPIC_BASE_URL";

/// Authentication mode for the client
#[derive(Debug, Clone)]
pub enum AuthMode {
    /// API key authentication (x-api-key header)
    ApiKey(Arc<str>),
    /// OAuth token authentication (Bearer token)
    OAuth(Arc<str>),
}

impl AuthMode {
    /// Apply authentication headers to a HeaderMap
    pub fn apply_to_headers(&self, headers: &mut HeaderMap) {
        match self {
            AuthMode::ApiKey(key) => {
                if let Ok(value) = HeaderValue::from_str(key) {
                    headers.insert("x-api-key", value);
                }
            }
            AuthMode::OAuth(token) => {
                if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                    headers.insert("authorization", value);
                }
            }
        }
    }
}

/// Per-request options to override client defaults
#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    /// Override timeout for this request
    pub timeout: Option<Duration>,

    /// Extra headers to merge with default headers
    pub extra_headers: HeaderMap,

    /// Extra query parameters
    pub extra_query: Vec<(Arc<str>, Arc<str>)>,

    /// Extra body fields to merge (for advanced use)
    pub extra_body: serde_json::Map<String, serde_json::Value>,

    /// Allow long non-streaming requests (bypasses timeout policy)
    pub allow_long_non_streaming: bool,
}

impl RequestOptions {
    /// Create a new RequestOptions with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            timeout: Some(timeout),
            ..Default::default()
        }
    }

    /// Add an extra header
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.extra_headers.insert(name, value);
        self
    }

    /// Add an extra query parameter
    pub fn query(mut self, key: impl Into<Arc<str>>, value: impl Into<Arc<str>>) -> Self {
        self.extra_query.push((key.into(), value.into()));
        self
    }

    /// Allow long non-streaming requests
    pub fn allow_long_request(mut self) -> Self {
        self.allow_long_non_streaming = true;
        self
    }
}

/// Builder for constructing a Client
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    /// API key for authentication
    api_key: Option<Arc<str>>,

    /// OAuth token for authentication
    auth_token: Option<Arc<str>>,

    /// Base URL for API requests
    base_url: Arc<str>,

    /// Default timeout for requests
    timeout: Duration,

    /// Maximum number of retries for failed requests
    max_retries: u32,

    /// Beta features to enable
    beta: Vec<Arc<str>>,

    /// Default headers to include in all requests
    default_headers: HeaderMap,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientBuilder {
    /// Create a new ClientBuilder with default values
    pub fn new() -> Self {
        Self {
            api_key: None,
            auth_token: None,
            base_url: Arc::from(DEFAULT_BASE_URL),
            timeout: DEFAULT_TIMEOUT,
            max_retries: 2,
            beta: Vec::new(),
            default_headers: HeaderMap::new(),
        }
    }

    /// Create a ClientBuilder from environment variables
    ///
    /// Reads from:
    /// - `ANTHROPIC_API_KEY` - API key
    /// - `ANTHROPIC_AUTH_TOKEN` - OAuth token (takes precedence over API key)
    /// - `ANTHROPIC_BASE_URL` - Base URL override
    pub fn from_env() -> Self {
        let mut builder = Self::new();

        if let Ok(key) = env::var(ENV_API_KEY) {
            builder.api_key = Some(Arc::from(key));
        }

        if let Ok(token) = env::var(ENV_AUTH_TOKEN) {
            builder.auth_token = Some(Arc::from(token));
        }

        if let Ok(url) = env::var(ENV_BASE_URL) {
            builder.base_url = Arc::from(url);
        }

        builder
    }

    /// Set the API key
    pub fn api_key(mut self, key: impl Into<Arc<str>>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the OAuth token
    pub fn auth_token(mut self, token: impl Into<Arc<str>>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Set the base URL
    pub fn base_url(mut self, url: impl Into<Arc<str>>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the default timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Add a beta feature
    pub fn beta(mut self, feature: impl Into<Arc<str>>) -> Self {
        self.beta.push(feature.into());
        self
    }

    /// Add multiple beta features
    pub fn betas(mut self, features: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        self.beta.extend(features.into_iter().map(Into::into));
        self
    }

    /// Add a default header
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.default_headers.insert(name, value);
        self
    }

    /// Validate and build the configuration
    pub fn build(self) -> Result<ClientConfig> {
        // Determine auth mode - OAuth takes precedence
        let auth = if let Some(token) = self.auth_token {
            AuthMode::OAuth(token)
        } else if let Some(key) = self.api_key {
            AuthMode::ApiKey(key)
        } else {
            return Err(AnthropicError::configuration(
                "No API key or OAuth token provided. Set ANTHROPIC_API_KEY or ANTHROPIC_AUTH_TOKEN environment variable.",
            ));
        };

        // Build default headers
        let mut headers = self.default_headers;

        // Add required headers
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(DEFAULT_API_VERSION),
        );
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        // Add beta header if features are specified
        if !self.beta.is_empty() {
            let beta_value = self.beta.join(",");
            if let Ok(value) = HeaderValue::from_str(&beta_value) {
                headers.insert("anthropic-beta", value);
            }
        }

        // Apply auth headers
        auth.apply_to_headers(&mut headers);

        Ok(ClientConfig {
            auth,
            base_url: self.base_url,
            timeout: self.timeout,
            max_retries: self.max_retries,
            default_headers: headers,
        })
    }
}

/// Validated client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Authentication mode
    pub(crate) auth: AuthMode,

    /// Base URL for API requests
    pub(crate) base_url: Arc<str>,

    /// Default timeout for requests
    pub(crate) timeout: Duration,

    /// Maximum number of retries
    pub(crate) max_retries: u32,

    /// Default headers for all requests
    pub(crate) default_headers: HeaderMap,
}

impl ClientConfig {
    /// Get the authentication mode
    pub fn auth(&self) -> &AuthMode {
        &self.auth
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the default timeout
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Get the maximum retries
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Get a reference to the default headers
    pub fn headers(&self) -> &HeaderMap {
        &self.default_headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_with_api_key() {
        let config = ClientBuilder::new()
            .api_key("test-key")
            .build()
            .expect("should build");

        assert!(matches!(config.auth, AuthMode::ApiKey(_)));
        assert_eq!(config.base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn test_builder_with_oauth() {
        let config = ClientBuilder::new()
            .auth_token("test-token")
            .build()
            .expect("should build");

        assert!(matches!(config.auth, AuthMode::OAuth(_)));
    }

    #[test]
    fn test_oauth_takes_precedence() {
        let config = ClientBuilder::new()
            .api_key("api-key")
            .auth_token("oauth-token")
            .build()
            .expect("should build");

        assert!(matches!(config.auth, AuthMode::OAuth(_)));
    }

    #[test]
    fn test_builder_no_auth_fails() {
        let result = ClientBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_with_beta() {
        let config = ClientBuilder::new()
            .api_key("test-key")
            .beta("feature-1")
            .beta("feature-2")
            .build()
            .expect("should build");

        let beta_header = config.default_headers.get("anthropic-beta");
        assert!(beta_header.is_some());
        assert_eq!(
            beta_header.unwrap().to_str().unwrap(),
            "feature-1,feature-2"
        );
    }
}
