use crate::config::*;
use crate::error::LlmError;
use crate::middleware::{LlmServiceBuilder, MiddlewareConfig, RateLimit};
use crate::types::{
    ChatCompletion, ChatMessage, ChatRequest, ChatRequestBuilder, ChatResponse, ChatRole,
    LlmProvider, StreamConfig, TokenUsage,
};
use std::time::Duration;

// Mock provider for testing
#[derive(Clone)]
struct MockProvider {
    name: &'static str,
    should_fail: bool,
}

impl MockProvider {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            should_fail: false,
        }
    }

    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
}

#[async_trait::async_trait]
impl LlmProvider for MockProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn default_base_url(&self) -> Option<&'static str> {
        Some("")
    }

    fn env_key(&self) -> &'static str {
        ""
    }

    async fn available_models(&self) -> Result<Vec<String>, LlmError> {
        if self.should_fail {
            return Err(LlmError::network("Mock network error"));
        }
        Ok(vec!["model-1".to_string(), "model-2".to_string()])
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        if self.should_fail {
            return Err(LlmError::service_unavailable(self.name));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl ChatCompletion for MockProvider {
    async fn chat_completion(
        &self,
        _request: ChatRequest,
        model: &str,
    ) -> Result<ChatResponse, LlmError> {
        if self.should_fail {
            return Err(LlmError::api(
                self.name,
                "Mock API error",
                Some("500".to_string()),
                None,
            ));
        }

        Ok(ChatResponse {
            message: ChatMessage {
                role: ChatRole::Assistant,
                content: "Mock response".to_string(),
            },
            model: model.to_string(),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            finish_reason: Some("stop".to_string()),
            provider_metadata: None,
        })
    }

    async fn chat_completion_stream(
        &self,
        _request: ChatRequest,
        _model: &str,
        _config: Option<StreamConfig>,
    ) -> crate::types::StreamResult<crate::types::ChatStreamEvent> {
        Box::pin(futures::stream::once(async {
            Err(LlmError::model("Streaming not implemented in mock"))
        }))
    }

    async fn estimate_tokens(&self, text: &str, _model: &str) -> Result<u32, LlmError> {
        Ok(text.len() as u32 / 4) // Rough estimate
    }
}

#[tokio::test]
async fn test_mock_provider_basic_functionality() {
    let provider = MockProvider::new("test");

    assert_eq!(provider.name(), "test");

    let models = provider.available_models().await.unwrap();
    assert_eq!(models, vec!["model-1", "model-2"]);

    provider.health_check().await.unwrap();

    let token_count = provider
        .estimate_tokens("hello world", "test-model")
        .await
        .unwrap();
    assert_eq!(token_count, 2); // "hello world".len() / 4 = 11 / 4 = 2
}

#[tokio::test]
async fn test_mock_provider_chat_completion() {
    let provider = MockProvider::new("test");

    let request = ChatRequestBuilder::new()
        .user("Hello, world!")
        .temperature(0.7)
        .max_tokens(100)
        .build();

    let response = provider
        .chat_completion(request, "test-model")
        .await
        .unwrap();

    assert_eq!(response.message.role, ChatRole::Assistant);
    assert_eq!(response.message.content, "Mock response");
    assert_eq!(response.model, "test-model");
    assert_eq!(response.usage.total_tokens, 15);
}

#[tokio::test]
async fn test_mock_provider_failure_cases() {
    let provider = MockProvider::new("test").with_failure();

    let health_result = provider.health_check().await;
    assert!(health_result.is_err());
    assert!(matches!(
        health_result.unwrap_err(),
        LlmError::ServiceUnavailable { .. }
    ));

    let models_result = provider.available_models().await;
    assert!(models_result.is_err());
    assert!(matches!(
        models_result.unwrap_err(),
        LlmError::Network { .. }
    ));

    let request = ChatRequestBuilder::new().user("test").build();
    let chat_result = provider.chat_completion(request, "test-model").await;
    assert!(chat_result.is_err());
    assert!(matches!(chat_result.unwrap_err(), LlmError::Api { .. }));
}

#[test]
fn test_chat_request_builder() {
    let request = ChatRequestBuilder::new()
        .system("You are a helpful assistant")
        .user("What is 2+2?")
        .assistant("2+2 equals 4")
        .user("What about 3+3?")
        .temperature(0.8)
        .max_tokens(150)
        .top_p(0.9)
        // .frequency_penalty(0.1)
        // .presence_penalty(0.1)
        // .stop_sequences(vec!["END".to_string()])
        .stream(true)
        .extra_param("custom_param", serde_json::json!("custom_value"))
        .build();

    assert_eq!(request.messages.len(), 4);
    assert_eq!(request.messages[0].role, ChatRole::System);
    assert_eq!(request.messages[1].role, ChatRole::User);
    assert_eq!(request.messages[2].role, ChatRole::Assistant);
    assert_eq!(request.messages[3].role, ChatRole::User);
    assert_eq!(request.temperature, Some(0.8));
    assert_eq!(request.max_tokens, Some(150));
    assert_eq!(request.top_p, Some(0.9));
    // assert_eq!(request.frequency_penalty, Some(0.1));
    // assert_eq!(request.presence_penalty, Some(0.1));
    // assert_eq!(request.stop, Some(vec!["END".to_string()]));
    assert_eq!(request.stream, Some(true));
    assert!(request.extra_params.is_some());
}

#[test]
fn test_openai_config() {
    let config = OpenAIConfig::new("sk-test123")
        .with_organization("org-123")
        .with_project("proj-456");

    assert_eq!(config.api_key(), "sk-test123");
    assert_eq!(config.base_url(), "https://api.openai.com/v1");

    let headers = config.headers();
    assert_eq!(
        headers.get("Authorization"),
        Some(&"Bearer sk-test123".to_string())
    );
    assert_eq!(
        headers.get("OpenAI-Organization"),
        Some(&"org-123".to_string())
    );
    assert_eq!(headers.get("OpenAI-Project"), Some(&"proj-456".to_string()));

    config.validate().unwrap();

    // Test invalid config (empty API key)
    let invalid_config = OpenAIConfig::new("");
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_anthropic_config() {
    let config = AnthropicConfig::new("sk-ant-test123");

    assert_eq!(config.api_key(), "sk-ant-test123");
    assert_eq!(config.base_url(), "https://api.anthropic.com");

    let headers = config.headers();
    assert_eq!(
        headers.get("x-api-key"),
        Some(&"sk-ant-test123".to_string())
    );
    assert_eq!(
        headers.get("anthropic-version"),
        Some(&"2023-06-01".to_string())
    );

    config.validate().unwrap();
}

#[test]
fn test_google_ai_config() {
    let config = GoogleAiConfig::new("AIza-test123");

    assert_eq!(config.api_key(), "AIza-test123");
    assert_eq!(
        config.base_url(),
        "https://generativelanguage.googleapis.com/v1beta"
    );

    config.validate().unwrap();
}

#[test]
fn test_all_llm_error_variants() {
    use std::collections::HashMap;

    // Test Network error
    let network_error = LlmError::network("Connection failed");
    assert!(matches!(network_error, LlmError::Network { .. }));
    assert_eq!(
        network_error.to_string(),
        "Network error: Connection failed"
    );

    // Test Network error with source
    let source_error = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
    let network_with_source = LlmError::network_with_source("Connection refused", source_error);
    assert!(matches!(network_with_source, LlmError::Network { .. }));

    // Test Authentication error
    let auth_error = LlmError::authentication("Invalid API key");
    assert!(matches!(auth_error, LlmError::Authentication { .. }));
    assert_eq!(
        auth_error.to_string(),
        "Authentication failed: Invalid API key"
    );

    // Test RateLimit error
    let rate_limit = LlmError::rate_limit(
        "Too many requests",
        Some(std::time::Duration::from_secs(60)),
    );
    assert!(matches!(rate_limit, LlmError::RateLimit { .. }));
    assert!(rate_limit.to_string().contains("Rate limit exceeded"));

    // Test Api error with details
    let mut details = HashMap::new();
    details.insert(
        "error_code".to_string(),
        serde_json::Value::String("QUOTA_EXCEEDED".to_string()),
    );
    let api_error = LlmError::api(
        "openai",
        "Quota exceeded",
        Some("429".to_string()),
        Some(details),
    );
    assert!(matches!(api_error, LlmError::Api { .. }));
    assert!(api_error.to_string().contains("API error from openai"));

    // Test Configuration error
    let config_error = LlmError::configuration("Missing API key");
    assert!(matches!(config_error, LlmError::Configuration { .. }));
    assert_eq!(
        config_error.to_string(),
        "Configuration error: Missing API key"
    );

    // Test Validation error
    let validation_error = LlmError::validation("Invalid model name");
    assert!(matches!(validation_error, LlmError::Validation { .. }));
    assert_eq!(
        validation_error.to_string(),
        "Validation error: Invalid model name"
    );

    // Test Timeout error
    let timeout_error = LlmError::timeout(std::time::Duration::from_secs(30));
    assert!(matches!(timeout_error, LlmError::Timeout { .. }));
    assert!(timeout_error.to_string().contains("Request timed out"));

    // Test Serialization error
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let serialization_error = LlmError::serialization("JSON parse failed", json_error);
    assert!(matches!(
        serialization_error,
        LlmError::Serialization { .. }
    ));
    assert!(
        serialization_error
            .to_string()
            .contains("Serialization error")
    );

    // Test Model error
    let model_error = LlmError::model("Model not found");
    assert!(matches!(model_error, LlmError::Model { .. }));
    assert_eq!(model_error.to_string(), "Model error: Model not found");

    // Test Resource error
    let resource_error = LlmError::resource("Insufficient credits");
    assert!(matches!(resource_error, LlmError::Resource { .. }));
    assert_eq!(
        resource_error.to_string(),
        "Resource error: Insufficient credits"
    );

    // Test ServiceUnavailable error
    let service_error = LlmError::service_unavailable("anthropic");
    assert!(matches!(service_error, LlmError::ServiceUnavailable { .. }));
    assert_eq!(
        service_error.to_string(),
        "Service unavailable: anthropic is currently unavailable"
    );

    // Test Unknown error
    let unknown_error = LlmError::unknown("Unexpected error");
    assert!(matches!(unknown_error, LlmError::Unknown { .. }));
    assert_eq!(
        unknown_error.to_string(),
        "Unexpected error: Unexpected error"
    );

    // Test Unknown error with source
    let io_error = std::io::Error::other("unknown");
    let unknown_with_source = LlmError::unknown_with_source("Something went wrong", io_error);
    assert!(matches!(unknown_with_source, LlmError::Unknown { .. }));
}

#[test]
fn test_error_conversions() {
    // Test From<serde_json::Error> conversion
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let llm_error: LlmError = json_error.into();
    assert!(matches!(llm_error, LlmError::Serialization { .. }));
    assert!(llm_error.to_string().contains("JSON serialization failed"));

    // Test From<reqwest::Error> for timeout
    // Note: We can't easily create specific reqwest errors without making actual requests
    // so we test the error conversion logic indirectly through the provider implementations
}

#[test]
fn test_provider_specific_error_mapping() {
    use std::collections::HashMap;

    // Test OpenAI-style error mapping
    let mut openai_details = HashMap::new();
    openai_details.insert(
        "type".to_string(),
        serde_json::Value::String("insufficient_quota".to_string()),
    );
    openai_details.insert("param".to_string(), serde_json::Value::Null);

    let openai_quota_error = LlmError::api(
        "openai",
        "You exceeded your current quota, please check your plan and billing details.",
        Some("429".to_string()),
        Some(openai_details.clone()),
    );

    assert!(matches!(openai_quota_error, LlmError::Api { .. }));
    if let LlmError::Api {
        provider,
        message,
        code,
        details,
    } = openai_quota_error
    {
        assert_eq!(provider, "openai");
        assert!(message.contains("quota"));
        assert_eq!(code, Some("429".to_string()));
        assert!(details.is_some());
        let details = details.unwrap();
        assert_eq!(
            details.get("type").unwrap(),
            &serde_json::Value::String("insufficient_quota".to_string())
        );
    }

    // Test Anthropic-style error mapping
    let mut anthropic_details = HashMap::new();
    anthropic_details.insert(
        "error_type".to_string(),
        serde_json::Value::String("authentication_error".to_string()),
    );

    let anthropic_auth_error = LlmError::api(
        "anthropic",
        "Invalid API key provided",
        Some("401".to_string()),
        Some(anthropic_details.clone()),
    );

    assert!(matches!(anthropic_auth_error, LlmError::Api { .. }));
    if let LlmError::Api {
        provider,
        message,
        code,
        details,
    } = anthropic_auth_error
    {
        assert_eq!(provider, "anthropic");
        assert_eq!(message, "Invalid API key provided");
        assert_eq!(code, Some("401".to_string()));
        assert!(details.is_some());
        let details = details.unwrap();
        assert_eq!(
            details.get("error_type").unwrap(),
            &serde_json::Value::String("authentication_error".to_string())
        );
    }

    // Test Google-style error mapping
    let mut google_details = HashMap::new();
    google_details.insert(
        "reason".to_string(),
        serde_json::Value::String("RATE_LIMIT_EXCEEDED".to_string()),
    );
    google_details.insert(
        "domain".to_string(),
        serde_json::Value::String("global".to_string()),
    );

    let google_rate_error = LlmError::api(
        "google",
        "Rate limit exceeded",
        Some("429".to_string()),
        Some(google_details.clone()),
    );

    assert!(matches!(google_rate_error, LlmError::Api { .. }));
    if let LlmError::Api {
        provider,
        message,
        code,
        details,
    } = google_rate_error
    {
        assert_eq!(provider, "google");
        assert_eq!(message, "Rate limit exceeded");
        assert_eq!(code, Some("429".to_string()));
        assert!(details.is_some());
        let details = details.unwrap();
        assert_eq!(
            details.get("reason").unwrap(),
            &serde_json::Value::String("RATE_LIMIT_EXCEEDED".to_string())
        );
    }
}

#[test]
fn test_error_source_chaining() {
    use std::error::Error;

    // Test that source errors are properly chained
    let io_error = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Connection refused");
    let network_error = LlmError::network_with_source("Failed to connect", io_error);

    // Verify the error source is accessible
    assert!(network_error.source().is_some());
    assert!(matches!(network_error, LlmError::Network { .. }));

    // Test serialization error with source
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let serialization_error = LlmError::serialization("Failed to parse JSON", json_error);

    assert!(serialization_error.source().is_some());
    assert!(matches!(
        serialization_error,
        LlmError::Serialization { .. }
    ));

    // Test unknown error with source
    let unknown_source = std::io::Error::other("mysterious error");
    let unknown_error =
        LlmError::unknown_with_source("Something unexpected happened", unknown_source);

    assert!(unknown_error.source().is_some());
    assert!(matches!(unknown_error, LlmError::Unknown { .. }));
}

#[test]
fn test_openai_provider_creation() {
    use crate::config::OpenAIConfig;
    use crate::providers::OpenAIProvider;

    // Test with valid config
    let config = OpenAIConfig::new("sk-test123");
    let provider = OpenAIProvider::new(config);
    assert!(provider.is_ok());

    // Test with invalid config (empty API key)
    let invalid_config = OpenAIConfig::new("");
    let invalid_provider = OpenAIProvider::new(invalid_config);
    assert!(invalid_provider.is_err());
}

#[tokio::test]
async fn test_openai_request_conversion() {
    use crate::config::OpenAIConfig;
    use crate::providers::OpenAIProvider;

    let config = OpenAIConfig::new("sk-test123");
    let provider = OpenAIProvider::new(config).unwrap();

    let _request = ChatRequestBuilder::new()
        .system("You are helpful")
        .user("Hello")
        .temperature(0.7)
        .max_tokens(100)
        .build();

    // We can't easily test the private method, but we can verify
    // the provider was created successfully and implements the traits
    assert_eq!(provider.name(), "openai");
}

#[tokio::test]
async fn test_openai_token_estimation() {
    use crate::config::OpenAIConfig;
    use crate::providers::OpenAIProvider;

    let config = OpenAIConfig::new("sk-test123");
    let provider = OpenAIProvider::new(config).unwrap();

    let tokens = provider
        .estimate_tokens("Hello world", "gpt-4")
        .await
        .unwrap();

    // Should be roughly 3 tokens for "Hello world" (8 chars / 4 = 2, rounded up to 3)
    assert!((2..=4).contains(&tokens));
}

#[test]
fn test_anthropic_provider_creation() {
    use crate::config::AnthropicConfig;
    use crate::providers::AnthropicProvider;

    // Test with valid config
    let config = AnthropicConfig::new("sk-ant-test123");
    let provider = AnthropicProvider::new(config);
    assert!(provider.is_ok());

    // Test with invalid config (empty API key)
    let invalid_config = AnthropicConfig::new("");
    let invalid_provider = AnthropicProvider::new(invalid_config);
    assert!(invalid_provider.is_err());
}

#[tokio::test]
async fn test_anthropic_token_estimation() {
    use crate::config::AnthropicConfig;
    use crate::providers::AnthropicProvider;

    let config = AnthropicConfig::new("sk-ant-test123");
    let provider = AnthropicProvider::new(config).unwrap();

    let tokens = provider
        .estimate_tokens("Hello world", "claude-3-haiku-20240307")
        .await
        .unwrap();

    // Should be roughly 3 tokens for "Hello world" (11 chars / 3.5 ≈ 3.14, rounded up to 4)
    assert!((3..=5).contains(&tokens));
}

#[test]
fn test_google_provider_creation() {
    use crate::config::GoogleAiConfig;
    use crate::providers::GoogleProvider;

    // Test with valid config
    let config = GoogleAiConfig::new("AIza-test123");
    let provider = GoogleProvider::new(config);
    assert!(provider.is_ok());

    // Test with invalid config (empty API key)
    let invalid_config = GoogleAiConfig::new("");
    let invalid_provider = GoogleProvider::new(invalid_config);
    assert!(invalid_provider.is_err());
}

#[tokio::test]
async fn test_google_token_estimation() {
    use crate::config::GoogleAiConfig;
    use crate::providers::GoogleProvider;

    let config = GoogleAiConfig::new("AIza-test123");
    let provider = GoogleProvider::new(config).unwrap();

    let tokens = provider
        .estimate_tokens("Hello world", "gemini-1.5-pro")
        .await
        .unwrap();

    // Should be roughly 3 tokens for "Hello world" (11 chars / 4 ≈ 2.75, rounded up to 3)
    assert!((2..=4).contains(&tokens));
}

#[test]
fn test_google_request_format() {
    use crate::config::GoogleAiConfig;
    use crate::providers::GoogleProvider;

    let config = GoogleAiConfig::new("AIza-test123");
    let provider = GoogleProvider::new(config).unwrap();

    let request = ChatRequestBuilder::new()
        .system("You are a helpful assistant")
        .user("Hello")
        .assistant("Hi there!")
        .user("How are you?")
        .temperature(0.7)
        .max_tokens(100)
        .top_p(0.9)
        // .stop_sequences(vec!["END".to_string()])
        .build();

    // We can't easily test the private method directly, but we can verify
    // the provider was created successfully and test the format indirectly
    // by testing the available functionality
    assert_eq!(provider.name(), "google");

    // The request should have system message separated from user/assistant messages
    let system_messages: Vec<_> = request
        .messages
        .iter()
        .filter(|m| m.role == ChatRole::System)
        .collect();
    assert_eq!(system_messages.len(), 1);
    assert_eq!(system_messages[0].content, "You are a helpful assistant");

    // Should have user and assistant messages
    let conversation_messages: Vec<_> = request
        .messages
        .iter()
        .filter(|m| matches!(m.role, ChatRole::User | ChatRole::Assistant))
        .collect();
    assert_eq!(conversation_messages.len(), 3);
}

#[test]
fn test_anthropic_request_format() {
    use crate::config::AnthropicConfig;
    use crate::providers::AnthropicProvider;

    let config = AnthropicConfig::new("sk-ant-test123");
    let provider = AnthropicProvider::new(config).unwrap();

    let request = ChatRequestBuilder::new()
        .system("You are a helpful assistant")
        .user("Hello")
        .assistant("Hi there!")
        .user("How are you?")
        .temperature(0.7)
        .max_tokens(100)
        .top_p(0.9)
        // .stop_sequences(vec!["END".to_string()])
        .build();

    // We can't easily test the private method directly, but we can verify
    // the provider was created successfully and test the format indirectly
    // by testing the available functionality
    assert_eq!(provider.name(), "anthropic");

    // The request should have system message separated from user/assistant messages
    let system_messages: Vec<_> = request
        .messages
        .iter()
        .filter(|m| m.role == ChatRole::System)
        .collect();
    let conversation_messages: Vec<_> = request
        .messages
        .iter()
        .filter(|m| m.role != ChatRole::System)
        .collect();

    assert_eq!(system_messages.len(), 1);
    assert_eq!(conversation_messages.len(), 3);
    assert_eq!(system_messages[0].content, "You are a helpful assistant");
}

#[test]
fn test_anthropic_response_parsing() {
    use crate::config::AnthropicConfig;
    use crate::providers::AnthropicProvider;

    let config = AnthropicConfig::new("sk-ant-test123");
    let provider = AnthropicProvider::new(config).unwrap();

    // Test parsing a mock Anthropic response
    let mock_response = serde_json::json!({
        "content": [
            {
                "type": "text",
                "text": "Hello! I'm Claude, an AI assistant."
            }
        ],
        "id": "msg_test123",
        "model": "claude-3-haiku-20240307",
        "role": "assistant",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "type": "message",
        "usage": {
            "input_tokens": 15,
            "output_tokens": 25
        }
    });

    let result = provider.parse_anthropic_response(mock_response);
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.message.role, ChatRole::Assistant);
    assert_eq!(
        response.message.content,
        "Hello! I'm Claude, an AI assistant."
    );
    assert_eq!(response.model, "claude-3-haiku-20240307");
    assert_eq!(response.usage.prompt_tokens, 15);
    assert_eq!(response.usage.completion_tokens, 25);
    assert_eq!(response.usage.total_tokens, 40);
    assert_eq!(response.finish_reason, Some("end_turn".to_string()));
}

#[test]
fn test_anthropic_response_parsing_errors() {
    use crate::config::AnthropicConfig;
    use crate::providers::AnthropicProvider;

    let config = AnthropicConfig::new("sk-ant-test123");
    let provider = AnthropicProvider::new(config).unwrap();

    // Test missing content array
    let invalid_response = serde_json::json!({
        "model": "claude-3-haiku-20240307",
        "usage": {"input_tokens": 10, "output_tokens": 5}
    });

    let result = provider.parse_anthropic_response(invalid_response);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        LlmError::Serialization { .. }
    ));

    // Test empty content array
    let empty_content_response = serde_json::json!({
        "content": [],
        "model": "claude-3-haiku-20240307",
        "usage": {"input_tokens": 10, "output_tokens": 5}
    });

    let result = provider.parse_anthropic_response(empty_content_response);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        LlmError::Serialization { .. }
    ));

    // Test missing text in content block
    let missing_text_response = serde_json::json!({
        "content": [{"type": "text"}],
        "model": "claude-3-haiku-20240307",
        "usage": {"input_tokens": 10, "output_tokens": 5}
    });

    let result = provider.parse_anthropic_response(missing_text_response);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        LlmError::Serialization { .. }
    ));
}

// =============================================================================
// Middleware Tests
// =============================================================================

#[test]
fn test_middleware_config_default() {
    let config = MiddlewareConfig::default();

    assert_eq!(config.timeout, Some(Duration::from_secs(30)));
    assert!(config.rate_limit.is_none());
    assert!(config.enable_logging);
    assert!(!config.enable_metrics);
}

#[test]
fn test_middleware_config_custom() {
    let rate_limit = RateLimit {
        requests_per_period: 100,
        period: Duration::from_secs(60),
    };

    let config = MiddlewareConfig {
        timeout: Some(Duration::from_secs(45)),
        rate_limit: Some(rate_limit.clone()),
        enable_logging: false,
        enable_metrics: true,
    };

    assert_eq!(config.timeout, Some(Duration::from_secs(45)));
    assert!(config.rate_limit.is_some());
    assert!(!config.enable_logging);
    assert!(config.enable_metrics);

    let rate_limit_config = config.rate_limit.as_ref().unwrap();
    assert_eq!(rate_limit_config.requests_per_period, 100);
    assert_eq!(rate_limit_config.period, Duration::from_secs(60));
}

#[test]
fn test_llm_service_builder_default() {
    let provider = MockProvider::new("test");
    let middleware_stack = LlmServiceBuilder::new().build(provider, "test-model".to_string());

    let config = middleware_stack.config();
    assert_eq!(config.timeout, Some(Duration::from_secs(30)));
    assert!(config.enable_logging);
    assert!(!config.enable_metrics);
}

#[test]
fn test_llm_service_builder_fluent_api() {
    let provider = MockProvider::new("test");

    let middleware_stack = LlmServiceBuilder::new()
        .timeout(Duration::from_secs(60))
        .rate_limit(50, Duration::from_secs(30))
        .logging()
        .metrics()
        .build(provider, "test-model".to_string());

    let config = middleware_stack.config();
    assert_eq!(config.timeout, Some(Duration::from_secs(60)));
    assert!(config.rate_limit.is_some());
    assert!(config.enable_logging);
    assert!(config.enable_metrics);

    let rate_limit = config.rate_limit.as_ref().unwrap();
    assert_eq!(rate_limit.requests_per_period, 50);
    assert_eq!(rate_limit.period, Duration::from_secs(30));
}

#[test]
fn test_llm_service_builder_with_config() {
    let custom_config = MiddlewareConfig {
        timeout: Some(Duration::from_secs(20)),
        rate_limit: None,
        enable_logging: false,
        enable_metrics: true,
    };

    let provider = MockProvider::new("test");
    let middleware_stack = LlmServiceBuilder::with_config(custom_config.clone())
        .build(provider, "test-model".to_string());

    let config = middleware_stack.config();
    assert_eq!(config.timeout, custom_config.timeout);
    assert_eq!(config.enable_logging, custom_config.enable_logging);
    assert_eq!(config.enable_metrics, custom_config.enable_metrics);
}

#[tokio::test]
async fn test_middleware_stack_basic_call() {
    let provider = MockProvider::new("test");
    let mut middleware_stack = LlmServiceBuilder::new()
        .logging()
        .build(provider, "test-model".to_string());

    let request = ChatRequestBuilder::new().user("Hello, middleware!").build();

    let response = middleware_stack.call(request).await.unwrap();

    assert_eq!(response.message.content, "Mock response");
    assert_eq!(response.model, "test-model");
    assert_eq!(response.usage.total_tokens, 15);
}

#[tokio::test]
async fn test_middleware_logging_and_metrics() {
    let provider = MockProvider::new("test");
    let mut middleware_stack = LlmServiceBuilder::new()
        .logging()
        .metrics()
        .build(provider, "test-model".to_string());

    let request = ChatRequestBuilder::new()
        .user("Test logging and metrics")
        .build();

    // This test mainly ensures the logging/metrics code doesn't crash
    // In a real scenario, you'd capture log output and verify metrics
    let response = middleware_stack.call(request).await.unwrap();

    assert_eq!(response.message.content, "Mock response");

    let config = middleware_stack.config();
    assert!(config.enable_logging);
    assert!(config.enable_metrics);
}

#[test]
fn test_rate_limit_configuration() {
    let rate_limit = RateLimit {
        requests_per_period: 100,
        period: Duration::from_secs(60),
    };

    assert_eq!(rate_limit.requests_per_period, 100);
    assert_eq!(rate_limit.period, Duration::from_secs(60));

    // Test with different values
    let rate_limit2 = RateLimit {
        requests_per_period: 50,
        period: Duration::from_secs(30),
    };

    assert_eq!(rate_limit2.requests_per_period, 50);
    assert_eq!(rate_limit2.period, Duration::from_secs(30));
}

#[tokio::test]
async fn test_middleware_error_propagation() {
    let provider = MockProvider::new("test").with_failure();
    let mut middleware_stack = LlmServiceBuilder::new()
        .logging()
        .build(provider, "test-model".to_string());

    let request = ChatRequestBuilder::new().user("This will fail").build();

    let result = middleware_stack.call(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, LlmError::Api { .. }));
}

#[test]
fn test_middleware_config_inspection() {
    let custom_config = MiddlewareConfig {
        timeout: Some(Duration::from_secs(25)),
        rate_limit: Some(RateLimit {
            requests_per_period: 75,
            period: Duration::from_secs(45),
        }),
        enable_logging: true,
        enable_metrics: false,
    };

    let provider = MockProvider::new("test");
    let middleware_stack = LlmServiceBuilder::with_config(custom_config.clone())
        .build(provider, "test-model".to_string());

    let config = middleware_stack.config();

    // Verify all configuration values are preserved
    assert_eq!(config.timeout, Some(Duration::from_secs(25)));
    assert!(config.enable_logging);
    assert!(!config.enable_metrics);

    if let Some(rate_limit) = &config.rate_limit {
        assert_eq!(rate_limit.requests_per_period, 75);
        assert_eq!(rate_limit.period, Duration::from_secs(45));
    } else {
        panic!("Expected rate limit configuration");
    }
}

#[tokio::test]
async fn test_middleware_performance_timing() {
    let provider = MockProvider::new("test");
    let mut middleware_stack = LlmServiceBuilder::new()
        .metrics() // Enable metrics to test timing logic
        .build(provider, "test-model".to_string());

    let request = ChatRequestBuilder::new().user("Performance test").build();

    let start = std::time::Instant::now();
    let _response = middleware_stack.call(request).await.unwrap();
    let duration = start.elapsed();

    // The call should complete relatively quickly for a mock provider
    assert!(duration < Duration::from_secs(1));
}
