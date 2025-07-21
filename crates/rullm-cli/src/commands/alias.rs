use crate::aliases::{AliasResolver, UserAliasConfig};
use crate::args::{Cli, CliConfig};
use crate::constants::ALIASES_CONFIG_FILE;
use crate::output::OutputLevel;
use anyhow::Result;
use clap::{Args, Subcommand};

use std::path::Path;

#[derive(Args)]
pub struct AliasArgs {
    #[command(subcommand)]
    pub action: AliasAction,
}

#[derive(Subcommand)]
pub enum AliasAction {
    /// List all available aliases (both built-in and user-defined)
    List,
    /// Add a new user alias
    Add {
        /// Alias name (e.g., "my-fast-model")
        alias: String,
        /// Target in provider/model format (e.g., "openai/gpt-4")
        target: String,
    },
    /// Remove a user alias
    Remove {
        /// Alias name to remove
        alias: String,
    },
    /// Show detailed information about a specific alias
    Show {
        /// Alias name to show
        alias: String,
    },
}

impl AliasArgs {
    pub async fn run(
        &self,
        output_level: OutputLevel,
        cli_config: &CliConfig,
        _cli: &Cli,
    ) -> Result<()> {
        let alias_config_path = &cli_config.config_base_path.join(ALIASES_CONFIG_FILE);

        match &self.action {
            AliasAction::List => {
                list_aliases(alias_config_path, output_level).await?;
            }
            AliasAction::Add { alias, target } => {
                add_alias(alias_config_path, alias, target, output_level).await?;
            }
            AliasAction::Remove { alias } => {
                remove_alias(alias_config_path, alias, output_level).await?;
            }
            AliasAction::Show { alias } => {
                show_alias(alias_config_path, alias, output_level).await?;
            }
        }

        Ok(())
    }
}

/// Helper function to format alias display
fn format_alias_display(alias: &str, target: &str) -> String {
    format!("  {alias:<20} → {target}")
}

/// Load resolver with user aliases from config
fn load_resolver_with_config(config_path: &Path) -> Result<AliasResolver> {
    let config = UserAliasConfig::load_from_file(config_path)?;
    Ok(AliasResolver::with_user_aliases(config.aliases))
}

/// List all available aliases
async fn list_aliases(config_path: &Path, output_level: OutputLevel) -> Result<()> {
    let resolver = load_resolver_with_config(config_path)?;
    let aliases = resolver.list_aliases();

    if aliases.is_empty() {
        crate::output::note("No aliases configured.", output_level);
        return Ok(());
    }

    crate::output::heading("Available aliases:", output_level);

    // Load config to determine which are user vs built-in
    let config = UserAliasConfig::load_from_file(config_path)?;

    let mut user_aliases = Vec::new();

    for (alias, target) in aliases {
        if config.aliases.contains_key(&alias.to_lowercase()) {
            user_aliases.push((alias, target));
        }
    }

    if !user_aliases.is_empty() {
        for (alias, target) in &user_aliases {
            crate::output::note(&format_alias_display(alias, target), output_level);
        }
    }

    Ok(())
}

/// Add a new user alias
async fn add_alias(
    config_path: &Path,
    alias: &str,
    target: &str,
    output_level: OutputLevel,
) -> Result<()> {
    // Validate alias name
    if alias.trim().is_empty() {
        return Err(anyhow::anyhow!("Alias name cannot be empty"));
    }

    if alias.contains('/') {
        return Err(anyhow::anyhow!("Alias name cannot contain '/' character"));
    }

    // Load existing config and resolver
    let mut config = UserAliasConfig::load_from_file(config_path)?;
    let resolver = load_resolver_with_config(config_path)?;

    // Check if alias already exists in resolver
    let existing_aliases = resolver.list_aliases();
    if let Some((_, existing_target)) = existing_aliases
        .iter()
        .find(|(a, _)| a.to_lowercase() == alias.to_lowercase())
    {
        let is_user = config.aliases.contains_key(&alias.to_lowercase());

        if is_user {
            crate::output::warning(
                &format!("User alias '{alias}' already exists (→ {existing_target})"),
                output_level,
            );
            print!("Do you want to overwrite it? [y/N]: ");
            use std::io::{self, Write};
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                crate::output::note("Cancelled.", output_level);
                return Ok(());
            }
        } else {
            crate::output::warning(
                &format!("This will override the built-in alias '{alias}' (→ {existing_target})"),
                output_level,
            );
        }
    }

    // Add the alias to config and save
    config.add_alias(alias, target)?;
    config.save_to_file(config_path)?;

    crate::output::success(&format!("Added alias: {alias} → {target}"), output_level);
    Ok(())
}

/// Remove a user alias
async fn remove_alias(config_path: &Path, alias: &str, output_level: OutputLevel) -> Result<()> {
    let mut config = UserAliasConfig::load_from_file(config_path)?;
    let removed = config.remove_alias(alias);

    if removed {
        config.save_to_file(config_path)?;
        crate::output::success(&format!("Removed alias: {alias}"), output_level);
    } else {
        crate::output::warning(&format!("Alias '{alias}' not found."), output_level);
    }

    Ok(())
}

/// Show detailed information about a specific alias
async fn show_alias(config_path: &Path, alias: &str, output_level: OutputLevel) -> Result<()> {
    let resolver = load_resolver_with_config(config_path)?;
    let aliases = resolver.list_aliases();
    let config = UserAliasConfig::load_from_file(config_path)?;

    if let Some((found_alias, target)) = aliases
        .iter()
        .find(|(a, _)| a.to_lowercase() == alias.to_lowercase())
    {
        let is_user = config.aliases.contains_key(&found_alias.to_lowercase());

        crate::output::note(&format!("Alias: {found_alias}"), output_level);
        crate::output::note(&format!("Target: {target}"), output_level);
        crate::output::note(
            &format!(
                "Type: {}",
                if is_user { "User-defined" } else { "Built-in" }
            ),
            output_level,
        );

        // Try to resolve it through the resolver to show effective resolution
        match resolver.resolve(alias) {
            Ok((provider, model)) => {
                crate::output::note(&format!("Resolves to: {provider} / {model}"), output_level);
            }
            Err(e) => {
                crate::output::error(&format!("Resolution error: {e}"), output_level);
            }
        }
    } else {
        crate::output::warning(&format!("Alias '{alias}' not found."), output_level);

        // Suggest similar aliases
        let similar: Vec<_> = aliases
            .iter()
            .filter(|(a, _)| {
                let alias_lower = alias.to_lowercase();
                let a_lower = a.to_lowercase();
                a_lower.contains(&alias_lower) || alias_lower.contains(&a_lower)
            })
            .take(5)
            .collect();

        if !similar.is_empty() {
            crate::output::note("\nDid you mean one of these?", output_level);
            for (alias, target) in similar {
                let is_user = config.aliases.contains_key(&alias.to_lowercase());
                let type_str = if is_user { "(user)" } else { "(built-in)" };
                crate::output::note(&format!("  {alias} → {target} {type_str}"), output_level);
            }
        }
    }

    Ok(())
}
