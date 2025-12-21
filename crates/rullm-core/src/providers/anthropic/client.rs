use super::config::AnthropicConfig;
use super::types::*;
use crate::config::ProviderConfig;
use crate::error::LlmError;
use crate::utils::sse::sse_lines;
use futures::Stream;
use futures::StreamExt;
use reqwest::Client;
use std::pin::Pin;

/// Anthropic client with full Messages API support
#[derive(Clone)]
pub struct AnthropicClient {
    config: AnthropicConfig,
    client: Client,
    base_url: String,
}

impl AnthropicClient {
    /// Create a new Anthropic client
    pub fn new(config: AnthropicConfig) -> Result<Self, LlmError> {
        config.validate()?;
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com".to_string());

        Ok(Self {
            config,
            client: Client::new(),
            base_url,
        })
    }

    /// Create client from environment variables
    pub fn from_env() -> Result<Self, LlmError> {
        let config = crate::config::ConfigBuilder::anthropic_from_env()?;
        Self::new(config)
    }

    /// Send a messages request
    pub async fn messages(&self, request: MessagesRequest) -> Result<MessagesResponse, LlmError> {
        let url = format!("{}/v1/messages", self.base_url);

        let mut req = self.client.post(&url);

        // Add headers from config
        for (key, value) in self.config.headers() {
            req = req.header(key, value);
        }

        let response = req.json(&request).send().await?;

        if !response.status().is_success() {
            let status = response.status().to_string();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(LlmError::api(
                "anthropic",
                format!("API Error: {status} - {error_text}"),
                Some(status),
                None,
            ));
        }

        let response_data: MessagesResponse = response.json().await.map_err(|e| {
            LlmError::serialization("Failed to parse MessagesResponse", Box::new(e))
        })?;

        Ok(response_data)
    }

    /// Send a streaming messages request
    pub async fn messages_stream(
        &self,
        mut request: MessagesRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LlmError>> + Send>>, LlmError> {
        // Force streaming
        request.stream = Some(true);

        let url = format!("{}/v1/messages", self.base_url);

        // Build headers
        let mut header_map = reqwest::header::HeaderMap::new();
        for (key, value) in self.config.headers() {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(&value),
            ) {
                header_map.insert(name, val);
            }
        }

        let response = self
            .client
            .post(&url)
            .headers(header_map)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().to_string();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(LlmError::api(
                "anthropic",
                format!("API Error: {status} - {error_text}"),
                Some(status),
                None,
            ));
        }

        let byte_stream = response.bytes_stream();
        let sse_stream = sse_lines(byte_stream);

        Ok(Box::pin(sse_stream.map(|event_result| {
            event_result.and_then(|data| {
                serde_json::from_str::<StreamEvent>(&data).map_err(|e| {
                    LlmError::serialization(
                        format!("Failed to parse StreamEvent: {}", e),
                        Box::new(e),
                    )
                })
            })
        })))
    }

    /// Count tokens (requires a separate API call)
    pub async fn count_tokens(
        &self,
        model: &str,
        messages: Vec<Message>,
        system: Option<SystemPrompt>,
    ) -> Result<u32, LlmError> {
        let url = format!("{}/v1/messages/count_tokens", self.base_url);

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "system": system,
        });

        let mut req = self.client.post(&url);
        for (key, value) in self.config.headers() {
            req = req.header(key, value);
        }

        let response = req.json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().to_string();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(LlmError::api(
                "anthropic",
                format!("API Error: {status} - {error_text}"),
                Some(status),
                None,
            ));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            LlmError::serialization("Failed to parse count_tokens response", Box::new(e))
        })?;

        let tokens = json["input_tokens"].as_u64().ok_or_else(|| {
            LlmError::serialization(
                "Missing input_tokens in response",
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid response format",
                )),
            )
        })? as u32;

        Ok(tokens)
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/v1/models", self.base_url);

        let mut req = self.client.get(&url);
        for (key, value) in self.config.headers() {
            req = req.header(key, value);
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            return Err(LlmError::api(
                "anthropic",
                "Failed to fetch available models",
                Some(response.status().to_string()),
                None,
            ));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LlmError::serialization("Failed to parse models response", Box::new(e)))?;

        let models_array = json
            .get("data")
            .and_then(|d| d.as_array())
            .or_else(|| json.get("models").and_then(|m| m.as_array()))
            .ok_or_else(|| {
                LlmError::serialization(
                    "Invalid models response format",
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Missing data array",
                    )),
                )
            })?;

        let models: Vec<String> = models_array
            .iter()
            .filter_map(|m| {
                m.get("id")
                    .and_then(|id| id.as_str())
                    .or_else(|| m.get("name").and_then(|name| name.as_str()))
                    .or_else(|| m.get("model").and_then(|model| model.as_str()))
                    .map(|s| s.to_string())
            })
            .collect();

        if models.is_empty() {
            return Err(LlmError::api(
                "anthropic",
                "No models found in response",
                None,
                None,
            ));
        }

        Ok(models)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<(), LlmError> {
        // Anthropic doesn't have a dedicated health endpoint
        // We can do a minimal request to check connectivity
        let url = format!("{}/v1/messages", self.base_url);

        let minimal_request =
            MessagesRequest::new("claude-3-haiku-20240307", vec![Message::user("hi")], 1);

        let mut req = self.client.post(&url);
        for (key, value) in self.config.headers() {
            req = req.header(key, value);
        }

        let response = req.json(&minimal_request).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(LlmError::api(
                "anthropic",
                "Health check failed",
                Some(response.status().to_string()),
                None,
            ))
        }
    }
}
