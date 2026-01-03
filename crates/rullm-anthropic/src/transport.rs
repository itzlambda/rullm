//! HTTP transport abstraction for the Anthropic client
//!
//! Provides a trait-based transport layer that can be mocked for testing.

use crate::config::{ClientConfig, RequestOptions};
use crate::error::{AnthropicError, ApiErrorResponse, Result};
use bytes::Bytes;
use futures::Stream;
use reqwest::header::HeaderMap;
use reqwest::{Client, Response, StatusCode};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// HTTP transport for making requests to the Anthropic API
pub struct HttpTransport {
    client: Client,
    config: ClientConfig,
}

impl HttpTransport {
    /// Create a new HTTP transport with the given configuration
    pub fn new(config: ClientConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(AnthropicError::Transport)?;

        Ok(Self { client, config })
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Get the default headers
    pub fn default_headers(&self) -> &HeaderMap {
        &self.config.default_headers
    }

    /// Make a POST request and return the response
    pub async fn post<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
        options: &RequestOptions,
    ) -> Result<Response> {
        let url = format!("{}{}", self.config.base_url, path);

        let mut request = self.client.post(&url);

        // Apply default headers
        request = request.headers(self.config.default_headers.clone());

        // Apply extra headers from options
        for (key, value) in &options.extra_headers {
            request = request.header(key, value);
        }

        // Apply query parameters
        for (key, value) in &options.extra_query {
            request = request.query(&[(key.as_ref(), value.as_ref())]);
        }

        // Apply timeout override
        if let Some(timeout) = options.timeout {
            request = request.timeout(timeout);
        }

        // Send request with body
        let response = request.json(body).send().await?;

        Ok(response)
    }

    /// Make a POST request and parse the JSON response
    pub async fn post_json<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
        options: &RequestOptions,
    ) -> Result<(R, ResponseMeta)> {
        let response = self.post(path, body, options).await?;
        let meta = ResponseMeta::from_response(&response);

        if !response.status().is_success() {
            return Err(parse_error_response(response, meta.request_id.clone()).await);
        }

        let data: R = response
            .json()
            .await
            .map_err(|e| AnthropicError::serialization("Failed to parse response", Box::new(e)))?;

        Ok((data, meta))
    }

    /// Make a streaming POST request
    pub async fn post_stream<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
        options: &RequestOptions,
    ) -> Result<(
        Pin<Box<dyn Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send>>,
        ResponseMeta,
    )> {
        let response = self.post(path, body, options).await?;
        let meta = ResponseMeta::from_response(&response);

        if !response.status().is_success() {
            return Err(parse_error_response(response, meta.request_id.clone()).await);
        }

        Ok((Box::pin(response.bytes_stream()), meta))
    }

    /// Get the configuration
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Get the maximum retries
    pub fn max_retries(&self) -> u32 {
        self.config.max_retries
    }

    /// Get the default timeout
    pub fn timeout(&self) -> Duration {
        self.config.timeout
    }
}

/// Metadata from an API response
#[derive(Debug, Clone, Default)]
pub struct ResponseMeta {
    /// Request ID from the x-request-id header
    pub request_id: Option<Arc<str>>,
}

impl ResponseMeta {
    /// Extract metadata from a response
    fn from_response(response: &Response) -> Self {
        let request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(Arc::from);

        Self { request_id }
    }
}

/// Parse an error response from the API
async fn parse_error_response(response: Response, request_id: Option<Arc<str>>) -> AnthropicError {
    let status = response.status();

    // Try to parse the error body
    let error_result: std::result::Result<ApiErrorResponse, _> = response.json().await;

    match error_result {
        Ok(error_response) => AnthropicError::api(status, request_id, error_response.error),
        Err(_) => {
            // Couldn't parse error, create a generic one
            AnthropicError::api(
                status,
                request_id,
                crate::error::ErrorObject {
                    error_type: Arc::from(status_to_error_type(status)),
                    message: Arc::from(format!("HTTP {}", status)),
                    param: None,
                },
            )
        }
    }
}

/// Map status code to error type string
fn status_to_error_type(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "invalid_request_error",
        StatusCode::UNAUTHORIZED => "authentication_error",
        StatusCode::FORBIDDEN => "permission_error",
        StatusCode::NOT_FOUND => "not_found_error",
        StatusCode::TOO_MANY_REQUESTS => "rate_limit_error",
        StatusCode::INTERNAL_SERVER_ERROR => "api_error",
        StatusCode::SERVICE_UNAVAILABLE => "overloaded_error",
        _ if status.as_u16() == 529 => "overloaded_error",
        _ => "api_error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ClientBuilder;

    #[test]
    fn test_response_meta_default() {
        let meta = ResponseMeta::default();
        assert!(meta.request_id.is_none());
    }

    #[test]
    fn test_transport_creation() {
        let config = ClientBuilder::new()
            .api_key("test-key")
            .build()
            .expect("should build config");

        let transport = HttpTransport::new(config);
        assert!(transport.is_ok());
    }

    #[test]
    fn test_status_to_error_type() {
        assert_eq!(
            status_to_error_type(StatusCode::BAD_REQUEST),
            "invalid_request_error"
        );
        assert_eq!(
            status_to_error_type(StatusCode::UNAUTHORIZED),
            "authentication_error"
        );
        assert_eq!(
            status_to_error_type(StatusCode::TOO_MANY_REQUESTS),
            "rate_limit_error"
        );
    }
}
