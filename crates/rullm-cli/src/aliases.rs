use super::provider::Provider;
use rullm_core::error::LlmError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{OnceLock, RwLock};

/// User alias configuration that can be saved/loaded from file
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserAliasConfig {
    /// User-defined aliases mapping alias -> provider/model
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

    /// Validate that a target is in valid provider/model format
    fn validate_target(target: &str) -> Result<(), LlmError> {
        if let Some((provider_str, model_name)) = target.split_once('/') {
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
                "Target '{target}' must be in 'provider/model' format"
            )))
        }
    }
}

/// Unified alias resolver that handles both default (built-in) and user-defined aliases.
/// All aliases resolve to canonical `provider/model` format.
#[derive(Debug, Clone)]
pub struct AliasResolver {
    /// User-defined aliases that override defaults
    user_aliases: HashMap<String, String>,
    /// Whether to enable case-insensitive matching
    case_insensitive: bool,
}

impl AliasResolver {
    /// Create a new alias resolver with default configuration
    pub fn new() -> Self {
        Self {
            user_aliases: HashMap::new(),
            case_insensitive: true,
        }
    }

    /// Create a resolver with user-defined aliases
    pub fn with_user_aliases(user_aliases: HashMap<String, String>) -> Self {
        Self {
            user_aliases,
            case_insensitive: true,
        }
    }

    /// Set case sensitivity for alias matching
    #[cfg(test)]
    pub fn case_insensitive(mut self, enabled: bool) -> Self {
        self.case_insensitive = enabled;
        self
    }

    /// Add a user alias at runtime
    #[cfg(test)]
    pub fn add_alias(&mut self, alias: &str, target: &str) -> Result<(), LlmError> {
        // Validate that target is in provider/model format
        self.validate_target(target)?;

        let key = if self.case_insensitive {
            alias.to_lowercase()
        } else {
            alias.to_string()
        };

        self.user_aliases.insert(key, target.to_string());
        Ok(())
    }

    /// Remove a user alias
    #[cfg(test)]
    pub fn remove_alias(&mut self, alias: &str) -> bool {
        let key = if self.case_insensitive {
            alias.to_lowercase()
        } else {
            alias.to_string()
        };

        self.user_aliases.remove(&key).is_some()
    }

    /// List all available aliases (both default and user)
    pub fn list_aliases(&self) -> Vec<(String, String)> {
        let mut aliases = Vec::new();

        // Add default aliases
        for (alias, target) in default_aliases() {
            aliases.push((alias.clone(), target.clone()));
        }

        // Add user aliases (which can override defaults)
        for (alias, target) in &self.user_aliases {
            aliases.push((alias.clone(), target.clone()));
        }

        aliases.sort_by(|a, b| a.0.cmp(&b.0));
        aliases
    }

    /// Resolve an input string to canonical (Provider, model) format
    ///
    /// Resolution order:
    /// 1. If already in provider/model format → validate and use
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

        // 1. Fast path: already in provider/model format
        if let Some((provider_str, model_name)) = input.split_once('/') {
            if let Some(provider) = Provider::from_alias(provider_str) {
                return Ok((provider, model_name.to_string()));
            } else {
                return Err(LlmError::validation(format!(
                    "Unknown provider prefix: '{provider_str}'"
                )));
            }
        }

        // 2. Check user aliases first (user overrides defaults)
        if let Some(target) = self.user_aliases.get(&normalized_input) {
            return self.parse_target(target);
        }

        // 3. Check default aliases
        if let Some(target) = default_aliases().get(&normalized_input) {
            return self.parse_target(target);
        }

        // 4. Try pattern inference (fallback to existing logic)
        self.infer_from_pattern(input)
    }

    /// Validate that a target is in valid provider/model format
    #[cfg(test)]
    fn validate_target(&self, target: &str) -> Result<(), LlmError> {
        if let Some((provider_str, model_name)) = target.split_once('/') {
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
                "Target '{target}' must be in 'provider/model' format"
            )))
        }
    }

    /// Parse a target string to (Provider, model)
    fn parse_target(&self, target: &str) -> Result<(Provider, String), LlmError> {
        if let Some((provider_str, model_name)) = target.split_once('/') {
            if let Some(provider) = Provider::from_alias(provider_str) {
                Ok((provider, model_name.to_string()))
            } else {
                Err(LlmError::validation(format!(
                    "Unknown provider '{provider_str}' in target '{target}'"
                )))
            }
        } else {
            Err(LlmError::validation(format!(
                "Invalid target format '{target}', expected 'provider/model'"
            )))
        }
    }

    /// Fallback pattern inference using existing logic
    fn infer_from_pattern(&self, input: &str) -> Result<(Provider, String), LlmError> {
        // Check if input is just a provider name (should error)
        if Provider::from_alias(input).is_some() {
            return Err(LlmError::validation(format!(
                "Input '{input}' is a provider name, not a model. Use format 'provider/model' or a specific model alias."
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
            "Unable to determine provider for model: '{input}'. Use format 'provider/model' or a recognized alias."
        )))
    }
}

/// Global alias resolver with dynamic user alias support
static GLOBAL_ALIAS_RESOLVER: OnceLock<RwLock<AliasResolver>> = OnceLock::new();

/// Get the global alias resolver, initializing it if needed
pub fn get_global_alias_resolver() -> &'static RwLock<AliasResolver> {
    GLOBAL_ALIAS_RESOLVER.get_or_init(|| RwLock::new(AliasResolver::new()))
}

/// Get default built-in aliases
/// Returns a map of alias -> canonical provider/model format
fn default_aliases() -> &'static HashMap<String, String> {
    static DEFAULT_ALIASES: OnceLock<HashMap<String, String>> = OnceLock::new();

    DEFAULT_ALIASES.get_or_init(|| {
        let mut aliases = HashMap::new();

        // === OpenAI Aliases ===

        // Popular model shortcuts
        aliases.insert("gpt4".to_string(), "openai/gpt-4".to_string());
        aliases.insert("gpt4o".to_string(), "openai/gpt-4o".to_string());
        aliases.insert("gpt4o-mini".to_string(), "openai/gpt-4o-mini".to_string());
        aliases.insert("gpt3".to_string(), "openai/gpt-3.5-turbo".to_string());
        aliases.insert("gpt35".to_string(), "openai/gpt-3.5-turbo".to_string());
        aliases.insert("turbo".to_string(), "openai/gpt-3.5-turbo".to_string());
        aliases.insert("chatgpt".to_string(), "openai/gpt-4o".to_string());

        // Reasoning models
        aliases.insert("o1".to_string(), "openai/o1".to_string());
        aliases.insert("o1-mini".to_string(), "openai/o1-mini".to_string());
        aliases.insert("o1-preview".to_string(), "openai/o1-preview".to_string());

        // === Anthropic Aliases ===

        // Claude shortcuts
        aliases.insert(
            "claude".to_string(),
            "anthropic/claude-3-5-sonnet-20241022".to_string(),
        );
        aliases.insert(
            "claude3".to_string(),
            "anthropic/claude-3-5-sonnet-20241022".to_string(),
        );
        aliases.insert(
            "claude35".to_string(),
            "anthropic/claude-3-5-sonnet-20241022".to_string(),
        );
        aliases.insert(
            "sonnet".to_string(),
            "anthropic/claude-3-5-sonnet-20241022".to_string(),
        );
        aliases.insert(
            "opus".to_string(),
            "anthropic/claude-3-opus-20240229".to_string(),
        );
        aliases.insert(
            "haiku".to_string(),
            "anthropic/claude-3-haiku-20240307".to_string(),
        );

        // Version-specific shortcuts
        aliases.insert(
            "claude-3-sonnet".to_string(),
            "anthropic/claude-3-sonnet-20240229".to_string(),
        );
        aliases.insert(
            "claude-3-opus".to_string(),
            "anthropic/claude-3-opus-20240229".to_string(),
        );
        aliases.insert(
            "claude-3-haiku".to_string(),
            "anthropic/claude-3-haiku-20240307".to_string(),
        );

        // === Google Aliases ===

        // Gemini shortcuts
        aliases.insert("gemini".to_string(), "google/gemini-1.5-pro".to_string());
        aliases.insert(
            "gemini-pro".to_string(),
            "google/gemini-2.5-pro".to_string(),
        );
        aliases.insert(
            "gemini-flash".to_string(),
            "google/gemini-1.5-flash".to_string(),
        );
        aliases.insert(
            "gemini2".to_string(),
            "google/gemini-2.0-flash-exp".to_string(),
        );
        aliases.insert(
            "gemini-2".to_string(),
            "google/gemini-2.0-flash-exp".to_string(),
        );

        // Legacy support
        aliases.insert("bard".to_string(), "google/gemini-1.5-pro".to_string());

        aliases
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_aliases() {
        let resolver = AliasResolver::new();

        // Test OpenAI aliases
        assert_eq!(
            resolver.resolve("gpt4").unwrap(),
            (Provider::OpenAI, "gpt-4".to_string())
        );
        assert_eq!(
            resolver.resolve("chatgpt").unwrap(),
            (Provider::OpenAI, "gpt-4o".to_string())
        );
        assert_eq!(
            resolver.resolve("turbo").unwrap(),
            (Provider::OpenAI, "gpt-3.5-turbo".to_string())
        );

        // Test Anthropic aliases
        assert_eq!(
            resolver.resolve("claude").unwrap(),
            (
                Provider::Anthropic,
                "claude-3-5-sonnet-20241022".to_string()
            )
        );
        assert_eq!(
            resolver.resolve("sonnet").unwrap(),
            (
                Provider::Anthropic,
                "claude-3-5-sonnet-20241022".to_string()
            )
        );
        assert_eq!(
            resolver.resolve("opus").unwrap(),
            (Provider::Anthropic, "claude-3-opus-20240229".to_string())
        );

        // Test Google aliases
        assert_eq!(
            resolver.resolve("gemini").unwrap(),
            (Provider::Google, "gemini-1.5-pro".to_string())
        );
        assert_eq!(
            resolver.resolve("gemini-flash").unwrap(),
            (Provider::Google, "gemini-1.5-flash".to_string())
        );
    }

    #[test]
    fn test_case_insensitive_aliases() {
        // Test default case insensitive behavior
        let resolver = AliasResolver::new();

        assert_eq!(
            resolver.resolve("GPT4").unwrap(),
            (Provider::OpenAI, "gpt-4".to_string())
        );
        assert_eq!(
            resolver.resolve("CLAUDE").unwrap(),
            (
                Provider::Anthropic,
                "claude-3-5-sonnet-20241022".to_string()
            )
        );
        assert_eq!(
            resolver.resolve("GEMINI").unwrap(),
            (Provider::Google, "gemini-1.5-pro".to_string())
        );

        // Test explicitly set case insensitive
        let resolver = AliasResolver::new().case_insensitive(true);
        assert_eq!(
            resolver.resolve("gpt4").unwrap(),
            (Provider::OpenAI, "gpt-4".to_string())
        );

        // Test case sensitive
        let resolver = AliasResolver::new().case_insensitive(false);
        assert_eq!(
            resolver.resolve("gpt4").unwrap(),
            (Provider::OpenAI, "gpt-4".to_string())
        );
        // This would fail in case-sensitive mode if the alias was uppercase
        assert!(resolver.resolve("GPT4").is_err());
    }

    #[test]
    fn test_explicit_provider_format() {
        let resolver = AliasResolver::new();

        // Should work with explicit format
        assert_eq!(
            resolver.resolve("openai/gpt-4").unwrap(),
            (Provider::OpenAI, "gpt-4".to_string())
        );
        assert_eq!(
            resolver.resolve("anthropic/claude-3-sonnet").unwrap(),
            (Provider::Anthropic, "claude-3-sonnet".to_string())
        );
    }

    #[test]
    fn test_user_aliases() {
        let mut resolver = AliasResolver::new();

        // Add user alias
        resolver.add_alias("my-gpt", "openai/gpt-4").unwrap();
        resolver
            .add_alias("my-claude", "anthropic/claude-3-opus-20240229")
            .unwrap();

        // Should resolve user aliases
        assert_eq!(
            resolver.resolve("my-gpt").unwrap(),
            (Provider::OpenAI, "gpt-4".to_string())
        );
        assert_eq!(
            resolver.resolve("my-claude").unwrap(),
            (Provider::Anthropic, "claude-3-opus-20240229".to_string())
        );
    }

    #[test]
    fn test_user_overrides_default() {
        let mut resolver = AliasResolver::new();

        // Override default "gpt4" alias
        resolver.add_alias("gpt4", "openai/gpt-4o").unwrap();

        // Should use user override
        assert_eq!(
            resolver.resolve("gpt4").unwrap(),
            (Provider::OpenAI, "gpt-4o".to_string())
        );
    }

    #[test]
    fn test_pattern_inference_fallback() {
        let resolver = AliasResolver::new();

        // Should fall back to pattern inference for unrecognized aliases
        assert_eq!(
            resolver.resolve("gpt-4o-mini").unwrap(),
            (Provider::OpenAI, "gpt-4o-mini".to_string())
        );
        assert_eq!(
            resolver.resolve("claude-3-5-sonnet-20241022").unwrap(),
            (
                Provider::Anthropic,
                "claude-3-5-sonnet-20241022".to_string()
            )
        );
    }

    #[test]
    fn test_invalid_inputs() {
        let resolver = AliasResolver::new();

        // Provider names should error
        assert!(resolver.resolve("openai").is_err());
        assert!(resolver.resolve("anthropic").is_err());
        assert!(resolver.resolve("google").is_err());

        // Unknown aliases should error
        assert!(resolver.resolve("unknown-model").is_err());

        // Empty input should error
        assert!(resolver.resolve("").is_err());
        assert!(resolver.resolve("   ").is_err());
    }

    #[test]
    fn test_alias_management() {
        let mut resolver = AliasResolver::new();

        // Add valid alias
        assert!(resolver.add_alias("test", "openai/gpt-4").is_ok());

        // Add invalid alias (bad format)
        assert!(resolver.add_alias("bad", "invalid-format").is_err());
        assert!(
            resolver
                .add_alias("bad2", "unknown-provider/model")
                .is_err()
        );

        // Remove alias
        assert!(resolver.remove_alias("test"));
        assert!(!resolver.remove_alias("nonexistent"));
    }

    #[test]
    fn test_list_aliases() {
        let mut resolver = AliasResolver::new();
        resolver.add_alias("my-test", "openai/gpt-4").unwrap();

        let aliases = resolver.list_aliases();

        // Should include both default and user aliases
        assert!(
            aliases
                .iter()
                .any(|(k, v)| k == "gpt4" && v == "openai/gpt-4")
        );
        assert!(
            aliases
                .iter()
                .any(|(k, v)| k == "claude" && v == "anthropic/claude-3-5-sonnet-20241022")
        );
        assert!(
            aliases
                .iter()
                .any(|(k, v)| k == "my-test" && v == "openai/gpt-4")
        );

        // Should be sorted
        let keys: Vec<&String> = aliases.iter().map(|(k, _)| k).collect();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        assert_eq!(keys, sorted_keys);
    }

    #[test]
    fn test_user_alias_config_file_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path();

        // Test saving and loading empty config
        let config = UserAliasConfig::default();
        config.save_to_file(config_path).unwrap();

        let loaded_config = UserAliasConfig::load_from_file(config_path).unwrap();
        assert!(loaded_config.aliases.is_empty());

        // Test adding aliases and saving
        let mut config = UserAliasConfig::default();
        config.add_alias("my-model", "openai/gpt-4").unwrap();
        config
            .add_alias("fast-model", "anthropic/claude-3-haiku-20240307")
            .unwrap();

        config.save_to_file(config_path).unwrap();

        // Test loading with aliases
        let loaded_config = UserAliasConfig::load_from_file(config_path).unwrap();
        assert_eq!(loaded_config.aliases.len(), 2);
        assert_eq!(
            loaded_config.aliases.get("my-model"),
            Some(&"openai/gpt-4".to_string())
        );
        assert_eq!(
            loaded_config.aliases.get("fast-model"),
            Some(&"anthropic/claude-3-haiku-20240307".to_string())
        );
    }

    #[test]
    fn test_user_alias_validation() {
        let mut config = UserAliasConfig::default();

        // Valid aliases should work
        assert!(config.add_alias("test", "openai/gpt-4").is_ok());
        assert!(
            config
                .add_alias("test2", "anthropic/claude-3-sonnet")
                .is_ok()
        );

        // Invalid aliases should fail
        assert!(config.add_alias("bad1", "invalid-format").is_err());
        assert!(config.add_alias("bad2", "unknown-provider/model").is_err());
        assert!(config.add_alias("bad3", "openai/").is_err());
    }
}
