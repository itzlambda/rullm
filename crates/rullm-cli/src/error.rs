//! CLI error types
//!
//! Unified error handling for the CLI that wraps errors from provider crates.

use std::sync::Arc;
use thiserror::Error;

use rullm_anthropic::AnthropicError;
use rullm_chat_completion::ClientError;

/// Main error type for CLI operations
#[derive(Error, Debug)]
pub enum CliError {
    /// Error from the Anthropic provider
    #[error("Anthropic error: {0}")]
    Anthropic(#[from] AnthropicError),

    /// Error from the chat completion provider (OpenAI, Groq, OpenRouter)
    #[error("Chat completion error: {0}")]
    ChatCompletion(#[from] ClientError),

    /// Validation error (invalid input, configuration, etc.)
    #[error("Validation error: {0}")]
    Validation(Arc<str>),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl CliError {
    /// Create a validation error
    pub fn validation(msg: impl Into<Arc<str>>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a generic error
    pub fn unknown(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
