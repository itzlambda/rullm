use crate::config::ProviderConfig;
use crate::error::LlmError;
use crate::middleware::EnhancedHttpClient;
use crate::types::{
    ChatMessage, ChatProvider, ChatRequest, ChatResponse, ChatRole, ChatStreamEvent, LlmProvider,
    StreamConfig, StreamResult, TokenUsage,
};
use futures::StreamExt;
use std::collections::HashMap;

/// Google AI Provider implementation
pub struct GoogleProvider {
    config: crate::config::GoogleAiConfig,
    client: EnhancedHttpClient,
}

impl GoogleProvider {
    /// Create a new Google AI provider instance
    pub fn new(config: crate::config::GoogleAiConfig) -> Result<Self, LlmError> {
        // Validate configuration first
        config.validate()?;

        let client = EnhancedHttpClient::new(&config)?;

        Ok(Self { config, client })
    }

    /// Convert internal ChatRequest to Google AI API format
    fn to_google_ai_request(&self, request: &ChatRequest) -> serde_json::Value {
        let mut contents = Vec::new();
        let mut system_instruction = None;

        // Process messages - separate system messages from conversation
        for message in &request.messages {
            match message.role {
                ChatRole::System => {
                    // Google AI uses systemInstruction for system messages
                    system_instruction = Some(serde_json::json!({
                        "parts": [{"text": message.content}]
                    }));
                }
                ChatRole::User => {
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{"text": message.content}]
                    }));
                }
                ChatRole::Assistant => {
                    contents.push(serde_json::json!({
                        "role": "model",
                        "parts": [{"text": message.content}]
                    }));
                }
                ChatRole::Tool => {
                    todo!("Handle Tool role for Google provider")
                }
            }
        }

        let mut request_body = serde_json::json!({
            "contents": contents
        });

        // Add system instruction if present
        if let Some(system_instr) = system_instruction {
            request_body["systemInstruction"] = system_instr;
        }

        // Add generation configuration
        let mut generation_config = serde_json::Map::new();

        if let Some(temperature) = request.temperature {
            generation_config.insert(
                "temperature".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(temperature as f64).unwrap(),
                ),
            );
        }

        if let Some(max_tokens) = request.max_tokens {
            generation_config.insert(
                "maxOutputTokens".to_string(),
                serde_json::Value::Number(serde_json::Number::from(max_tokens)),
            );
        }

        if let Some(top_p) = request.top_p {
            generation_config.insert(
                "topP".to_string(),
                serde_json::Value::Number(serde_json::Number::from_f64(top_p as f64).unwrap()),
            );
        }

        // if let Some(stop_sequences) = &request.stop {
        //     if !stop_sequences.is_empty() {
        //         generation_config.insert(
        //             "stopSequences".to_string(),
        //             serde_json::Value::Array(
        //                 stop_sequences
        //                     .iter()
        //                     .map(|s| serde_json::Value::String(s.clone()))
        //                     .collect(),
        //             ),
        //         );
        //     }
        // }

        if !generation_config.is_empty() {
            request_body["generationConfig"] = serde_json::Value::Object(generation_config);
        }

        // Add any extra parameters
        if let Some(extra_params) = &request.extra_params {
            for (key, value) in extra_params {
                request_body[key] = value.clone();
            }
        }

        request_body
    }

    /// Parse Google AI API response to internal ChatResponse format
    fn parse_google_ai_response(
        &self,
        response: serde_json::Value,
        model: String,
    ) -> Result<ChatResponse, LlmError> {
        // Extract candidates array
        let candidates = response["candidates"]
            .as_array()
            .ok_or_else(|| LlmError::api("google", "No candidates in response", None, None))?;

        if candidates.is_empty() {
            return Err(LlmError::api(
                "google",
                "Empty candidates array",
                None,
                None,
            ));
        }

        let candidate = &candidates[0];

        // Extract content from the first candidate
        let content_parts = candidate["content"]["parts"]
            .as_array()
            .ok_or_else(|| LlmError::api("google", "No content parts in candidate", None, None))?;

        if content_parts.is_empty() {
            return Err(LlmError::api("google", "Empty content parts", None, None));
        }

        // Combine all text parts
        let content = content_parts
            .iter()
            .filter_map(|part| part["text"].as_str())
            .collect::<Vec<_>>()
            .join("");

        // Extract finish reason
        let finish_reason = candidate["finishReason"].as_str().map(|s| s.to_string());

        // Extract usage information
        let usage_metadata = response
            .get("usageMetadata")
            .unwrap_or(&serde_json::Value::Null);
        let prompt_tokens = usage_metadata["promptTokenCount"].as_u64().unwrap_or(0) as u32;
        let completion_tokens = usage_metadata["candidatesTokenCount"].as_u64().unwrap_or(0) as u32;
        let total_tokens = usage_metadata["totalTokenCount"]
            .as_u64()
            .unwrap_or((prompt_tokens + completion_tokens) as u64)
            as u32;

        let usage = TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        };

        // Create provider metadata
        let mut provider_metadata = HashMap::new();
        provider_metadata.insert(
            "google_ai_version".to_string(),
            serde_json::Value::String("v1beta".to_string()),
        );

        if let Some(safety_ratings) = candidate.get("safetyRatings") {
            provider_metadata.insert("safety_ratings".to_string(), safety_ratings.clone());
        }

        Ok(ChatResponse {
            message: ChatMessage {
                role: ChatRole::Assistant,
                content,
            },
            model,
            usage,
            finish_reason,
            provider_metadata: Some(provider_metadata),
        })
    }
}

#[async_trait::async_trait]
impl LlmProvider for GoogleProvider {
    fn provider_name(&self) -> &'static str {
        "google"
    }

    async fn available_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!(
            "{}/models?key={}",
            self.config.base_url(),
            self.config.api_key()
        );

        let resp = self
            .client
            .get_with_retry(&url, &self.config.headers())
            .await?;

        if !resp.status().is_success() {
            return Err(LlmError::api(
                "google",
                "Failed to fetch available models",
                Some(resp.status().to_string()),
                None,
            ));
        }

        let json: serde_json::Value = resp
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
                    .or_else(|| m.get("id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.split('/').next_back().unwrap_or(s).to_string())
            })
            .collect();

        if models.is_empty() {
            return Err(LlmError::api(
                "google",
                "No models found in response",
                None,
                None,
            ));
        }

        Ok(models)
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        // For Google AI, we'll make a simple request to list models to verify connectivity
        let url = format!(
            "{}/models?key={}",
            self.config.base_url(),
            self.config.api_key()
        );

        let response = self
            .client
            .get_with_retry(&url, &self.config.headers())
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else if status == reqwest::StatusCode::UNAUTHORIZED {
            Err(LlmError::authentication("Invalid Google AI API key"))
        } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Err(LlmError::rate_limit(
                "Rate limit exceeded",
                Some(std::time::Duration::from_secs(60)),
            ))
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(LlmError::api(
                "google",
                format!("Health check failed: {error_text}"),
                Some(status.as_str().to_string()),
                None,
            ))
        }
    }
}

#[async_trait::async_trait]
impl ChatProvider for GoogleProvider {
    async fn chat_completion(
        &self,
        request: ChatRequest,
        model: &str,
    ) -> Result<ChatResponse, LlmError> {
        let google_request = self.to_google_ai_request(&request);

        // Build the URL with API key as query parameter
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.config.base_url(),
            model,
            self.config.api_key()
        );

        let response = self
            .client
            .post_with_retry(&url, &self.config.headers(), google_request)
            .await?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| LlmError::network(format!("Failed to read response: {e}")))?;

        if !status.is_success() {
            return match status {
                reqwest::StatusCode::UNAUTHORIZED => {
                    Err(LlmError::authentication("Invalid Google AI API key"))
                }
                reqwest::StatusCode::TOO_MANY_REQUESTS => Err(LlmError::rate_limit(
                    "Rate limit exceeded",
                    Some(std::time::Duration::from_secs(60)),
                )),
                reqwest::StatusCode::BAD_REQUEST => Err(LlmError::validation(format!(
                    "Bad request: {response_text}"
                ))),
                _ => {
                    // Try to parse error from response
                    if let Ok(error_json) =
                        serde_json::from_str::<serde_json::Value>(&response_text)
                    {
                        let error_message = error_json
                            .get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(|m| m.as_str())
                            .unwrap_or(&response_text);
                        Err(LlmError::api(
                            "google",
                            error_message,
                            Some(status.as_str().to_string()),
                            None,
                        ))
                    } else {
                        Err(LlmError::api(
                            "google",
                            &response_text,
                            Some(status.as_str().to_string()),
                            None,
                        ))
                    }
                }
            };
        }

        let response_json: serde_json::Value =
            serde_json::from_str(&response_text).map_err(|e| {
                LlmError::serialization(format!("Failed to parse JSON response: {e}"), e)
            })?;

        self.parse_google_ai_response(response_json, model.to_string())
    }

    async fn chat_completion_stream(
        &self,
        request: ChatRequest,
        model: &str,
        _config: Option<StreamConfig>,
    ) -> StreamResult<ChatStreamEvent> {
        let google_request = self.to_google_ai_request(&request);

        // Build the URL with API key for streaming endpoint
        let url = format!(
            "{}/models/{}:streamGenerateContent?key={}",
            self.config.base_url(),
            model,
            self.config.api_key()
        );

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
            .json(&google_request)
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
                                LlmError::authentication("Invalid Google AI API key")
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
                                LlmError::api("google", &error_text, Some(status.as_str().to_string()), None)
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

            // Get the byte stream and parse newline-delimited JSON chunks
            let mut byte_stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        // Add new bytes to buffer
                        match std::str::from_utf8(&bytes) {
                            Ok(text) => {
                                buffer.push_str(text);

                                // Process complete lines (Google uses newline-delimited JSON)
                                while let Some(newline_pos) = buffer.find('\n') {
                                    let line = buffer[..newline_pos].trim().to_string();
                                    buffer.drain(..newline_pos + 1);

                                    // Skip empty lines
                                    if line.is_empty() {
                                        continue;
                                    }

                                    // Parse the JSON chunk
                                    match serde_json::from_str::<serde_json::Value>(&line) {
                                        Ok(chunk) => {
                                            // Extract content from candidates[0].content.parts[].text
                                            if let Some(candidates) = chunk["candidates"].as_array() {
                                                if let Some(first_candidate) = candidates.first() {
                                                    if let Some(content) = first_candidate.get("content") {
                                                        if let Some(parts) = content["parts"].as_array() {
                                                            for part in parts {
                                                                if let Some(text) = part["text"].as_str() {
                                                                    yield Ok(ChatStreamEvent::Token(text.to_string()));
                                                                }
                                                            }
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
                            }
                            Err(e) => {
                                yield Err(LlmError::serialization(
                                    "Invalid UTF-8 in response stream",
                                    Box::new(e),
                                ));
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(LlmError::network(format!("Stream error: {e}")));
                        return;
                    }
                }
            }

            // Process any remaining content in buffer
            if !buffer.trim().is_empty() {
                match serde_json::from_str::<serde_json::Value>(buffer.trim()) {
                    Ok(chunk) => {
                        if let Some(candidates) = chunk["candidates"].as_array() {
                            if let Some(first_candidate) = candidates.first() {
                                if let Some(content) = first_candidate.get("content") {
                                    if let Some(parts) = content["parts"].as_array() {
                                        for part in parts {
                                            if let Some(text) = part["text"].as_str() {
                                                yield Ok(ChatStreamEvent::Token(text.to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // Ignore parse errors for trailing content that might not be complete JSON
                    }
                }
            }

            // Emit Done event when streaming completes
            yield Ok(ChatStreamEvent::Done);
        })
    }

    async fn estimate_tokens(&self, text: &str, _model: &str) -> Result<u32, LlmError> {
        // Simple approximation: ~4 characters per token for Google AI models
        Ok((text.len() as f32 / 4.0).ceil() as u32)
    }
}

#[cfg(test)]
mod tests {

    use futures::{StreamExt, stream};

    // Helper to create a fake newline-delimited JSON stream for testing
    fn fake_json_stream(
        chunks: &[&str],
    ) -> impl futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> {
        let data: Vec<Result<bytes::Bytes, reqwest::Error>> = chunks
            .iter()
            .map(|chunk| Ok(bytes::Bytes::from(format!("{chunk}\n"))))
            .collect();
        stream::iter(data)
    }

    #[tokio::test]
    async fn test_google_stream_parsing_single_part() {
        // Test with Google-style JSON chunks containing single text parts
        let chunks = vec![
            r#"{"candidates":[{"content":{"parts":[{"text":"Hello"}]}}]}"#,
            r#"{"candidates":[{"content":{"parts":[{"text":" world"}]}}]}"#,
            r#"{"candidates":[{"content":{"parts":[{"text":"!"}]}}]}"#,
        ];

        let fake_stream = fake_json_stream(&chunks);
        let mut buffer = String::new();
        let mut tokens = Vec::new();

        // Simulate the parsing logic from our implementation
        let mut byte_stream = fake_stream;
        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    match std::str::from_utf8(&bytes) {
                        Ok(text) => {
                            buffer.push_str(text);

                            // Process complete lines
                            while let Some(newline_pos) = buffer.find('\n') {
                                let line = buffer[..newline_pos].trim().to_string();
                                buffer.drain(..newline_pos + 1);

                                if line.is_empty() {
                                    continue;
                                }

                                // Parse the JSON chunk
                                match serde_json::from_str::<serde_json::Value>(&line) {
                                    Ok(chunk) => {
                                        if let Some(candidates) = chunk["candidates"].as_array() {
                                            if let Some(first_candidate) = candidates.first() {
                                                if let Some(content) =
                                                    first_candidate.get("content")
                                                {
                                                    if let Some(parts) = content["parts"].as_array()
                                                    {
                                                        for part in parts {
                                                            if let Some(text) =
                                                                part["text"].as_str()
                                                            {
                                                                tokens.push(text.to_string());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => panic!("Failed to parse chunk JSON: {e}"),
                                }
                            }
                        }
                        Err(e) => panic!("UTF-8 error: {e}"),
                    }
                }
                Err(e) => panic!("Stream error: {e}"),
            }
        }

        // Verify we got the expected tokens
        assert_eq!(tokens, vec!["Hello", " world", "!"]);

        // Verify concatenated content
        let full_content: String = tokens.join("");
        assert_eq!(full_content, "Hello world!");
    }

    #[tokio::test]
    async fn test_google_stream_parsing_multi_parts() {
        // Test with chunks containing multiple parts in one response
        let chunks = vec![
            r#"{"candidates":[{"content":{"parts":[{"text":"First "},{"text":"part"}]}}]}"#,
            r#"{"candidates":[{"content":{"parts":[{"text":" and "},{"text":"second "},{"text":"part"}]}}]}"#,
        ];

        let fake_stream = fake_json_stream(&chunks);
        let mut buffer = String::new();
        let mut tokens = Vec::new();

        let mut byte_stream = fake_stream;
        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(bytes) => match std::str::from_utf8(&bytes) {
                    Ok(text) => {
                        buffer.push_str(text);

                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer[..newline_pos].trim().to_string();
                            buffer.drain(..newline_pos + 1);

                            if line.is_empty() {
                                continue;
                            }

                            match serde_json::from_str::<serde_json::Value>(&line) {
                                Ok(chunk) => {
                                    if let Some(candidates) = chunk["candidates"].as_array() {
                                        if let Some(first_candidate) = candidates.first() {
                                            if let Some(content) = first_candidate.get("content") {
                                                if let Some(parts) = content["parts"].as_array() {
                                                    for part in parts {
                                                        if let Some(text) = part["text"].as_str() {
                                                            tokens.push(text.to_string());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => panic!("Failed to parse chunk JSON: {e}"),
                            }
                        }
                    }
                    Err(e) => panic!("UTF-8 error: {e}"),
                },
                Err(e) => panic!("Stream error: {e}"),
            }
        }

        // Should get all individual parts
        assert_eq!(tokens, vec!["First ", "part", " and ", "second ", "part"]);
    }

    #[tokio::test]
    async fn test_google_stream_empty_content() {
        // Test with chunks that have no content or empty parts
        let chunks = vec![
            r#"{"candidates":[{"content":{"parts":[{"text":"Valid"}]}}]}"#,
            r#"{"candidates":[{"content":{"parts":[]}}]}"#, // Empty parts
            r#"{"candidates":[{"content":{"parts":[{"text":"After"}]}}]}"#,
        ];

        let fake_stream = fake_json_stream(&chunks);
        let mut buffer = String::new();
        let mut tokens = Vec::new();

        let mut byte_stream = fake_stream;
        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(bytes) => match std::str::from_utf8(&bytes) {
                    Ok(text) => {
                        buffer.push_str(text);

                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer[..newline_pos].trim().to_string();
                            buffer.drain(..newline_pos + 1);

                            if line.is_empty() {
                                continue;
                            }

                            match serde_json::from_str::<serde_json::Value>(&line) {
                                Ok(chunk) => {
                                    if let Some(candidates) = chunk["candidates"].as_array() {
                                        if let Some(first_candidate) = candidates.first() {
                                            if let Some(content) = first_candidate.get("content") {
                                                if let Some(parts) = content["parts"].as_array() {
                                                    for part in parts {
                                                        if let Some(text) = part["text"].as_str() {
                                                            tokens.push(text.to_string());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => panic!("Failed to parse chunk JSON: {e}"),
                            }
                        }
                    }
                    Err(e) => panic!("UTF-8 error: {e}"),
                },
                Err(e) => panic!("Stream error: {e}"),
            }
        }

        // Should only get the valid content, skip empty parts
        assert_eq!(tokens, vec!["Valid", "After"]);
    }

    #[tokio::test]
    async fn test_google_stream_malformed_json() {
        // Test handling of malformed JSON
        let chunks = vec![
            r#"{"candidates":[{"content":{"parts":[{"text":"Good"}]}}]}"#,
            r#"{"invalid json"#, // Malformed JSON
            r#"{"candidates":[{"content":{"parts":[{"text":"After"}]}}]}"#,
        ];

        let fake_stream = fake_json_stream(&chunks);
        let mut buffer = String::new();
        let mut tokens = Vec::new();
        let mut had_parse_error = false;

        let mut byte_stream = fake_stream;
        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    match std::str::from_utf8(&bytes) {
                        Ok(text) => {
                            buffer.push_str(text);

                            while let Some(newline_pos) = buffer.find('\n') {
                                let line = buffer[..newline_pos].trim().to_string();
                                buffer.drain(..newline_pos + 1);

                                if line.is_empty() {
                                    continue;
                                }

                                match serde_json::from_str::<serde_json::Value>(&line) {
                                    Ok(chunk) => {
                                        if let Some(candidates) = chunk["candidates"].as_array() {
                                            if let Some(first_candidate) = candidates.first() {
                                                if let Some(content) =
                                                    first_candidate.get("content")
                                                {
                                                    if let Some(parts) = content["parts"].as_array()
                                                    {
                                                        for part in parts {
                                                            if let Some(text) =
                                                                part["text"].as_str()
                                                            {
                                                                tokens.push(text.to_string());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        // In real implementation this would yield an error and return
                                        // For test, mark that we encountered the error and break
                                        had_parse_error = true;
                                        break;
                                    }
                                }
                            }

                            if had_parse_error {
                                break;
                            }
                        }
                        Err(e) => panic!("UTF-8 error: {e}"),
                    }
                }
                Err(e) => panic!("Stream error: {e}"),
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
