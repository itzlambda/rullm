use std::path::PathBuf;

use anyhow::Result;
use chrono::Utc;
use clap::{Args, Subcommand};
use clap_complete::engine::ArgValueCompleter;
use etcetera::BaseStrategy;
use rullm_core::{LlmError, SimpleLlm, SimpleLlmClient};

use crate::{
    args::{Cli, CliConfig, model_completer},
    cli_helpers::resolve_model,
    client,
    commands::{ModelsCache, format_duration},
    constants::MODEL_FILE_NAME,
    output::OutputLevel,
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
    /// Fetch fresh models from provider and update local cache
    Update {
        /// Model to use in format: provider/model-name (e.g., openai/gpt-4, gemini/gemini-pro, anthropic/claude-3-sonnet)
        #[arg(short, long, add = ArgValueCompleter::new(model_completer))]
        model: Option<String>,
    },
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
            ModelsAction::Update { model } => {
                let model_str = resolve_model(&cli.model, model, &cli_config.config.default_model)?;
                let client = client::from_model(&model_str, cli, cli_config)?;
                update_models(&client, output_level)
                    .await
                    .map_err(anyhow::Error::from)?;
            }
            ModelsAction::Clear => {
                clear_models_cache(output_level)?;
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
    if let Ok(Some(cache)) = load_models_cache() {
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

    for m in entries.iter() {
        crate::output::note(&crate::output::format_model(m), output_level);
    }

    Ok(())
}

pub async fn set_default_model(
    cli_config: &mut CliConfig,
    model: &str,
    output_level: OutputLevel,
) -> Result<()> {
    cli_config.config.default_model = Some(model.to_string());
    cli_config.config.save(&cli_config.config_base_path)?;

    crate::output::success(
        &format!(
            "Default model set to {}",
            crate::output::format_model(model)
        ),
        output_level,
    );
    Ok(())
}

pub fn clear_models_cache(output_level: OutputLevel) -> Result<()> {
    use std::fs;

    let path = get_models_cache_path()?;

    if path.exists() {
        fs::remove_file(&path)?;
        crate::output::success("Models cache cleared successfully.", output_level);
    } else {
        crate::output::note("No models cache found to clear.", output_level);
    }

    Ok(())
}

pub async fn update_models(
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
            if let Err(e) = cache_models(client.provider_name(), &models) {
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

fn cache_models(provider_name: &str, models: &[String]) -> Result<()> {
    use std::fs;

    let path = get_models_cache_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Prepend provider name to model list as "provider/model"
    let entries: Vec<String> = models
        .iter()
        .map(|m| format!("{}/{}", provider_name.to_lowercase(), m))
        .collect();

    let cache = ModelsCache::new(entries);
    let json = serde_json::to_string_pretty(&cache)?;
    fs::write(path, json)?;
    Ok(())
}

/// Helper function to get models cache path, eliminating duplication
fn get_models_cache_path() -> Result<PathBuf> {
    use crate::constants::BINARY_NAME;
    use etcetera::choose_base_strategy;

    let strategy = choose_base_strategy()?;
    Ok(strategy.data_dir().join(BINARY_NAME).join(MODEL_FILE_NAME))
}

pub(crate) fn load_models_cache() -> Result<Option<ModelsCache>> {
    use std::fs;

    let path = get_models_cache_path()?;

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

pub fn load_cached_models() -> Result<Vec<String>> {
    use std::fs;

    let path = get_models_cache_path()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)?;

    // Try to parse as new format first
    if let Ok(cache) = serde_json::from_str::<ModelsCache>(&content) {
        return Ok(cache.models);
    }

    // Fallback to old format (simple array)
    let entries: Vec<String> = serde_json::from_str(&content)?;
    Ok(entries)
}
