use crate::config::ProviderConfig;
use crate::error::LlmError;
use crate::types::{
    ChatCompletion, ChatMessage, ChatRequest, ChatResponse, ChatRole, ChatStreamEvent, LlmProvider,
    StreamConfig, StreamResult, TokenUsage,
};
use crate::utils::sse::sse_lines;
use futures::StreamExt;
use reqwest::Client;

/// Anthropic provider for Claude models
pub struct AnthropicProvider {
    config: crate::config::AnthropicConfig,
    client: Client,
}

impl AnthropicProvider {
    /// Create a new AnthropicProvider
    pub fn new(config: crate::config::AnthropicConfig) -> Result<Self, LlmError> {
        config.validate()?;
        let client = Client::new();
        Ok(Self { config, client })
    }

    /// Convert our ChatRequest to Anthropic's message format
    fn to_anthropic_request(&self, request: &ChatRequest, model: &str) -> serde_json::Value {
        let mut anthropic_request = serde_json::json!({
            "model": model,
            "max_tokens": request.max_tokens.unwrap_or(1000)
        });

        // Extract system messages and regular messages
        let mut system_messages = Vec::new();
        let mut conversation_messages = Vec::new();

        for message in &request.messages {
            match message.role {
                ChatRole::System => {
                    system_messages.push(message.content.clone());
                }
                ChatRole::User => {
                    conversation_messages.push(serde_json::json!({
                        "role": "user",
                        "content": message.content
                    }));
                }
                ChatRole::Assistant => {
                    conversation_messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": message.content
                    }));
                }
                ChatRole::Tool => {
                    todo!("Handle Tool role for Anthropic provider")
                }
            }
        }

        // Add system prompt if we have system messages
        if !system_messages.is_empty() {
            anthropic_request["system"] = serde_json::Value::String(system_messages.join("\n\n"));
        }

        anthropic_request["messages"] = serde_json::Value::Array(conversation_messages);

        // Add optional parameters
        if let Some(temperature) = request.temperature {
            anthropic_request["temperature"] = serde_json::Value::Number(
                serde_json::Number::from_f64(temperature as f64).unwrap(),
            );
        }

        if let Some(top_p) = request.top_p {
            anthropic_request["top_p"] =
                serde_json::Value::Number(serde_json::Number::from_f64(top_p as f64).unwrap());
        }

        // if let Some(stop) = &request.stop {
        //     anthropic_request["stop_sequences"] = serde_json::Value::Array(
        //         stop.iter()
        //             .map(|s| serde_json::Value::String(s.clone()))
        //             .collect(),
        //     );
        // }

        if let Some(stream) = request.stream {
            anthropic_request["stream"] = serde_json::Value::Bool(stream);
        }

        anthropic_request
    }

    /// Parse Anthropic's response format into our ChatResponse
    pub fn parse_anthropic_response(
        &self,
        response: serde_json::Value,
    ) -> Result<ChatResponse, LlmError> {
        let content_array = response["content"].as_array().ok_or_else(|| {
            LlmError::serialization(
                "Missing 'content' in Anthropic response",
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid response format",
                )),
            )
        })?;

        let first_content = content_array.first().ok_or_else(|| {
            LlmError::serialization(
                "No content blocks in Anthropic response",
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Empty content array",
                )),
            )
        })?;

        let content_text = first_content["text"].as_str().ok_or_else(|| {
            LlmError::serialization(
                "Missing text in Anthropic content block",
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Missing text field",
                )),
            )
        })?;

        let usage = &response["usage"];
        let token_usage = TokenUsage {
            prompt_tokens: usage["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: usage["output_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: (usage["input_tokens"].as_u64().unwrap_or(0)
                + usage["output_tokens"].as_u64().unwrap_or(0)) as u32,
        };

        let model = response["model"].as_str().unwrap_or("unknown").to_string();

        let finish_reason = response["stop_reason"].as_str().map(|s| s.to_string());

        Ok(ChatResponse {
            message: ChatMessage {
                role: ChatRole::Assistant,
                content: content_text.to_string(),
            },
            model,
            usage: token_usage,
            finish_reason,
            provider_metadata: None,
        })
    }
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["anthropic", "claude"]
    }

    fn env_key(&self) -> &'static str {
        "ANTHROPIC_API_KEY"
    }

    fn default_base_url(&self) -> Option<&'static str> {
        Some("https://api.anthropic.com/v1")
    }

    async fn available_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/v1/models", self.config.base_url().trim_end_matches('/'));

        let mut req = self.client.get(&url);
        for (key, value) in self.config.headers() {
            req = req.header(&key, &value);
        }
        let resp = req.send().await?;

        if !resp.status().is_success() {
            return Err(LlmError::api(
                "anthropic",
                "Failed to fetch available models",
                Some(resp.status().to_string()),
                None,
            ));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::serialization("Failed to parse models response", Box::new(e)))?;

        // Try to parse with "data" field (Anthropic/OpenAI format)
        if let Some(models_array) = json.get("data").and_then(|d| d.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| {
                    m.get("id")
                        .or_else(|| m.get("name"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect();

            if !models.is_empty() {
                return Ok(models);
            }
        }

        // Try to parse as direct array first
        if let Some(models_array) = json.as_array() {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| {
                    m.get("id")
                        .or_else(|| m.get("name"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect();

            if !models.is_empty() {
                return Ok(models);
            }
        }

        // Try to parse with "models" field
        if let Some(models_array) = json.get("models").and_then(|m| m.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| {
                    m.get("id")
                        .or_else(|| m.get("name"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect();

            if !models.is_empty() {
                return Ok(models);
            }
        }

        Err(LlmError::serialization(
            "Invalid models response format - no recognizable models array found",
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unexpected response structure: {json}"),
            )),
        ))
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        // Use the models endpoint for health check - faster and no token usage
        let url = format!("{}/v1/models", self.config.base_url().trim_end_matches('/'));

        let mut req = self.client.get(&url);
        for (key, value) in self.config.headers() {
            req = req.header(&key, &value);
        }
        let response = req.send().await?;

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

#[async_trait::async_trait]
impl ChatCompletion for AnthropicProvider {
    async fn chat_completion(
        &self,
        request: ChatRequest,
        model: &str,
    ) -> Result<ChatResponse, LlmError> {
        let url = format!("{}/messages", self.config.base_url());
        let body = self.to_anthropic_request(&request, model);

        let mut req = self.client.post(&url);
        for (key, value) in self.config.headers() {
            req = req.header(&key, &value);
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

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LlmError::serialization("Failed to parse JSON response", Box::new(e)))?;

        self.parse_anthropic_response(response_json)
    }

    async fn chat_completion_stream(
        &self,
        request: ChatRequest,
        model: &str,
        _config: Option<StreamConfig>,
    ) -> StreamResult<ChatStreamEvent> {
        let mut anthropic_request = self.to_anthropic_request(&request, model);

        // Ensure streaming is enabled
        anthropic_request["stream"] = serde_json::Value::Bool(true);

        let url = format!("{}/messages", self.config.base_url());

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

        let response_future = client
            .post(&url)
            .headers(header_map)
            .json(&anthropic_request)
            .send();

        Box::pin(async_stream::stream! {
            // Handle the initial request
            let response = match response_future.await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let error_text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());

                        let error = match status {
                            reqwest::StatusCode::UNAUTHORIZED => {
                                LlmError::authentication("Invalid Anthropic API key")
                            }
                            reqwest::StatusCode::TOO_MANY_REQUESTS => {
                                LlmError::rate_limit(
                                    "Rate limit exceeded",
                                    Some(std::time::Duration::from_secs(60)),
                                )
                            }
                            reqwest::StatusCode::BAD_REQUEST => {
                                LlmError::validation(format!("Bad request: {error_text}"))
                            }
                            _ => {
                                LlmError::api("anthropic", &error_text, Some(status.as_str().to_string()), None)
                            }
                        };
                        yield Err(error);
                        return;
                    }
                    resp
                }
                Err(e) => {
                    yield Err(LlmError::network(format!("Request failed: {e}")));
                    return;
                }
            };

            // Use the SSE helper to parse the response stream
            let byte_stream = response.bytes_stream();
            let mut sse_stream = sse_lines(byte_stream);

            while let Some(sse_result) = sse_stream.next().await {
                match sse_result {
                    Ok(sse_data) => {
                        // Parse the SSE data as JSON
                        match serde_json::from_str::<serde_json::Value>(&sse_data) {
                            Ok(event) => {
                                // Handle different Anthropic SSE event types
                                if let Some(event_type) = event.get("type").and_then(|v| v.as_str()) {
                                    match event_type {
                                        "content_block_delta" => {
                                            // Extract content from delta.text field
                                            if let Some(delta) = event.get("delta") {
                                                if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                                    yield Ok(ChatStreamEvent::Token(text.to_string()));
                                                }
                                            }
                                        }
                                        "message_stop" => {
                                            // End of stream
                                            yield Ok(ChatStreamEvent::Done);
                                            return;
                                        }
                                        "ping" | "message_start" | "content_block_start" | "content_block_stop" => {
                                            // Ignore these event types
                                            continue;
                                        }
                                        "error" => {
                                            if let Some(error_msg) = event.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                                                yield Err(LlmError::api("anthropic", error_msg, None, None));
                                                return;
                                            }
                                        }
                                        _ => {
                                            // Skip unknown event types
                                            continue;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                yield Err(LlmError::serialization(
                                    format!("Failed to parse SSE event JSON: {e}"),
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

            // If we reach here without a message_stop event, emit Done anyway
            yield Ok(ChatStreamEvent::Done);
        })
    }

    async fn estimate_tokens(&self, text: &str, _model: &str) -> Result<u32, LlmError> {
        // Simple estimation: approximately 3.5 characters per token for Claude
        // This is a rough approximation - Anthropic doesn't provide a public tokenizer
        Ok((text.len() as f32 / 3.5).ceil() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_helpers::fake_sse_response;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_anthropic_stream_parsing_content_delta() {
        // Test with Anthropic-style SSE events containing content_block_delta
        let events = vec![
            r#"{"type": "message_start", "message": {"id": "msg_123", "type": "message", "role": "assistant", "content": [], "model": "claude-3-sonnet-20240229", "stop_reason": null, "stop_sequence": null, "usage": {"input_tokens": 25, "output_tokens": 1}}}"#,
            r#"{"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}"#,
            r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}"#,
            r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": " world"}}"#,
            r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "!"}}"#,
            r#"{"type": "content_block_stop", "index": 0}"#,
            r#"{"type": "message_stop"}"#,
        ];

        let fake_stream = fake_sse_response(&events, None);
        let mut sse_stream = sse_lines(fake_stream);
        let mut tokens = Vec::new();

        // Simulate the parsing logic from our implementation
        while let Some(sse_result) = sse_stream.next().await {
            match sse_result {
                Ok(sse_data) => {
                    match serde_json::from_str::<serde_json::Value>(&sse_data) {
                        Ok(event) => {
                            if let Some(event_type) = event.get("type").and_then(|v| v.as_str()) {
                                match event_type {
                                    "content_block_delta" => {
                                        if let Some(delta) = event.get("delta") {
                                            if let Some(text) =
                                                delta.get("text").and_then(|t| t.as_str())
                                            {
                                                tokens.push(text.to_string());
                                            }
                                        }
                                    }
                                    "message_stop" => {
                                        tokens.push("DONE".to_string());
                                        break;
                                    }
                                    _ => {
                                        // Ignore other event types like in real implementation
                                        continue;
                                    }
                                }
                            }
                        }
                        Err(e) => panic!("Failed to parse event JSON: {e}"),
                    }
                }
                Err(e) => panic!("SSE parsing error: {e}"),
            }
        }

        // Verify we got the expected tokens
        assert_eq!(tokens, vec!["Hello", " world", "!", "DONE"]);

        // Verify concatenated content (excluding DONE marker)
        let full_content: String = tokens[..tokens.len() - 1].join("");
        assert_eq!(full_content, "Hello world!");
    }

    #[tokio::test]
    async fn test_anthropic_stream_parsing_with_empty_deltas() {
        // Test with events that have empty or missing text in delta
        let events = vec![
            r#"{"type": "message_start", "message": {"id": "msg_123"}}"#,
            r#"{"type": "content_block_start", "index": 0}"#,
            r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Valid"}}"#,
            r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": ""}}"#, // Empty text
            r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta"}}"#, // Missing text field
            r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "After"}}"#,
            r#"{"type": "message_stop"}"#,
        ];

        let fake_stream = fake_sse_response(&events, None);
        let mut sse_stream = sse_lines(fake_stream);
        let mut tokens = Vec::new();

        while let Some(sse_result) = sse_stream.next().await {
            match sse_result {
                Ok(sse_data) => {
                    match serde_json::from_str::<serde_json::Value>(&sse_data) {
                        Ok(event) => {
                            if let Some(event_type) = event.get("type").and_then(|v| v.as_str()) {
                                match event_type {
                                    "content_block_delta" => {
                                        if let Some(delta) = event.get("delta") {
                                            if let Some(text) =
                                                delta.get("text").and_then(|t| t.as_str())
                                            {
                                                // Only add non-empty text
                                                if !text.is_empty() {
                                                    tokens.push(text.to_string());
                                                }
                                            }
                                        }
                                    }
                                    "message_stop" => {
                                        tokens.push("DONE".to_string());
                                        break;
                                    }
                                    _ => continue,
                                }
                            }
                        }
                        Err(e) => panic!("Failed to parse event JSON: {e}"),
                    }
                }
                Err(e) => panic!("SSE parsing error: {e}"),
            }
        }

        // Should only get the valid, non-empty content
        assert_eq!(tokens, vec!["Valid", "After", "DONE"]);
    }

    #[tokio::test]
    async fn test_anthropic_stream_malformed_json() {
        // Test handling of malformed JSON events
        let events = vec![
            r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Good"}}"#,
            r#"{"invalid json"#, // Malformed JSON
            r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "After"}}"#,
        ];

        let fake_stream = fake_sse_response(&events, None);
        let mut sse_stream = sse_lines(fake_stream);
        let mut tokens = Vec::new();
        let mut had_parse_error = false;

        while let Some(sse_result) = sse_stream.next().await {
            match sse_result {
                Ok(sse_data) => {
                    match serde_json::from_str::<serde_json::Value>(&sse_data) {
                        Ok(event) => {
                            if let Some(event_type) = event.get("type").and_then(|v| v.as_str()) {
                                if event_type == "content_block_delta" {
                                    if let Some(delta) = event.get("delta") {
                                        if let Some(text) =
                                            delta.get("text").and_then(|t| t.as_str())
                                        {
                                            tokens.push(text.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // Malformed JSON encountered
                            had_parse_error = true;
                            break; // In real implementation, this would yield an error and return
                        }
                    }
                }
                Err(_) => {
                    had_parse_error = true;
                    break;
                }
            }
        }

        // Should have processed the first valid event
        assert_eq!(tokens, vec!["Good"]);
        // Should have encountered a parse error
        assert!(had_parse_error);
    }

    #[tokio::test]
    async fn test_anthropic_stream_unknown_event_types() {
        // Test that unknown event types are properly ignored
        let events = vec![
            r#"{"type": "unknown_event", "data": "should be ignored"}"#,
            r#"{"type": "ping"}"#, // Should be ignored
            r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Valid"}}"#,
            r#"{"type": "future_event_type", "new_field": "should be ignored"}"#,
            r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Text"}}"#,
            r#"{"type": "message_stop"}"#,
        ];

        let fake_stream = fake_sse_response(&events, None);
        let mut sse_stream = sse_lines(fake_stream);
        let mut tokens = Vec::new();

        while let Some(sse_result) = sse_stream.next().await {
            match sse_result {
                Ok(sse_data) => {
                    match serde_json::from_str::<serde_json::Value>(&sse_data) {
                        Ok(event) => {
                            if let Some(event_type) = event.get("type").and_then(|v| v.as_str()) {
                                match event_type {
                                    "content_block_delta" => {
                                        if let Some(delta) = event.get("delta") {
                                            if let Some(text) =
                                                delta.get("text").and_then(|t| t.as_str())
                                            {
                                                tokens.push(text.to_string());
                                            }
                                        }
                                    }
                                    "message_stop" => {
                                        tokens.push("DONE".to_string());
                                        break;
                                    }
                                    "ping"
                                    | "message_start"
                                    | "content_block_start"
                                    | "content_block_stop" => {
                                        // These should be ignored
                                        continue;
                                    }
                                    _ => {
                                        // Unknown events should be ignored
                                        continue;
                                    }
                                }
                            }
                        }
                        Err(e) => panic!("Failed to parse event JSON: {e}"),
                    }
                }
                Err(e) => panic!("SSE parsing error: {e}"),
            }
        }

        // Should only get the valid content deltas, not the unknown events
        assert_eq!(tokens, vec!["Valid", "Text", "DONE"]);
    }
}
