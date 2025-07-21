use anyhow::Result;
use chrono::Utc;
use clap::{Args, Subcommand};
use rullm_core::{LlmError, SimpleLlm, SimpleLlmClient};
use strum::IntoEnumIterator;

use crate::{
    aliases::UserAliasConfig,
    args::{Cli, CliConfig},
    client,
    commands::{ModelsCache, format_duration},
    constants::{ALIASES_CONFIG_FILE, MODEL_FILE_NAME},
    output::OutputLevel,
    provider::Provider,
};

#[derive(Args)]
pub struct ModelsArgs {
    #[command(subcommand)]
    pub action: ModelsAction,
}

#[derive(Subcommand)]
pub enum ModelsAction {
    /// List available models for the current provider (default)
    List,
    /// Set a default model that will be used when --model is not supplied
    Default {
        /// Model identifier in the form provider/model-name (e.g. openai/gpt-4o)
        model: Option<String>,
    },
    /// Fetch fresh models from all providers with available API keys and update local cache
    Update,
    /// Clear the local models cache
    Clear,
}

impl ModelsArgs {
    pub async fn run(
        &self,
        output_level: OutputLevel,
        cli_config: &mut CliConfig,
        cli: &Cli,
    ) -> Result<()> {
        match &self.action {
            ModelsAction::List => {
                show_cached_models(cli_config, output_level)?;
            }
            ModelsAction::Default { model } => {
                match model {
                    Some(model) => {
                        set_default_model(cli_config, model.as_str(), output_level).await?;
                    }
                    None => {
                        // Print default model
                        crate::output::note(
                            &format!(
                                "Default model: {}",
                                crate::output::format_model(
                                    cli_config.config.default_model.as_ref().unwrap()
                                )
                            ),
                            output_level,
                        );
                    }
                }
            }
            ModelsAction::Update => {
                // List of supported providers
                let providers = Provider::iter();
                let mut updated = vec![];
                let mut skipped = vec![];

                for provider in providers {
                    let provider = format!("{provider}");
                    // Try to create a client for this provider
                    let model_hint = format!("{provider}/dummy"); // dummy model name, just to get the client
                    let client = match client::from_model(&model_hint, cli, cli_config) {
                        Ok(c) => c,
                        Err(_) => {
                            skipped.push(provider);
                            continue;
                        }
                    };
                    match update_models(cli_config, &client, output_level).await {
                        Ok(_) => updated.push(provider),
                        Err(_) => skipped.push(provider),
                    }
                }

                if !skipped.is_empty() {
                    crate::output::note(
                        &format!("Skipped (no API key or error): {}", skipped.join(", ")),
                        output_level,
                    );
                }
            }
            ModelsAction::Clear => {
                clear_models_cache(cli_config, output_level)?;
            }
        }

        Ok(())
    }
}

pub fn show_cached_models(cli_config: &CliConfig, output_level: OutputLevel) -> Result<()> {
    let entries = &cli_config.models.models;

    if entries.is_empty() {
        crate::output::error_with_suggestion(
            "No cached models found",
            &format!(
                "Run {} to fetch available models",
                crate::output::format_command(&format!(
                    "{} models update",
                    crate::constants::BINARY_NAME
                ))
            ),
            output_level,
        );
        return Ok(());
    }

    // Check if cache is stale (older than 24 hours)
    if let Ok(Some(cache)) = load_models_cache(cli_config) {
        let now = Utc::now();
        let cache_age = now.signed_duration_since(cache.last_updated);

        if cache_age.num_hours() > 24 {
            crate::output::error_with_suggestion(
                &format!("Model cache is {} old", format_duration(cache_age)),
                &format!(
                    "Run {} to refresh the cache",
                    crate::output::format_command(&format!(
                        "{} models update",
                        crate::constants::BINARY_NAME
                    ))
                ),
                output_level,
            );
        }
    }

    let alias_config_path = &cli_config.config_base_path.join(ALIASES_CONFIG_FILE);
    let aliases = UserAliasConfig::load_from_file(alias_config_path)?;

    for m in entries.iter() {
        let model_aliases = aliases
            .aliases
            .iter()
            .filter(|(_, v)| v.starts_with(m))
            .map(|(k, _)| k.clone())
            .collect::<Vec<_>>();

        let message = if model_aliases.is_empty() {
            format!("{}", crate::output::format_model(m))
        } else {
            format!(
                "{}: (aliases: {})",
                crate::output::format_model(m),
                model_aliases.join(", ")
            )
        };

        crate::output::note(&message, output_level);
    }

    Ok(())
}

pub async fn set_default_model(
    cli_config: &mut CliConfig,
    model: &str,
    output_level: OutputLevel,
) -> Result<()> {
    let models_cache = load_models_cache(cli_config)?.unwrap_or(ModelsCache::new(vec![]));

    if models_cache.models.contains(&model.to_string()) {
        cli_config.config.default_model = Some(model.to_string());
        cli_config.config.save(&cli_config.config_base_path)?;

        crate::output::success(
            &format!(
                "Default model set to {}",
                crate::output::format_model(model)
            ),
            output_level,
        );
    } else {
        crate::output::error_with_suggestion(
            &format!("Model \"{model}\" not found in cache"),
            &format!(
                "Try running \"{} models update\" to update the cache",
                crate::constants::BINARY_NAME
            ),
            output_level,
        );
    }
    Ok(())
}

pub fn clear_models_cache(cli_config: &CliConfig, output_level: OutputLevel) -> Result<()> {
    use std::fs;

    let path = cli_config.data_base_path.join(MODEL_FILE_NAME);

    if path.exists() {
        fs::remove_file(&path)?;
        crate::output::success("Models cache cleared successfully.", output_level);
    } else {
        crate::output::note("No models cache found to clear.", output_level);
    }

    Ok(())
}

pub async fn update_models(
    cli_config: &mut CliConfig,
    client: &SimpleLlmClient,
    output_level: OutputLevel,
) -> Result<(), LlmError> {
    crate::output::progress(
        &format!(
            "Fetching models from {}...",
            crate::output::format_provider(client.provider_name())
        ),
        output_level,
    );

    match client.models().await {
        Ok(models) => {
            crate::output::progress(
                &format!("Fetched {} models. Caching", models.len()),
                output_level,
            );
            if let Err(e) = cache_models(cli_config, client.provider_name(), &models) {
                crate::output::error(&format!("Failed to cache: {e}"), output_level);
            }
        }
        Err(e) => {
            crate::output::error(&format!("Failed to fetch models: {e}"), output_level);
            return Err(e);
        }
    }

    Ok(())
}

fn cache_models(cli_config: &CliConfig, provider_name: &str, models: &[String]) -> Result<()> {
    use std::fs;

    let path = cli_config.data_base_path.join(MODEL_FILE_NAME);
    // TODO: we shouldn't need to do this here, this should be done while cli_config is created
    // TODO: Remove if we already do this.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Load existing cache if present
    let mut entries = if let Ok(Some(cache)) = load_models_cache(cli_config) {
        cache.models
    } else {
        Vec::new()
    };

    // Remove all entries for this provider
    let prefix = format!("{}/", provider_name.to_lowercase());
    entries.retain(|m| !m.starts_with(&prefix));

    // Add new models for this provider
    let new_entries: Vec<String> = models
        .iter()
        .map(|m| format!("{}/{}", provider_name.to_lowercase(), m))
        .collect();
    entries.extend(new_entries);

    let cache = ModelsCache::new(entries);
    let json = serde_json::to_string_pretty(&cache)?;
    fs::write(path, json)?;
    Ok(())
}

pub(crate) fn load_models_cache(cli_config: &CliConfig) -> Result<Option<ModelsCache>> {
    use std::fs;

    let path = cli_config.data_base_path.join(MODEL_FILE_NAME);

    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;

    // Try to parse as new format
    if let Ok(cache) = serde_json::from_str::<ModelsCache>(&content) {
        return Ok(Some(cache));
    }

    // Old format doesn't have timestamp info
    Ok(None)
}
