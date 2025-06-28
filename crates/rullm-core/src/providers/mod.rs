// Re-export all providers
pub mod anthropic;
pub mod google;
pub mod openai;

pub use anthropic::AnthropicProvider;
pub use google::GoogleProvider;
pub use openai::OpenAIProvider;

use clap::{ValueEnum, builder::PossibleValue};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

/// Enum of supported providers usable throughout the library and by downstream
/// crates (e.g. `rullm-cli`).  It is placed in the core crate so both the
/// library and any front-end binaries share a single source of truth.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter)]
pub enum Provider {
    OpenAI,
    Anthropic,
    Google,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Provider::OpenAI => "openai",
            Provider::Anthropic => "anthropic",
            Provider::Google => "google",
        };
        write!(f, "{name}")
    }
}

impl ValueEnum for Provider {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::OpenAI, Self::Anthropic, Self::Google]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        let value = match self {
            Self::OpenAI => PossibleValue::new("openai"),
            Self::Anthropic => PossibleValue::new("anthropic"),
            Self::Google => PossibleValue::new("google"),
        };
        Some(value)
    }
}

impl Provider {
    // this part should be moved to cli
    pub fn aliases(&self) -> &'static [&'static str] {
        match self {
            Provider::OpenAI => &["openai", "gpt"],
            Provider::Anthropic => &["anthropic", "claude"],
            Provider::Google => &["google", "gemini"],
        }
    }

    // this part should be moved to cli
    /// Attempt to resolve a provider from a string identifier (case-insensitive).
    pub fn from_alias(alias: &str) -> Option<Provider> {
        let candidate = alias.to_ascii_lowercase();
        Provider::iter().find(|p| p.aliases().contains(&candidate.as_str()))
    }

    // this part should be moved to cli
    /// Parse a model identifier in explicit provider/model format and return the provider
    /// along with the model name (e.g. "openai/gpt-4" → (OpenAI, "gpt-4")).
    pub fn from_model(input: &str) -> Result<(Provider, String), crate::error::LlmError> {
        if let Some((provider_str, model)) = input.split_once('/') {
            let provider = Provider::from_alias(provider_str).ok_or_else(|| {
                crate::error::LlmError::validation(format!("Unsupported provider: {provider_str}"))
            })?;

            if model.is_empty() {
                return Err(crate::error::LlmError::validation(
                    "Model name cannot be empty".to_string(),
                ));
            }

            Ok((provider, model.to_string()))
        } else {
            Err(crate::error::LlmError::validation(format!(
                "Invalid model format '{input}'. Expected 'provider/model'"
            )))
        }
    }
}
