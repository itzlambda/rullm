//! OAuth authentication module for rullm.
//!
//! Provides OAuth 2.0 authentication flows for supported providers.

mod pkce;
mod server;

pub mod anthropic;
pub mod openai;

pub use pkce::PkceChallenge;
pub use server::{CallbackResult, CallbackServer};
