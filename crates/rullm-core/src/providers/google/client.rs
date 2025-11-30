use super::types::*;
use crate::config::{GoogleAiConfig, ProviderConfig};
use crate::error::LlmError;
use crate::utils::sse::sse_lines;
use futures::Stream;
use futures::StreamExt;
use reqwest::Client;
use std::pin::Pin;

/// Google Gemini client with full API support
#[derive(Clone)]
pub struct GoogleClient {
    config: GoogleAiConfig,
    client: Client,
    base_url: String,
}

impl GoogleClient {
    /// Create a new Google client
    pub fn new(config: GoogleAiConfig) -> Result<Self, LlmError> {
        config.validate()?;
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string());

        Ok(Self {
            config,
            client: Client::new(),
            base_url,
        })
    }

    /// Create client from environment variables
    pub fn from_env() -> Result<Self, LlmError> {
        let config = crate::config::ConfigBuilder::google_ai_from_env()?;
        Self::new(config)
    }

    /// Generate content
    pub async fn generate_content(
        &self,
        model: &str,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse, LlmError> {
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url,
            model,
            self.config.api_key()
        );

        let mut req = self.client.post(&url);
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
                "google",
                format!("API Error: {status} - {error_text}"),
                Some(status),
                None,
            ));
        }

        let response_data: GenerateContentResponse = response.json().await.map_err(|e| {
            LlmError::serialization("Failed to parse GenerateContentResponse", Box::new(e))
        })?;

        Ok(response_data)
    }

    /// Stream generate content
    pub async fn stream_generate_content(
        &self,
        model: &str,
        request: GenerateContentRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<GenerateContentResponse, LlmError>> + Send>>,
        LlmError,
    > {
        let url = format!(
            "{}/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url,
            model,
            self.config.api_key()
        );

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
                "google",
                format!("API Error: {status} - {error_text}"),
                Some(status),
                None,
            ));
        }

        let byte_stream = response.bytes_stream();
        let sse_stream = sse_lines(byte_stream);

        Ok(Box::pin(sse_stream.map(|event_result| {
            event_result.and_then(|data| {
                serde_json::from_str::<GenerateContentResponse>(&data).map_err(|e| {
                    LlmError::serialization(
                        format!("Failed to parse GenerateContentResponse: {}", e),
                        Box::new(e),
                    )
                })
            })
        })))
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/models?key={}", self.base_url, self.config.api_key());

        let mut req = self.client.get(&url);
        for (key, value) in self.config.headers() {
            req = req.header(key, value);
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            return Err(LlmError::api(
                "google",
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
            .get("models")
            .and_then(|m| m.as_array())
            .ok_or_else(|| {
                LlmError::serialization(
                    "Invalid models response format",
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Missing models array",
                    )),
                )
            })?;

        let models: Vec<String> = models_array
            .iter()
            .filter_map(|m| {
                m.get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.split('/').last().unwrap_or(s).to_string())
            })
            .collect();

        Ok(models)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<(), LlmError> {
        let url = format!("{}/models?key={}", self.base_url, self.config.api_key());

        let mut req = self.client.get(&url);
        for (key, value) in self.config.headers() {
            req = req.header(key, value);
        }

        let response = req.send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(LlmError::api(
                "google",
                "Health check failed",
                Some(response.status().to_string()),
                None,
            ))
        }
    }
}
