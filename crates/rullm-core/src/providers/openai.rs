use crate::config::ProviderConfig;
use crate::error::LlmError;
use crate::middleware::EnhancedHttpClient;
use crate::types::{
    ChatMessage, ChatProvider, ChatRequest, ChatResponse, ChatRole, ChatStreamEvent, LlmProvider,
    StreamConfig, StreamResult, TokenUsage,
};
use crate::utils::sse::sse_lines;
use futures::StreamExt;

/// OpenAI provider implementation
#[derive(Clone)]
pub struct OpenAIProvider {
    config: crate::config::OpenAIConfig,
    client: EnhancedHttpClient,
}

impl OpenAIProvider {
    pub fn new(config: crate::config::OpenAIConfig) -> Result<Self, LlmError> {
        config.validate()?;

        let client = EnhancedHttpClient::new(&config)?;

        Ok(Self { config, client })
    }

    /// Convert our ChatRequest to OpenAI's API format
    fn to_openai_request(&self, request: &ChatRequest, model: &str) -> serde_json::Value {
        let mut openai_request = serde_json::json!({
            "model": model,
            "messages": request.messages.iter().map(|msg| serde_json::json!({
                "role": msg.role,
                "content": msg.content
            })).collect::<Vec<_>>()
        });

        if let Some(temp) = request.temperature {
            openai_request["temperature"] =
                serde_json::Value::Number(serde_json::Number::from_f64(temp as f64).unwrap());
        }

        if let Some(max_tokens) = request.max_tokens {
            openai_request["max_tokens"] =
                serde_json::Value::Number(serde_json::Number::from(max_tokens));
        }

        if let Some(top_p) = request.top_p {
            openai_request["top_p"] =
                serde_json::Value::Number(serde_json::Number::from_f64(top_p as f64).unwrap());
        }

        // if let Some(freq_penalty) = request.frequency_penalty {
        //     openai_request["frequency_penalty"] = serde_json::Value::Number(
        //         serde_json::Number::from_f64(freq_penalty as f64).unwrap(),
        //     );
        // }

        // if let Some(pres_penalty) = request.presence_penalty {
        //     openai_request["presence_penalty"] = serde_json::Value::Number(
        //         serde_json::Number::from_f64(pres_penalty as f64).unwrap(),
        //     );
        // }

        // if let Some(stop) = &request.stop {
        //     openai_request["stop"] = serde_json::Value::Array(
        //         stop.iter()
        //             .map(|s| serde_json::Value::String(s.clone()))
        //             .collect(),
        //     );
        // }

        if let Some(stream) = request.stream {
            openai_request["stream"] = serde_json::Value::Bool(stream);
        }

        openai_request
    }

    /// Parse OpenAI's response format into our ChatResponse
    fn parse_openai_response(&self, response: serde_json::Value) -> Result<ChatResponse, LlmError> {
        let choices = response["choices"].as_array().ok_or_else(|| {
            LlmError::serialization(
                "Missing 'choices' in OpenAI response",
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid response format",
                )),
            )
        })?;

        let first_choice = choices.first().ok_or_else(|| {
            LlmError::serialization(
                "No choices in OpenAI response",
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Empty choices array",
                )),
            )
        })?;

        let message = &first_choice["message"];
        let content = message["content"].as_str().ok_or_else(|| {
            LlmError::serialization(
                "Missing content in OpenAI response",
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Missing content field",
                )),
            )
        })?;

        let role = message["role"].as_str().ok_or_else(|| {
            LlmError::serialization(
                "Missing role in OpenAI response",
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Missing role field",
                )),
            )
        })?;

        let parsed_role = match role {
            "assistant" => ChatRole::Assistant,
            "user" => ChatRole::User,
            "system" => ChatRole::System,
            "tool" => ChatRole::Tool,
            _ => {
                return Err(LlmError::serialization(
                    format!("Unknown role: {role}"),
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid role",
                    )),
                ));
            }
        };

        let usage = &response["usage"];
        let token_usage = TokenUsage {
            prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: usage["total_tokens"].as_u64().unwrap_or(0) as u32,
        };

        let model = response["model"].as_str().unwrap_or("unknown").to_string();

        let finish_reason = first_choice["finish_reason"]
            .as_str()
            .map(|s| s.to_string());

        Ok(ChatResponse {
            message: ChatMessage {
                role: parsed_role,
                content: content.to_string(),
            },
            model,
            usage: token_usage,
            finish_reason,
            provider_metadata: None,
        })
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAIProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["openai", "gpt"]
    }

    fn env_key(&self) -> &'static str {
        "OPENAI_API_KEY"
    }

    fn default_base_url(&self) -> Option<&'static str> {
        Some("https://api.openai.com/v1")
    }

    async fn available_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/models", self.config.base_url());

        let resp = self
            .client
            .get_with_retry(&url, &self.config.headers())
            .await?;

        if !resp.status().is_success() {
            return Err(LlmError::api(
                "openai",
                "Failed to fetch available models",
                Some(resp.status().to_string()),
                None,
            ));
        }

        let json: serde_json::Value = resp
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

        if models.is_empty() {
            return Err(LlmError::api(
                "openai",
                "No models found in response",
                None,
                None,
            ));
        }

        Ok(models)
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        let url = format!("{}/models", self.config.base_url());

        let response = self
            .client
            .get_with_retry(&url, &self.config.headers())
            .await?;

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

#[async_trait::async_trait]
impl ChatProvider for OpenAIProvider {
    async fn chat_completion(
        &self,
        request: ChatRequest,
        model: &str,
    ) -> Result<ChatResponse, LlmError> {
        let url = format!("{}/chat/completions", self.config.base_url());
        let body = self.to_openai_request(&request, model);

        let response = self
            .client
            .post_with_retry(&url, &self.config.headers(), body)
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

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LlmError::serialization("Failed to parse JSON response", Box::new(e)))?;

        self.parse_openai_response(response_json)
    }

    async fn chat_completion_stream(
        &self,
        request: ChatRequest,
        model: &str,
        _config: Option<StreamConfig>,
    ) -> StreamResult<ChatStreamEvent> {
        let url = format!("{}/chat/completions", self.config.base_url());

        // Create streaming request with stream: true
        let mut streaming_request = request.clone();
        streaming_request.stream = Some(true);
        let body = self.to_openai_request(&streaming_request, model);

        // Make the streaming HTTP request using reqwest Client directly
        let client = reqwest::Client::new();
        let headers = self.config.headers();

        // Convert HashMap to HeaderMap
        let mut header_map = reqwest::header::HeaderMap::new();
        for (key, value) in headers {
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

        let response_future = client.post(&url).headers(header_map).json(&body).send();

        Box::pin(async_stream::stream! {
            // Handle the initial request
            let response = match response_future.await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status().to_string();
                        let error_text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        yield Err(LlmError::api(
                            "openai",
                            format!("API Error: {status} - {error_text}"),
                            Some(status),
                            None,
                        ));
                        return;
                    }
                    resp
                }
                Err(e) => {
                    yield Err(LlmError::network(format!("Request failed: {e}")));
                    return;
                }
            };

            // Get the byte stream and parse SSE events
            let byte_stream = response.bytes_stream();
            let mut sse_stream = sse_lines(byte_stream);

            while let Some(event_result) = sse_stream.next().await {
                match event_result {
                    Ok(data) => {
                        // Parse the JSON chunk
                        match serde_json::from_str::<serde_json::Value>(&data) {
                            Ok(chunk) => {
                                // Extract content from choices[0].delta.content
                                if let Some(choices) = chunk["choices"].as_array() {
                                    if let Some(first_choice) = choices.first() {
                                        if let Some(delta) = first_choice.get("delta") {
                                            if let Some(content) = delta["content"].as_str() {
                                                yield Ok(ChatStreamEvent::Token(content.to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                yield Err(LlmError::serialization(
                                    format!("Failed to parse chunk JSON: {e}"),
                                    Box::new(e),
                                ));
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                        return;
                    }
                }
            }

            // Emit Done event when streaming completes
            yield Ok(ChatStreamEvent::Done);
        })
    }

    async fn estimate_tokens(&self, text: &str, _model: &str) -> Result<u32, LlmError> {
        // Simple estimation: approximately 4 characters per token for English text
        // This is a rough approximation - in production you'd want to use tiktoken or similar
        Ok((text.len() as f32 / 4.0).ceil() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_helpers::fake_sse_response;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_openai_stream_parsing() {
        // Create fake OpenAI-style SSE events
        let events = vec![
            r#"{"choices":[{"delta":{"content":"Hello"}}]}"#,
            r#"{"choices":[{"delta":{"content":" "}}]}"#,
            r#"{"choices":[{"delta":{"content":"world"}}]}"#,
            r#"{"choices":[{"delta":{"content":"!"}}]}"#,
        ];

        // Create fake SSE stream
        let fake_stream = fake_sse_response(&events, None);

        // Parse using our sse_lines function
        let mut sse_stream = sse_lines(fake_stream);
        let mut tokens = Vec::new();

        // Process all events like the real implementation does
        while let Some(event_result) = sse_stream.next().await {
            match event_result {
                Ok(data) => {
                    // Parse the JSON chunk
                    match serde_json::from_str::<serde_json::Value>(&data) {
                        Ok(chunk) => {
                            // Extract content from choices[0].delta.content
                            if let Some(choices) = chunk["choices"].as_array() {
                                if let Some(first_choice) = choices.first() {
                                    if let Some(delta) = first_choice.get("delta") {
                                        if let Some(content) = delta["content"].as_str() {
                                            tokens.push(content.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => panic!("Failed to parse chunk JSON: {e}"),
                    }
                }
                Err(e) => panic!("SSE parsing error: {e}"),
            }
        }

        // Verify we got the expected tokens
        assert_eq!(tokens, vec!["Hello", " ", "world", "!"]);

        // Verify concatenated content
        let full_content: String = tokens.join("");
        assert_eq!(full_content, "Hello world!");
    }

    #[tokio::test]
    async fn test_openai_stream_with_empty_deltas() {
        // Test with some empty delta content (which OpenAI sometimes sends)
        let events = vec![
            r#"{"choices":[{"delta":{"content":"Start"}}]}"#,
            r#"{"choices":[{"delta":{}}]}"#, // Empty delta
            r#"{"choices":[{"delta":{"content":"End"}}]}"#,
        ];

        let fake_stream = fake_sse_response(&events, None);
        let mut sse_stream = sse_lines(fake_stream);
        let mut tokens = Vec::new();

        while let Some(event_result) = sse_stream.next().await {
            match event_result {
                Ok(data) => match serde_json::from_str::<serde_json::Value>(&data) {
                    Ok(chunk) => {
                        if let Some(choices) = chunk["choices"].as_array() {
                            if let Some(first_choice) = choices.first() {
                                if let Some(delta) = first_choice.get("delta") {
                                    if let Some(content) = delta["content"].as_str() {
                                        tokens.push(content.to_string());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => panic!("Failed to parse chunk JSON: {e}"),
                },
                Err(e) => panic!("SSE parsing error: {e}"),
            }
        }

        // Should only get the non-empty content
        assert_eq!(tokens, vec!["Start", "End"]);
    }

    #[tokio::test]
    async fn test_openai_stream_malformed_json() {
        // Test handling of malformed JSON - simulate real implementation behavior
        let events = vec![
            r#"{"choices":[{"delta":{"content":"Good"}}]}"#,
            r#"{"invalid json"#, // Malformed JSON
            r#"{"choices":[{"delta":{"content":"After"}}]}"#,
        ];

        let fake_stream = fake_sse_response(&events, None);
        let mut sse_stream = sse_lines(fake_stream);
        let mut tokens = Vec::new();
        let mut had_parse_error = false;

        while let Some(event_result) = sse_stream.next().await {
            match event_result {
                Ok(data) => {
                    match serde_json::from_str::<serde_json::Value>(&data) {
                        Ok(chunk) => {
                            if let Some(choices) = chunk["choices"].as_array() {
                                if let Some(first_choice) = choices.first() {
                                    if let Some(delta) = first_choice.get("delta") {
                                        if let Some(content) = delta["content"].as_str() {
                                            tokens.push(content.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // In real implementation this would yield an error and return
                            // For test, mark that we encountered the error and break like real impl
                            had_parse_error = true;
                            break; // Stop processing like the real implementation would
                        }
                    }
                }
                Err(e) => panic!("SSE parsing error: {e}"),
            }
        }

        // Should have gotten first token and encountered a parse error
        assert_eq!(tokens, vec!["Good"]);
        assert!(
            had_parse_error,
            "Should have encountered a JSON parse error"
        );
    }
}
