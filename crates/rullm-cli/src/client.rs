use super::provider::Provider;
use crate::args::{Cli, CliConfig};
use crate::auth;
use crate::cli_client::{CliClient, CliConfig as CoreCliConfig};
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

/// Create a CliClient from a model string, CLI arguments, and configuration.
///
/// This function handles OAuth token refresh automatically if the token is expired.
/// The `cli_config` is mutable because refreshing a token requires saving the new credential.
pub async fn from_model(
    model_str: &str,
    cli: &Cli,
    cli_config: &mut CliConfig,
) -> Result<CliClient> {
    // Use the global alias resolver for CLI functionality
    // Resolve provider and model inside a block so the lock is dropped before the await
    let (provider, model_name) = {
        let resolver = crate::aliases::get_global_alias_resolver(&cli_config.config_base_path);
        let resolver = resolver
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to acquire read lock on global resolver"))?;
        resolver
            .resolve(model_str)
            .context("Invalid model format")?
    };

    // Get token with automatic refresh for OAuth
    let token = auth::get_or_refresh_token(
        &provider,
        &mut cli_config.auth_config,
        &cli_config.config_base_path,
    )
    .await
    .map_err(|e| {
        anyhow::anyhow!(
            "{}. Run 'rullm auth login {}' or set {} environment variable",
            e,
            provider,
            provider.env_key()
        )
    })?;

    create_client(&provider, &token, None, cli, &model_name).map_err(anyhow::Error::from)
}
