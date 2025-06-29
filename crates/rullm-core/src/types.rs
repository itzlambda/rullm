use crate::error::LlmError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;

/// Represents a message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// Role of the message sender
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Request parameters for chat completions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    // pub frequency_penalty: Option<f32>,
    // pub presence_penalty: Option<f32>,
    // pub stop: Option<Vec<String>>,
    pub stream: Option<bool>,
    pub extra_params: Option<HashMap<String, serde_json::Value>>,
}

/// Response from chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub model: String,
    pub usage: TokenUsage,
    pub finish_reason: Option<String>,
    pub provider_metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Events emitted during streaming chat completions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatStreamEvent {
    /// A token (piece of text) from the streaming response
    Token(String),
    /// The stream has completed successfully
    Done,
    /// An error occurred during streaming
    Error(String),
}

/// Type alias for streaming results
pub type StreamResult<T> = Pin<Box<dyn futures::Stream<Item = Result<T, LlmError>> + Send>>;

/// Configuration for streaming responses
#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub buffer_size: usize,
    pub timeout_ms: u64,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1024,
            timeout_ms: 30000,
        }
    }
}

/// Main trait for LLM providers
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Canonical provider identifier (e.g. "openai", "anthropic").
    fn name(&self) -> &'static str;

    /// Alternative identifiers that should map to this provider (e.g. ["gpt"], ["claude"], ...).
    fn aliases(&self) -> &'static [&'static str];

    /// Environment variable expected to contain the provider API key.
    fn env_key(&self) -> &'static str;

    /// Default base URL used when no explicit endpoint is supplied.
    fn default_base_url(&self) -> Option<&'static str>;

    /// Get available models for this provider
    async fn available_models(&self) -> Result<Vec<String>, LlmError>;

    /// Check if the provider is properly configured
    async fn health_check(&self) -> Result<(), LlmError>;
}

/// Trait for chat-based interactions
#[async_trait::async_trait]
pub trait ChatProvider: LlmProvider {
    /// Send a chat completion request
    async fn chat_completion(
        &self,
        request: ChatRequest,
        model: &str,
    ) -> Result<ChatResponse, LlmError>;

    /// Send a streaming chat completion request
    async fn chat_completion_stream(
        &self,
        _request: ChatRequest,
        _model: &str,
        _config: Option<StreamConfig>,
    ) -> StreamResult<ChatStreamEvent> {
        // Default implementation returns NotImplemented error
        Box::pin(futures::stream::once(async {
            Err(LlmError::model(
                "chat_completion_stream not implemented for this provider",
            ))
        }))
    }

    /// Estimate token count for a given text (provider-specific)
    async fn estimate_tokens(&self, text: &str, model: &str) -> Result<u32, LlmError>;
}

/// Builder pattern for chat requests
#[derive(Default)]
pub struct ChatRequestBuilder {
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    // frequency_penalty: Option<f32>,
    // presence_penalty: Option<f32>,
    // stop: Option<Vec<String>>,
    stream: Option<bool>,
    extra_params: Option<HashMap<String, serde_json::Value>>,
}

impl ChatRequestBuilder {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            // frequency_penalty: None,
            // presence_penalty: None,
            // stop: None,
            stream: None,
            extra_params: None,
        }
    }

    pub fn add_message(mut self, role: ChatRole, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage {
            role,
            content: content.into(),
        });
        self
    }

    pub fn system(self, content: impl Into<String>) -> Self {
        self.add_message(ChatRole::System, content)
    }

    pub fn user(self, content: impl Into<String>) -> Self {
        self.add_message(ChatRole::User, content)
    }

    pub fn assistant(self, content: impl Into<String>) -> Self {
        self.add_message(ChatRole::Assistant, content)
    }

    pub fn tool(self, content: impl Into<String>) -> Self {
        self.add_message(ChatRole::Tool, content)
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    // pub fn frequency_penalty(mut self, penalty: f32) -> Self {
    //     self.frequency_penalty = Some(penalty);
    //     self
    // }

    // pub fn presence_penalty(mut self, penalty: f32) -> Self {
    //     self.presence_penalty = Some(penalty);
    //     self
    // }

    // pub fn stop_sequences(mut self, stop: Vec<String>) -> Self {
    //     self.stop = Some(stop);
    //     self
    // }

    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    pub fn extra_param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra_params
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value);
        self
    }

    pub fn build(self) -> ChatRequest {
        ChatRequest {
            messages: self.messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            top_p: self.top_p,
            // frequency_penalty: self.frequency_penalty,
            // presence_penalty: self.presence_penalty,
            // stop: self.stop,
            stream: self.stream,
            extra_params: self.extra_params,
        }
    }
}
