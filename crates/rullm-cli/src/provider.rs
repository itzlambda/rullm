use clap::{ValueEnum, builder::PossibleValue};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Clone, Debug, PartialEq, Eq, EnumIter)]
pub enum Provider {
    OpenAI,
    Groq,
    OpenRouter,
    Anthropic,
    Google,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Provider::OpenAI => "openai",
            Provider::Groq => "groq",
            Provider::OpenRouter => "openrouter",
            Provider::Anthropic => "anthropic",
            Provider::Google => "google",
        };
        write!(f, "{name}")
    }
}

impl ValueEnum for Provider {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::OpenAI, Self::Groq, Self::OpenRouter, Self::Anthropic, Self::Google]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        let value = match self {
            Self::OpenAI => PossibleValue::new("openai"),
            Self::Groq => PossibleValue::new("groq"),
            Self::OpenRouter => PossibleValue::new("openrouter"),
            Self::Anthropic => PossibleValue::new("anthropic"),
            Self::Google => PossibleValue::new("google"),
        };
        Some(value)
    }
}

impl Provider {
    pub fn aliases(&self) -> &'static [&'static str] {
        match self {
            Provider::OpenAI => &["openai", "gpt"],
            Provider::Groq => &["groq"],
            Provider::OpenRouter => &["openrouter"],
            Provider::Anthropic => &["anthropic", "claude"],
            Provider::Google => &["google", "gemini"],
        }
    }

    pub fn from_alias(alias: &str) -> Option<Provider> {
        let candidate = alias.to_ascii_lowercase();
        Provider::iter().find(|p| p.aliases().contains(&candidate.as_str()))
    }

    #[allow(dead_code)]
    pub fn from_model(input: &str) -> Result<(Provider, String), anyhow::Error> {
        if let Some((provider_str, model)) = input.split_once(':') {
            let provider = Provider::from_alias(provider_str)
                .ok_or_else(|| anyhow::anyhow!(format!("Unsupported provider: {provider_str}")))?;

            if model.is_empty() {
                return Err(anyhow::anyhow!("Model name cannot be empty"));
            }

            Ok((provider, model.to_string()))
        } else {
            Err(anyhow::anyhow!(format!(
                "Invalid model format '{input}'. Expected 'provider:model'"
            )))
        }
    }

    pub fn env_key(&self) -> &'static str {
        match self {
            Provider::OpenAI => "OPENAI_API_KEY",
            Provider::Groq => "GROQ_API_KEY",
            Provider::OpenRouter => "OPENROUTER_API_KEY",
            Provider::Anthropic => "ANTHROPIC_API_KEY",
            Provider::Google => "GOOGLE_AI_API_KEY",
        }
    }
}
