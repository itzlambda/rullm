//! Anthropic provider implementation with complete Messages API support
//!
//! This module provides a feature-complete Anthropic client that supports all
//! parameters and features available in the Anthropic Messages API.
//!
//! # Example
//!
//! ```no_run
//! use rullm_core::providers::anthropic::{AnthropicClient, MessagesRequest, Message};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = AnthropicClient::from_env()?;
//!
//! let request = MessagesRequest::new(
//!     "claude-3-opus-20240229",
//!     vec![Message::user("Hello!")],
//!     1024,
//! );
//!
//! let response = client.messages(request).await?;
//! println!("{:?}", response.content);
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod types;

pub use client::AnthropicClient;
pub use types::*;
