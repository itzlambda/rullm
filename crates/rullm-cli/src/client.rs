use super::provider::Provider;
use crate::api_keys::ApiKeys;
use crate::args::{Cli, CliConfig};
use crate::cli_client::{CliClient, CliConfig as CoreCliConfig};
use crate::constants;
use anyhow::{Context, Result};

use rullm_core::LlmError;

pub fn create_client(
    provider: &Provider,
    api_key: &str,
    _base_url: Option<&str>,
    cli: &Cli,
    model_name: &str,
) -> Result<CliClient, LlmError> {
    // Build CoreCliConfig based on CLI args
    let mut config = CoreCliConfig::default();

    // Parse options from --option key value format
    for (key, value) in &cli.option {
        match key.as_str() {
            "temperature" => {
                if let Ok(temp) = value.parse::<f32>() {
                    config.temperature = Some(temp);
                }
            }
            "max_tokens" => {
                if let Ok(max_tokens) = value.parse::<u32>() {
                    config.max_tokens = Some(max_tokens);
                }
            }
            _ => {
                // Ignore unknown options for now
            }
        }
    }

    match provider {
        Provider::OpenAI => CliClient::openai(api_key, model_name, config),
        Provider::Groq => CliClient::groq(api_key, model_name, config),
        Provider::OpenRouter => CliClient::openrouter(api_key, model_name, config),
        Provider::Anthropic => CliClient::anthropic(api_key, model_name, config),
        Provider::Google => CliClient::google(api_key, model_name, config),
    }
}

/// Create a CliClient from a model string, CLI arguments, and configuration
/// This is the promoted version of the create_client_from_model closure from lib.rs
pub fn from_model(model_str: &str, cli: &Cli, cli_config: &CliConfig) -> Result<CliClient> {
    // Use the global alias resolver for CLI functionality
    let resolver = crate::aliases::get_global_alias_resolver(&cli_config.config_base_path);
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
