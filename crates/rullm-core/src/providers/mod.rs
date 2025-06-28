// Re-export all providers
pub mod anthropic;
pub mod google;
pub mod openai;

pub use anthropic::AnthropicProvider;
pub use google::GoogleProvider;
pub use openai::OpenAIProvider;

use clap::ValueEnum;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

/// Enum of supported providers usable throughout the library and by downstream
/// crates (e.g. `rullm-cli`).  It is placed in the core crate so both the
/// library and any front-end binaries share a single source of truth.
#[derive(Clone, Debug, PartialEq, Eq, ValueEnum, EnumIter)]
pub enum Provider {
    #[value(name = "openai")]
    OpenAI,
    #[value(name = "anthropic")]
    Anthropic,
    #[value(name = "google")]
    Google,
}

/// Extension trait providing metadata & helper utilities for [`Provider`].
pub trait ProviderExt {
    /// Environment variable expected to contain the provider API key.
    fn env_key(&self) -> &'static str;

    /// Canonical provider identifier (lower-case, e.g. "openai").
    fn name(&self) -> &'static str;

    /// Alternative identifiers that should map to this provider (e.g. "gpt", "gemini").
    fn aliases(&self) -> &'static [&'static str];

    /// Default base URL used when no explicit endpoint is supplied.
    fn default_base_url(&self) -> &'static str;
}

impl ProviderExt for Provider {
    fn env_key(&self) -> &'static str {
        match self {
            Provider::OpenAI => "OPENAI_API_KEY",
            Provider::Anthropic => "ANTHROPIC_API_KEY",
            Provider::Google => "GOOGLE_AI_API_KEY",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Provider::OpenAI => "openai",
            Provider::Anthropic => "anthropic",
            Provider::Google => "google",
        }
    }

    fn aliases(&self) -> &'static [&'static str] {
        match self {
            Provider::OpenAI => &["openai", "gpt"],
            Provider::Anthropic => &["anthropic", "claude"],
            Provider::Google => &["google", "gemini"],
        }
    }

    fn default_base_url(&self) -> &'static str {
        match self {
            Provider::OpenAI => "https://api.openai.com/v1",
            Provider::Anthropic => "https://api.anthropic.com/v1",
            Provider::Google => "https://generativelanguage.googleapis.com/v1beta",
        }
    }
}

impl Provider {
    /// Attempt to resolve a provider from a string identifier (case-insensitive) using
    /// [`ProviderExt::aliases`]. Returns `None` if no match is found.
    pub fn from_alias(alias: &str) -> Option<Provider> {
        let candidate = alias.to_ascii_lowercase();
        Provider::iter().find(|p| p.aliases().contains(&candidate.as_str()))
    }

    /// Parse a model identifier in explicit provider/model format and return the provider
    /// along with the model name.
    ///
    /// This method only handles explicit provider prefixes: "openai/gpt-4" → (OpenAI, "gpt-4")
    /// For alias resolution and pattern inference, use the CLI layer's alias resolver.
    ///
    /// Returns an error if the input is not in valid provider/model format.
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

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
