use crate::error::LlmError;
use crate::providers::openai_compatible::{identities, OpenAICompatibleProvider};
use crate::types::{
    ChatCompletion, ChatRequest, ChatResponse, ChatStreamEvent, LlmProvider, StreamConfig,
    StreamResult,
};

/// OpenAI provider implementation (wrapper around OpenAICompatibleProvider)
#[derive(Clone)]
pub struct OpenAIProvider {
    inner: OpenAICompatibleProvider,
}

impl OpenAIProvider {
    pub fn new(config: crate::config::OpenAIConfig) -> Result<Self, LlmError> {
        let inner = OpenAICompatibleProvider::new(config, identities::OPENAI)?;
        Ok(Self { inner })
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAIProvider {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn aliases(&self) -> &'static [&'static str] {
        self.inner.aliases()
    }

    fn env_key(&self) -> &'static str {
        self.inner.env_key()
    }

    fn default_base_url(&self) -> Option<&'static str> {
        self.inner.default_base_url()
    }

    async fn available_models(&self) -> Result<Vec<String>, LlmError> {
        self.inner.available_models().await
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        self.inner.health_check().await
    }
}

#[async_trait::async_trait]
impl ChatCompletion for OpenAIProvider {
    async fn chat_completion(
        &self,
        request: ChatRequest,
        model: &str,
    ) -> Result<ChatResponse, LlmError> {
        self.inner.chat_completion(request, model).await
    }

    async fn chat_completion_stream(
        &self,
        request: ChatRequest,
        model: &str,
        config: Option<StreamConfig>,
    ) -> StreamResult<ChatStreamEvent> {
        self.inner.chat_completion_stream(request, model, config).await
    }

    async fn estimate_tokens(&self, text: &str, model: &str) -> Result<u32, LlmError> {
        self.inner.estimate_tokens(text, model).await
    }
}

#[cfg(test)]
mod tests {
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
        let mut sse_stream = crate::utils::sse::sse_lines(fake_stream);
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
        let mut sse_stream = crate::utils::sse::sse_lines(fake_stream);
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
        let mut sse_stream = crate::utils::sse::sse_lines(fake_stream);
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
