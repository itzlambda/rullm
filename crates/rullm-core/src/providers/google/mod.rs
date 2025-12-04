//! Google Gemini provider implementation with complete API support
//!
//! This module provides a feature-complete Google Gemini client that supports all
//! parameters and features available in the Google Gemini API.
//!
//! # Example
//!
//! ```no_run
//! use rullm_core::providers::google::{GoogleClient, GenerateContentRequest, Content};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = GoogleClient::from_env()?;
//!
//! let request = GenerateContentRequest::new(vec![
//!     Content::user("Hello!"),
//! ]);
//!
//! let response = client.generate_content("gemini-pro", request).await?;
//! println!("{:?}", response.candidates);
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod types;

pub use client::GoogleClient;
pub use types::*;
