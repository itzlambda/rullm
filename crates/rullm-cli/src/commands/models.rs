use anyhow::Result;
use chrono::Utc;
use clap::{Args, Subcommand};
use serde::Deserialize;
use std::collections::HashMap;
use strum::IntoEnumIterator;

use crate::{
    aliases::UserAliasConfig,
    args::{Cli, CliConfig},
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
    /// List cached models
    List,
    /// Set a default model that will be used when --model is not supplied
    Default {
        /// Model identifier in the form provider:model-name (e.g. openai:gpt-4o)
        model: Option<String>,
    },
    /// Fetch fresh models from models.dev and update local cache
    Update,
    /// Clear the local models cache
    Clear,
}

impl ModelsArgs {
    pub async fn run(
        &self,
        output_level: OutputLevel,
        cli_config: &mut CliConfig,
        _cli: &Cli,
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
                let supported: Vec<&str> = Provider::iter().map(|p| p.models_dev_id()).collect();

                crate::output::progress("Fetching models from models.dev...", output_level);

                let models = fetch_models_from_models_dev(&supported).await?;
                if models.is_empty() {
                    anyhow::bail!("No models returned by models.dev");
                }

                let cache = ModelsCache::new(models);
                let path = cli_config.data_base_path.join(MODEL_FILE_NAME);

                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                std::fs::write(&path, serde_json::to_string_pretty(&cache)?)?;

                crate::output::success(
                    &format!("Updated {} models", cache.models.len()),
                    output_level,
                );
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
            crate::output::format_model(m)
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

#[derive(Deserialize)]
struct ModelsDevProvider {
    models: HashMap<String, ModelsDevModel>,
}

#[derive(Deserialize)]
struct ModelsDevModel {
    #[serde(default)]
    id: Option<String>,
}

async fn fetch_models_from_models_dev(supported_providers: &[&str]) -> Result<Vec<String>> {
    let response = reqwest::get("https://models.dev/api.json")
        .await?
        .error_for_status()?;
    let providers: HashMap<String, ModelsDevProvider> = response.json().await?;

    let mut all_models = Vec::new();
    for provider_id in supported_providers {
        if let Some(provider) = providers.get(*provider_id) {
            for (model_id, model) in &provider.models {
                let id = model.id.as_deref().unwrap_or(model_id);
                all_models.push(format!("{provider_id}:{id}"));
            }
        }
    }

    all_models.sort();
    all_models.dedup();
    Ok(all_models)
}
