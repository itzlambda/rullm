//! # rullm-chat-completion
//!
//! An idiomatic Rust client for the OpenAI Chat Completions API.
//!
//! This library provides a provider-agnostic client for any OpenAI-compatible
//! chat completions endpoint, with full support for streaming, tools, and
//! structured outputs.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rullm_chat_completion::{ChatCompletionsClient, ClientConfig, AuthConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ClientConfig::builder()
//!         .bearer_token("your-api-key")
//!         .build()?;
//!
//!     let client = ChatCompletionsClient::new(config)?;
//!
//!     let response = client.chat()
//!         .model("gpt-4o")
//!         .system("You are a helpful assistant.")
//!         .user("Hello!")
//!         .send()
//!         .await?;
//!
//!     println!("{}", response.data.first_text().unwrap_or("No response"));
//!     Ok(())
//! }
//! ```
//!
//! ## Streaming
//!
//! ```rust,no_run
//! use futures::StreamExt;
//! use rullm_chat_completion::{ChatCompletionsClient, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ClientConfig::builder()
//!         .bearer_token("your-api-key")
//!         .build()?;
//!
//!     let client = ChatCompletionsClient::new(config)?;
//!
//!     let mut stream = client.chat()
//!         .model("gpt-4o")
//!         .user("Tell me a story")
//!         .stream()
//!         .await?;
//!
//!     while let Some(chunk) = stream.next().await {
//!         let chunk = chunk?;
//!         if let Some(content) = chunk.choices.first()
//!             .and_then(|c| c.delta.content.as_ref())
//!         {
//!             print!("{}", content);
//!         }
//!     }
//!     Ok(())
//! }
//! ```

mod client;
mod config;
mod error;
mod streaming;
mod types;

// Re-export main types
pub use client::{ChatCompletionsClient, ChatRequestBuilder};
pub use config::{
    ApiResponse, AuthConfig, ClientConfig, ClientConfigBuilder, RateLimitInfo, ResponseMeta,
};
pub use error::{ApiError, ApiErrorBody, ClientError, DeserializeError, HttpError, StreamError};
pub use streaming::{ChatCompletionAccumulator, ChatCompletionStream};
pub use types::{
    ApproximateLocation, AssistantAudio, AudioConfig, ChatChoice, ChatChunkChoice, ChatCompletion,
    ChatCompletionChunk, ChatCompletionRequest, ChunkDelta, CompletionTokensDetails, ContentPart,
    FilePart, FunctionCall, FunctionCallDelta, FunctionDefinition, ImageUrlPart, InputAudioPart,
    JsonSchemaFormat, Logprobs, Message, MessageContent, ModelId, Prediction, PromptTokensDetails,
    ResponseFormat, Role, Stop, StreamOptions, TokenLogprob, ToolCall, ToolCallDelta, ToolChoice,
    ToolChoiceFunction, ToolDefinition, TopLogprob, Usage, UserLocation, WebSearchOptions,
};
