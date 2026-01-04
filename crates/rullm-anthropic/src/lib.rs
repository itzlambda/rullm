//! Anthropic Messages API Rust Client
//!
//! A feature-complete, idiomatic Rust client for the Anthropic Messages API.
//!
//! # Features
//!
//! - Full Messages API support with all parameters
//! - Ergonomic builder patterns for requests
//! - Streaming support with high-level helpers
//! - Strong typing for all API objects
//! - Comprehensive error handling
//!
//! # Quick Start
//!
//! ```no_run
//! use rullm_anthropic::{Client, Message, MessagesRequest, RequestOptions};
//!
//! # async fn example() -> Result<(), rullm_anthropic::AnthropicError> {
//! // Create client from environment
//! let client = Client::from_env()?;
//!
//! // Build a request
//! let request = MessagesRequest::builder("claude-3-5-sonnet-20241022", 1024)
//!     .system("You are a helpful assistant.")
//!     .message(Message::user("Hello!"))
//!     .temperature(0.7)
//!     .build();
//!
//! // Send request
//! let response = client.messages().create(request, RequestOptions::default()).await?;
//! println!("{}", response.text());
//! # Ok(())
//! # }
//! ```
//!
//! # Streaming
//!
//! ```no_run
//! use rullm_anthropic::{Client, Message, MessagesRequest, RequestOptions};
//! use futures::StreamExt;
//! use std::pin::pin;
//!
//! # async fn example() -> Result<(), rullm_anthropic::AnthropicError> {
//! let client = Client::from_env()?;
//! let messages = client.messages();
//!
//! let request = MessagesRequest::builder("claude-3-5-sonnet-20241022", 1024)
//!     .message(Message::user("Tell me a story"))
//!     .build();
//!
//! let stream = messages.stream(request, RequestOptions::default()).await?;
//! let mut text_stream = pin!(stream.text_stream());
//!
//! while let Some(chunk) = text_stream.next().await {
//!     print!("{}", chunk?);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Authentication
//!
//! The client supports two authentication methods:
//!
//! - **API Key**: Set `ANTHROPIC_API_KEY` environment variable
//! - **OAuth Token**: Set `ANTHROPIC_AUTH_TOKEN` environment variable (takes precedence)
//!
//! You can also configure authentication programmatically:
//!
//! ```no_run
//! use rullm_anthropic::Client;
//!
//! # fn example() -> Result<(), rullm_anthropic::AnthropicError> {
//! let client = Client::builder()
//!     .api_key("your-api-key")
//!     .build()?;
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod config;
pub mod error;
pub mod messages;
pub mod transport;

// Re-export main types at crate root for convenience
pub use client::{Client, MessagesClient};
pub use config::{ClientBuilder, ClientConfig, RequestOptions};
pub use error::{AnthropicError, ErrorObject, Result};
pub use messages::{
    // System content
    CacheControl,
    // Content blocks
    ContentBlock,
    ContentBlockParam,
    // Other types
    CountTokensRequest,
    CountTokensResponse,
    // Tools
    CustomTool,
    // Streaming
    Delta,
    DocumentSource,
    ImageSource,
    // Request/Response
    Message,
    MessageContent,
    MessageStream,
    MessagesRequest,
    MessagesRequestBuilder,
    MessagesResponse,
    Metadata,
    Role,
    ServerTool,
    ServiceTier,
    StopReason,
    StreamEvent,
    SystemBlock,
    SystemContent,
    ThinkingConfig,
    Tool,
    ToolChoice,
    ToolResultContent,
    Usage,
};
