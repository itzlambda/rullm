use crate::constants::ALIASES_CONFIG_FILE;

use super::provider::Provider;
use rullm_core::error::LlmError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{OnceLock, RwLock};

/// User alias configuration that can be saved/loaded from file
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserAliasConfig {
    /// User-defined aliases mapping alias -> provider:model
    pub aliases: HashMap<String, String>,
}

impl UserAliasConfig {
    /// Load user aliases from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, LlmError> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| LlmError::validation(format!("Failed to read alias config: {e}")))?;

        // Handle empty files gracefully
        if content.trim().is_empty() {
            return Ok(Self::default());
        }

        toml::from_str(&content)
            .map_err(|e| LlmError::validation(format!("Failed to parse alias config: {e}")))
    }

    /// Save user aliases to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), LlmError> {
        let path = path.as_ref();

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                LlmError::validation(format!("Failed to create config directory: {e}"))
            })?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| LlmError::validation(format!("Failed to serialize alias config: {e}")))?;

        // Ensure we always have the [aliases] section even if empty
        let content = if self.aliases.is_empty() {
            "[aliases]\n".to_string()
        } else {
            content
        };

        std::fs::write(path, content)
            .map_err(|e| LlmError::validation(format!("Failed to write alias config: {e}")))
    }

    /// Add a new alias
    pub fn add_alias(&mut self, alias: &str, target: &str) -> Result<(), LlmError> {
        // Validate the target format
        Self::validate_target(target)?;

        self.aliases
            .insert(alias.to_lowercase(), target.to_string());
        Ok(())
    }

    /// Remove an alias
    pub fn remove_alias(&mut self, alias: &str) -> bool {
        self.aliases.remove(&alias.to_lowercase()).is_some()
    }

    /// Validate that a target is in valid provider:model format
    fn validate_target(target: &str) -> Result<(), LlmError> {
        if let Some((provider_str, model_name)) = target.split_once(':') {
            if Provider::from_alias(provider_str).is_none() {
                return Err(LlmError::validation(format!(
                    "Invalid provider '{provider_str}' in target '{target}'"
                )));
            }
            if model_name.trim().is_empty() {
                return Err(LlmError::validation(format!(
                    "Model name cannot be empty in target '{target}'"
                )));
            }
            Ok(())
        } else {
            Err(LlmError::validation(format!(
                "Target '{target}' must be in 'provider:model' format"
            )))
        }
    }
}

/// Unified alias resolver that handles both default (built-in) and user-defined aliases.
/// All aliases resolve to canonical `provider:model` format.
#[derive(Debug, Clone)]
pub struct AliasResolver {
    /// User-defined aliases that override defaults
    user_alias: UserAliasConfig,
    /// Whether to enable case-insensitive matching
    case_insensitive: bool,
}

impl AliasResolver {
    /// Create a new alias resolver with default configuration
    pub fn new(config_path: &Path) -> Self {
        let user_alias = UserAliasConfig::load_from_file(config_path).unwrap_or_default();
        Self {
            user_alias,
            case_insensitive: true,
        }
    }

    /// List all available aliases (both default and user)
    pub fn list_aliases(&self) -> Vec<(String, String)> {
        let mut aliases = Vec::new();

        // Add user aliases (which can override defaults)
        for (alias, target) in &self.user_alias.aliases {
            aliases.push((alias.clone(), target.clone()));
        }

        aliases.sort_by(|a, b| a.0.cmp(&b.0));
        aliases
    }

    /// Resolve an input string to canonical (Provider, model) format
    ///
    /// Resolution order:
    /// 1. If already in provider:model format → validate and use
    /// 2. Check user aliases (user overrides defaults)
    /// 3. Check default aliases
    /// 4. Try pattern inference
    /// 5. Error if unresolvable
    pub fn resolve(&self, input: &str) -> Result<(Provider, String), LlmError> {
        if input.trim().is_empty() {
            return Err(LlmError::validation("Input cannot be empty".to_string()));
        }

        let normalized_input = if self.case_insensitive {
            input.to_lowercase()
        } else {
            input.to_string()
        };

        // 1. Fast path: already in provider:model format
        if let Some((provider_str, model_name)) = input.split_once(':') {
            if let Some(provider) = Provider::from_alias(provider_str) {
                return Ok((provider, model_name.to_string()));
            } else {
                return Err(LlmError::validation(format!(
                    "Unknown provider prefix: '{provider_str}'"
                )));
            }
        }

        // 2. Check user aliases first (user overrides defaults)
        if let Some(target) = self.user_alias.aliases.get(&normalized_input) {
            return self.parse_target(target);
        }

        // 3. Try pattern inference (fallback to existing logic)
        self.infer_from_pattern(input)
    }

    /// Parse a target string to (Provider, model)
    fn parse_target(&self, target: &str) -> Result<(Provider, String), LlmError> {
        if let Some((provider_str, model_name)) = target.split_once(':') {
            if let Some(provider) = Provider::from_alias(provider_str) {
                Ok((provider, model_name.to_string()))
            } else {
                Err(LlmError::validation(format!(
                    "Unknown provider '{provider_str}' in target '{target}'"
                )))
            }
        } else {
            Err(LlmError::validation(format!(
                "Invalid target format '{target}', expected 'provider:model'"
            )))
        }
    }

    /// Fallback pattern inference using existing logic
    fn infer_from_pattern(&self, input: &str) -> Result<(Provider, String), LlmError> {
        // Check if input is just a provider name (should error)
        if Provider::from_alias(input).is_some() {
            return Err(LlmError::validation(format!(
                "Input '{input}' is a provider name, not a model. Use format 'provider:model' or a specific model alias."
            )));
        }

        // Try to infer provider from model name patterns
        for provider in [Provider::OpenAI, Provider::Anthropic, Provider::Google] {
            for alias in provider.aliases() {
                // Check if the model starts with an alias followed by a separator
                if input.starts_with(&format!("{alias}-"))
                    || input.starts_with(&format!("{alias}."))
                    || input.starts_with(&format!("{alias}:"))
                    || input.starts_with(&format!("{alias}_"))
                {
                    return Ok((provider, input.to_string()));
                }
            }
        }

        Err(LlmError::validation(format!(
            "Unable to determine provider for model: '{input}'. Use format 'provider:model' or a recognized alias."
        )))
    }
}

/// Global alias resolver with dynamic user alias support
static GLOBAL_ALIAS_RESOLVER: OnceLock<RwLock<AliasResolver>> = OnceLock::new();

/// Get the global alias resolver, initializing it if needed
pub fn get_global_alias_resolver(config_base_path: &Path) -> &'static RwLock<AliasResolver> {
    GLOBAL_ALIAS_RESOLVER.get_or_init(|| {
        RwLock::new(AliasResolver::new(
            &config_base_path.join(ALIASES_CONFIG_FILE),
        ))
    })
}
