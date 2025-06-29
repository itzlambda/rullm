use super::provider::Provider;
use crate::api_keys::ApiKeys;
use crate::args::{Cli, CliConfig};
use crate::constants;
use anyhow::{Context, Result};

use rullm_core::simple::{SimpleLlmBuilder, SimpleLlmClient, SimpleLlmConfig};
use rullm_core::{AnthropicConfig, GoogleAiConfig, LlmError, OpenAIConfig};

/// Generic helper to create provider configs with API key and optional base URL
trait ProviderConfigBuilder<T> {
    fn new_config(api_key: String) -> T;
    fn with_base_url(config: T, base_url: &str) -> T;
    fn build_client(
        builder: SimpleLlmBuilder,
        config: T,
        simple_config: SimpleLlmConfig,
    ) -> Result<SimpleLlmClient, LlmError>;
}

struct OpenAIConfigBuilder;
impl ProviderConfigBuilder<OpenAIConfig> for OpenAIConfigBuilder {
    fn new_config(api_key: String) -> OpenAIConfig {
        OpenAIConfig::new(api_key)
    }

    fn with_base_url(config: OpenAIConfig, base_url: &str) -> OpenAIConfig {
        config.with_base_url(base_url)
    }

    fn build_client(
        builder: SimpleLlmBuilder,
        config: OpenAIConfig,
        simple_config: SimpleLlmConfig,
    ) -> Result<SimpleLlmClient, LlmError> {
        builder
            .with_openai(config)
            .with_simple_config(simple_config)
            .build_openai()
    }
}

struct AnthropicConfigBuilder;
impl ProviderConfigBuilder<AnthropicConfig> for AnthropicConfigBuilder {
    fn new_config(api_key: String) -> AnthropicConfig {
        AnthropicConfig::new(api_key)
    }

    fn with_base_url(config: AnthropicConfig, base_url: &str) -> AnthropicConfig {
        config.with_base_url(base_url)
    }

    fn build_client(
        builder: SimpleLlmBuilder,
        config: AnthropicConfig,
        simple_config: SimpleLlmConfig,
    ) -> Result<SimpleLlmClient, LlmError> {
        builder
            .with_anthropic(config)
            .with_simple_config(simple_config)
            .build_anthropic()
    }
}

struct GoogleConfigBuilder;
impl ProviderConfigBuilder<GoogleAiConfig> for GoogleConfigBuilder {
    fn new_config(api_key: String) -> GoogleAiConfig {
        GoogleAiConfig::new(api_key)
    }

    fn with_base_url(config: GoogleAiConfig, base_url: &str) -> GoogleAiConfig {
        config.with_base_url(base_url)
    }

    fn build_client(
        builder: SimpleLlmBuilder,
        config: GoogleAiConfig,
        simple_config: SimpleLlmConfig,
    ) -> Result<SimpleLlmClient, LlmError> {
        builder
            .with_google(config)
            .with_simple_config(simple_config)
            .build_google()
    }
}

/// Generic function to create provider client, eliminating duplication
fn create_provider_client<T, B>(
    api_key: &str,
    base_url: Option<&str>,
    simple_config: SimpleLlmConfig,
) -> Result<SimpleLlmClient, LlmError>
where
    B: ProviderConfigBuilder<T>,
{
    let mut config = B::new_config(api_key.to_string());
    if let Some(url) = base_url {
        config = B::with_base_url(config, url);
    }
    B::build_client(SimpleLlmBuilder::new(), config, simple_config)
}

pub fn create_client(
    provider: &Provider,
    api_key: &str,
    base_url: Option<&str>,
    cli: &Cli,
    model_name: &str,
) -> Result<SimpleLlmClient, LlmError> {
    // Build custom SimpleLlmConfig based on CLI args
    let mut simple_config = SimpleLlmConfig::new();

    // Parse options from --option key value format
    for (key, value) in &cli.option {
        match key.as_str() {
            "temperature" => {
                if let Ok(temp) = value.parse::<f32>() {
                    simple_config = simple_config.with_temperature(temp);
                }
            }
            "max_tokens" => {
                if let Ok(max_tokens) = value.parse::<u32>() {
                    simple_config = simple_config.with_max_tokens(max_tokens);
                }
            }
            _ => {
                // Ignore unknown options for now
            }
        }
    }

    // Set custom model if specified
    simple_config = match provider {
        Provider::OpenAI => simple_config.with_openai_model(model_name),
        Provider::Anthropic => simple_config.with_anthropic_model(model_name),
        Provider::Google => simple_config.with_google_model(model_name),
    };

    match provider {
        Provider::OpenAI => create_provider_client::<OpenAIConfig, OpenAIConfigBuilder>(
            api_key,
            base_url,
            simple_config,
        ),
        Provider::Anthropic => create_provider_client::<AnthropicConfig, AnthropicConfigBuilder>(
            api_key,
            base_url,
            simple_config,
        ),
        Provider::Google => create_provider_client::<GoogleAiConfig, GoogleConfigBuilder>(
            api_key,
            base_url,
            simple_config,
        ),
    }
}

/// Create a SimpleLlmClient from a model string, CLI arguments, and configuration
/// This is the promoted version of the create_client_from_model closure from lib.rs
pub fn from_model(model_str: &str, cli: &Cli, cli_config: &CliConfig) -> Result<SimpleLlmClient> {
    // Use the global alias resolver for CLI functionality
    let resolver = crate::aliases::get_global_alias_resolver();
    let resolver = resolver
        .read()
        .map_err(|_| anyhow::anyhow!("Failed to acquire read lock on global resolver"))?;
    let (provider, model_name) = resolver
        .resolve(model_str)
        .context("Invalid model format")?;

    let api_key = ApiKeys::get_api_key(&provider, &cli_config.api_keys).ok_or_else(|| {
        anyhow::anyhow!(
            "API key required. Set {} environment variable or add it to {} in config directory",
            provider.env_key(),
            constants::CONFIG_FILE_NAME
        )
    })?;

    create_client(&provider, &api_key, None, cli, &model_name).map_err(anyhow::Error::from)
}
