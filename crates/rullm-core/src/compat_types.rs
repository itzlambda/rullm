//! Compatibility types for OpenAI-compatible provider
//!
//! These are minimal types to support Groq/OpenRouter through the OpenAICompatibleProvider.
//! For full-featured usage, use the concrete provider clients directly.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub model: String,
    pub usage: TokenUsage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatStreamEvent {
    Token(String),
    Done,
    Error(String),
}

pub struct ChatRequestBuilder {
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
}

impl ChatRequestBuilder {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            top_p: None,
        }
    }

    pub fn add_message(mut self, role: ChatRole, content: impl Into<String>) -> Self {
        self.messages.push(ChatMessage {
            role,
            content: content.into(),
        });
        self
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

    pub fn build(self) -> ChatRequest {
        ChatRequest {
            messages: self.messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            top_p: self.top_p,
            stream: Some(false),
        }
    }
}

impl Default for ChatRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}
