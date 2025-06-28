//! # rullm-core - Rust LLM Library
//!
//! A high-performance Rust library for interacting with Large Language Models (LLMs).
//! Built with Tower middleware for enterprise-grade reliability, featuring retry logic,
//! rate limiting, circuit breakers, and comprehensive error handling.
//!
//! ## Features
//!
//! - **Multiple LLM Providers** - OpenAI, Anthropic, Google AI
//! - **High Performance** - Built on Tower with connection pooling and async/await
//! - **Enterprise Ready** - Retry logic, rate limiting, circuit breakers, timeouts
//! - **Dual APIs** - Simple string-based API + advanced API with full control
//! - **Real-time Streaming** - Stream responses token by token as they're generated
//! - **Well Tested** - Comprehensive test suite with examples
//! - **Observability** - Built-in metrics, logging, and error handling
//!
//! ## Quick Start
//!
//! ### Simple API (Recommended)
//!
//! ```rust,no_run
//! use rullm_core::simple::{SimpleLlm, SimpleLlmClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = SimpleLlmClient::openai("your-api-key")?;
//!     let response = client.chat("What is the capital of France?").await?;
//!     println!("Response: {}", response);
//!     Ok(())
//! }
//! ```
//!
//! ### Advanced API (Full Control)
//!
//! ```rust,no_run
//! use rullm_core::{OpenAIConfig, OpenAIProvider, ChatProvider, ChatRequestBuilder, ChatRole};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = OpenAIConfig::new("your-api-key");
//!     let provider = OpenAIProvider::new(config)?;
//!     
//!     let request = ChatRequestBuilder::new()
//!         .user("Hello, world!")
//!         .temperature(0.7)
//!         .max_tokens(100)
//!         .build();
//!     
//!     let response = provider.chat_completion(request, "gpt-3.5-turbo").await?;
//!     println!("Response: {}", response.message.content);
//!     Ok(())
//! }
//! ```
//!
//! ## Streaming API Overview
//!
//! The streaming API enables real-time token-by-token responses, perfect for interactive
//! chat applications and live user experiences.
//!
//! ### Core Streaming Types
//!
//! - [`ChatStreamEvent`] - Events emitted during streaming (Token, Done, Error)
//! - [`StreamResult`] - Type alias for `Pin<Box<dyn Stream<Item = Result<ChatStreamEvent, LlmError>>>>`
//! - [`ChatProvider::chat_completion_stream`] - Main streaming method for all providers
//!
//! ### Basic Streaming Usage
//!
//! ```rust,no_run
//! use rullm_core::{OpenAIProvider, ChatProvider, ChatRequestBuilder, ChatStreamEvent};
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let provider = OpenAIProvider::new(/* config */)?;
//!     
//!     let request = ChatRequestBuilder::new()
//!         .user("Tell me a story")
//!         .stream(true) // Enable streaming
//!         .build();
//!
//!     let mut stream = provider
//!         .chat_completion_stream(request, "gpt-3.5-turbo", None)
//!         .await;
//!
//!     while let Some(event) = stream.next().await {
//!         match event? {
//!             ChatStreamEvent::Token(token) => {
//!                 print!("{}", token);
//!                 std::io::Write::flush(&mut std::io::stdout())?;
//!             }
//!             ChatStreamEvent::Done => {
//!                 println!("\n✅ Stream completed");
//!                 break;
//!             }
//!             ChatStreamEvent::Error(error) => {
//!                 println!("\n❌ Stream error: {}", error);
//!                 break;
//!             }
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ### Streaming Examples
//!
//! The library includes comprehensive streaming examples for each provider:
//!
//! - `openai_stream.rs` - OpenAI GPT models streaming
//! - `anthropic_stream.rs` - Anthropic Claude models streaming  
//! - `gemini_stream.rs` - Google Gemini models streaming
//!
//! Run examples with:
//! ```bash
//! cargo run --example openai_stream     # Requires OPENAI_API_KEY
//! cargo run --example anthropic_stream  # Requires ANTHROPIC_API_KEY
//! cargo run --example gemini_stream     # Requires GOOGLE_API_KEY
//! ```
//!
//! ### Provider-Specific Streaming Features
//!
//! | Provider | Models | Key Features |
//! |----------|--------|--------------|
//! | OpenAI | GPT-3.5, GPT-4 | Token counting, creative writing |
//! | Anthropic | Claude 3 variants | Reasoning, code analysis |
//! | Google | Gemini 1.5/2.0 | Multimodal, experimental models |
//!
//! ## Error Handling
//!
//! All operations return [`Result<T, LlmError>`](LlmError) for comprehensive error handling:
//!
//! ```rust,no_run
//! use rullm_core::{LlmError, OpenAIProvider, ChatProvider};
//!
//! match provider.chat_completion(request, "gpt-4").await {
//!     Ok(response) => println!("Success: {}", response.message.content),
//!     Err(LlmError::Authentication(_)) => println!("Invalid API key"),
//!     Err(LlmError::RateLimit { retry_after }) => {
//!         println!("Rate limited, retry after: {:?}", retry_after);
//!     }
//!     Err(e) => println!("Other error: {}", e),
//! }
//! ```

pub mod config;
pub mod error;
pub mod middleware;
pub mod providers;
pub mod simple;
pub mod types;
pub mod utils;

#[cfg(test)]
mod tests;

pub use config::{AnthropicConfig, ConfigBuilder, GoogleAiConfig, OpenAIConfig, ProviderConfig};
pub use error::LlmError;
pub use middleware::{LlmServiceBuilder, MiddlewareConfig, MiddlewareStack, RateLimit};
pub use providers::Provider;
pub use providers::{AnthropicProvider, GoogleProvider, OpenAIProvider};
pub use simple::{DefaultModels, SimpleLlm, SimpleLlmBuilder, SimpleLlmClient, SimpleLlmConfig};
pub use types::{
    ChatMessage, ChatProvider, ChatRequest, ChatRequestBuilder, ChatResponse, ChatRole,
    ChatStreamEvent, LlmProvider, StreamConfig, StreamResult, TokenUsage,
};
pub use utils::sse::sse_lines;

// Re-export test utilities for integration tests and examples
#[cfg(test)]
pub use utils::test_helpers;

// Re-export commonly used types
pub use serde::{Deserialize, Serialize};

pub use providers::ProviderExt;
