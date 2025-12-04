//! OpenAI provider implementation with complete API support
//!
//! This module provides a feature-complete OpenAI client that supports all
//! parameters and features available in the OpenAI Chat Completions API.
//!
//! # Example
//!
//! ```no_run
//! use rullm_core::providers::openai::{OpenAIClient, ChatCompletionRequest, ChatMessage};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OpenAIClient::from_env()?;
//!
//! let request = ChatCompletionRequest::new(
//!     "gpt-4",
//!     vec![
//!         ChatMessage::system("You are a helpful assistant"),
//!         ChatMessage::user("Hello!"),
//!     ],
//! );
//!
//! let response = client.chat_completion(request).await?;
//! println!("{}", response.choices[0].message.content.as_ref().unwrap());
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod types;

pub use client::OpenAIClient;
pub use types::*;
