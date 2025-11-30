use super::types::*;
use crate::config::{OpenAIConfig, ProviderConfig};
use crate::error::LlmError;
use crate::utils::sse::sse_lines;
use futures::Stream;
use futures::StreamExt;
use reqwest::Client;
use std::pin::Pin;

/// OpenAI client with full API support
#[derive(Clone)]
pub struct OpenAIClient {
    config: OpenAIConfig,
    client: Client,
    base_url: String,
}

impl OpenAIClient {
    /// Create a new OpenAI client
    pub fn new(config: OpenAIConfig) -> Result<Self, LlmError> {
        config.validate()?;
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        Ok(Self {
            config,
            client: Client::new(),
            base_url,
        })
    }

    /// Create client from environment variables
    pub fn from_env() -> Result<Self, LlmError> {
        let config = crate::config::ConfigBuilder::openai_from_env()?;
        Self::new(config)
    }

    /// Send a chat completion request
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);

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
                "openai",
                format!("API Error: {status} - {error_text}"),
                Some(status),
                None,
            ));
        }

        let response_data: ChatCompletionResponse = response.json().await.map_err(|e| {
            LlmError::serialization("Failed to parse ChatCompletionResponse", Box::new(e))
        })?;

        Ok(response_data)
    }

    /// Send a streaming chat completion request
    pub async fn chat_completion_stream(
        &self,
        mut request: ChatCompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, LlmError>> + Send>>, LlmError>
    {
        // Force streaming
        request.stream = Some(true);

        let url = format!("{}/chat/completions", self.base_url);

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
        header_map.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("text/event-stream"),
        );

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
                "openai",
                format!("API Error: {status} - {error_text}"),
                Some(status),
                None,
            ));
        }

        let byte_stream = response.bytes_stream();
        let sse_stream = sse_lines(byte_stream);

        Ok(Box::pin(sse_stream.map(|event_result| {
            event_result.and_then(|data| {
                // OpenAI sends "[DONE]" to signal end of stream
                if data.trim() == "[DONE]" {
                    // We could return a special marker, but for now just skip it
                    // The stream will end naturally
                    return Err(LlmError::model("Stream complete"));
                }

                serde_json::from_str::<ChatCompletionChunk>(&data).map_err(|e| {
                    LlmError::serialization(
                        format!("Failed to parse ChatCompletionChunk: {}", e),
                        Box::new(e),
                    )
                })
            })
        })))
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/models", self.base_url);

        let mut req = self.client.get(&url);
        for (key, value) in self.config.headers() {
            req = req.header(key, value);
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            return Err(LlmError::api(
                "openai",
                "Failed to fetch available models",
                Some(response.status().to_string()),
                None,
            ));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LlmError::serialization("Failed to parse models response", Box::new(e)))?;

        let models_array = json.get("data").and_then(|d| d.as_array()).ok_or_else(|| {
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
                    .map(|s| s.to_string())
            })
            .collect();

        Ok(models)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<(), LlmError> {
        let url = format!("{}/models", self.base_url);

        let mut req = self.client.get(&url);
        for (key, value) in self.config.headers() {
            req = req.header(key, value);
        }

        let response = req.send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(LlmError::api(
                "openai",
                "Health check failed",
                Some(response.status().to_string()),
                None,
            ))
        }
    }
}
