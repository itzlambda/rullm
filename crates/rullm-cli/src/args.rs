use etcetera::BaseStrategy;
use std::path::{Path, PathBuf};

use clap::Parser;
use clap_complete::CompletionCandidate;
use clap_complete::engine::ArgValueCompleter;
use std::ffi::OsStr;

use crate::auth::AuthConfig;
use crate::commands::models::load_models_cache;
use crate::commands::{Commands, ModelsCache};
use crate::config::{self, Config};
use crate::constants::BINARY_NAME;
use crate::templates::TemplateStore;

// Example strings for after_long_help
const CLI_EXAMPLES: &str = r#"EXAMPLES:
  rullm "What is Rust?"                           # Quick query with default model
  rullm -m openai/gpt-4 "Explain async Rust"     # Query with specific model
  rullm -m claude "Write a hello world program"  # Using model alias
  rullm --no-streaming "Tell me a story"          # Disable streaming for buffered output
  rullm -m gpt4 "Code a web server"               # Stream tokens as they arrive (default)
  rullm -t code-review "Review this code"         # Use template for query
  rullm -t greeting "Hello"                     # Template with input parameter
  rullm chat                                      # Start interactive chat
  rullm chat -m gemini/gemini-pro                # Chat with specific model
  rullm chat --no-streaming -m claude            # Interactive chat without streaming"#;

/// Helper function to remove quotes from values, eliminating duplication
fn unquote_value(value: &str) -> String {
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

/// Parse a single key-value pair
/// Supports formats: "key=value", "key=\"quoted value\"", "key value" (legacy)
fn parse_key_val(s: &str) -> anyhow::Result<(String, String)> {
    let s = s.trim();

    // Reject empty or whitespace-only input
    if s.is_empty() {
        return Err(anyhow::anyhow!(
            "invalid KEY=VALUE or KEY VALUE format: empty input"
        ));
    }

    // Prefer "key=value" format
    if let Some(pos) = s.find('=') {
        let key = s[..pos].trim().to_string();
        let value = s[pos + 1..].trim();

        // Reject empty keys
        if key.is_empty() {
            return Err(anyhow::anyhow!("invalid KEY=VALUE format: empty key"));
        }

        // Handle quoted values
        return Ok((key, unquote_value(value)));
    }

    // Fall back to legacy "key value" format (space-separated)
    let pos = s.find(' ').ok_or_else(|| {
        anyhow::anyhow!("invalid KEY=VALUE or KEY VALUE format: no '=' or space found in `{s}`")
    })?;

    let key = s[..pos].trim().to_string();
    let value = s[pos + 1..].trim();

    // Reject empty keys
    if key.is_empty() {
        return Err(anyhow::anyhow!("invalid KEY VALUE format: empty key"));
    }

    // Handle quoted values in legacy format too
    Ok((key, unquote_value(value)))
}

pub struct CliConfig {
    pub config_base_path: PathBuf,
    pub data_base_path: PathBuf,
    pub config: Config,
    pub models: Models,
    pub auth_config: AuthConfig,
}

impl CliConfig {
    pub fn load() -> Self {
        let strategy = etcetera::choose_base_strategy().unwrap();

        let config_base_path = strategy.config_dir().join(BINARY_NAME);
        let data_base_path = strategy.data_dir().join(BINARY_NAME);

        let config = config::Config::load(&config_base_path).unwrap();
        let models = Models::load(&data_base_path).unwrap();
        let auth_config = AuthConfig::load(&config_base_path).unwrap_or_default();

        Self {
            config_base_path,
            data_base_path,
            config,
            models,
            auth_config,
        }
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(about = "A CLI tool for interacting with LLM providers")]
#[command(name = BINARY_NAME)]
#[command(after_long_help = CLI_EXAMPLES)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Model to use in format: provider/model-name (e.g., openai/gpt-4, gemini/gemini-pro, anthropic/claude-3-sonnet)
    #[arg(short, long, add = ArgValueCompleter::new(model_completer))]
    pub model: Option<String>,

    /// Template to use for the query (only available for quick-query mode)
    #[arg(short, long, add = ArgValueCompleter::new(template_completer))]
    pub template: Option<String>,

    /// Set options in format: --option key value (e.g., --option temperature 0.1 --option max_tokens 2096)
    #[arg(long, value_parser = parse_key_val, global = true)]
    pub option: Vec<(String, String)>,

    /// Verbose output
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Quiet output (only show errors)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Disable streaming output (stream tokens by default)
    #[arg(long, global = true)]
    pub no_streaming: bool,

    /// System prompt
    #[arg(long, global = true)]
    pub system: Option<String>,

    /// The user query/prompt
    #[arg(value_name = "QUERY")]
    pub query: Option<String>,
}

pub struct Models {
    pub models: Vec<String>,
}

impl Models {
    pub fn load(base_path: &Path) -> anyhow::Result<Models> {
        use std::fs;
        let path = base_path.join(crate::constants::MODEL_FILE_NAME);

        if !path.exists() {
            return Ok(Models { models: Vec::new() });
        }

        let content = fs::read_to_string(path)?;

        // Try to parse as new format first
        if let Ok(cache) = serde_json::from_str::<ModelsCache>(&content) {
            return Ok(Models {
                models: cache.models,
            });
        }

        // Fallback to old format (simple array)
        let models: Vec<String> = serde_json::from_str(&content)?;
        Ok(Models { models })
    }
}

pub fn model_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    // Predefined providers or aliases
    const PROVIDED: &[&str] = &["openai:", "anthropic:", "google:"];

    let cli_config = CliConfig::load();
    let cur_str = current.to_string_lossy();

    // If there is a colon already, offer all cached models that start with input
    if cur_str.contains(':') {
        if let Some((provider, _)) = cur_str.split_once(':') {
            // Load cached models
            if let Ok(Some(entries)) = load_models_cache(&cli_config) {
                let mut v: Vec<CompletionCandidate> = entries
                    .models
                    .into_iter()
                    .filter(|m| m.starts_with(cur_str.as_ref()))
                    .map(|m| m.into())
                    .collect();

                // Always offer the raw `provider:` prefix too.
                v.push(format!("{provider}:").into());

                return v;
            }
        }

        // No cache available, fallthrough to simple provider + ':'
        return PROVIDED
            .iter()
            .filter(|p| p.starts_with(cur_str.as_ref()))
            .map(|m| (*m).into())
            .collect();
    }

    // Only offer provider prefixes when ':' is not yet typed
    PROVIDED
        .iter()
        .filter(|p| p.starts_with(cur_str.as_ref()))
        .map(|m| (*m).into())
        .collect()
}

pub fn template_completer(current: &std::ffi::OsStr) -> Vec<clap_complete::CompletionCandidate> {
    let cur_str = current.to_string_lossy();
    let mut candidates = Vec::new();

    // Only suggest .toml files in CWD as @filename.toml if user input starts with '@'
    if let Some(prefix) = cur_str.strip_prefix('@') {
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "toml" {
                        if let Some(fname) = path.file_name().and_then(|f| f.to_str()) {
                            if fname.starts_with(prefix) {
                                candidates.push(format!("@{fname}").into());
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Otherwise, suggest installed template names
        let strategy = match etcetera::choose_base_strategy() {
            Ok(s) => s,
            Err(_) => return candidates,
        };
        let config_base_path = strategy.config_dir().join(BINARY_NAME);
        let mut store = TemplateStore::new(&config_base_path);
        if store.load().is_ok() {
            for name in store.list() {
                if name.starts_with(cur_str.as_ref()) {
                    candidates.push(name.into());
                }
            }
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_val_equals_format() {
        // Basic key=value format
        let result = parse_key_val("temperature=0.7").unwrap();
        assert_eq!(result, ("temperature".to_string(), "0.7".to_string()));

        // With spaces around equals
        let result = parse_key_val("max_tokens = 100").unwrap();
        assert_eq!(result, ("max_tokens".to_string(), "100".to_string()));

        // Complex value
        let result = parse_key_val("model=openai/gpt-4").unwrap();
        assert_eq!(result, ("model".to_string(), "openai/gpt-4".to_string()));
    }

    #[test]
    fn test_parse_key_val_quoted_values() {
        // Double quoted values
        let result = parse_key_val("prompt=\"Hello world with spaces\"").unwrap();
        assert_eq!(
            result,
            ("prompt".to_string(), "Hello world with spaces".to_string())
        );

        // Single quoted values
        let result = parse_key_val("system='You are a helpful assistant'").unwrap();
        assert_eq!(
            result,
            (
                "system".to_string(),
                "You are a helpful assistant".to_string()
            )
        );

        // Quoted value with equals inside
        let result = parse_key_val("query=\"What is 2+2=\"").unwrap();
        assert_eq!(result, ("query".to_string(), "What is 2+2=".to_string()));
    }

    #[test]
    fn test_parse_key_val_legacy_space_format() {
        // Basic legacy format
        let result = parse_key_val("temperature 0.8").unwrap();
        assert_eq!(result, ("temperature".to_string(), "0.8".to_string()));

        // With quoted value in legacy format
        let result = parse_key_val("prompt \"Hello world\"").unwrap();
        assert_eq!(result, ("prompt".to_string(), "Hello world".to_string()));
    }

    #[test]
    fn test_parse_key_val_edge_cases() {
        // Empty value
        let result = parse_key_val("empty=").unwrap();
        assert_eq!(result, ("empty".to_string(), "".to_string()));

        // Value with multiple equals
        let result = parse_key_val("url=https://api.example.com/v1/chat").unwrap();
        assert_eq!(
            result,
            (
                "url".to_string(),
                "https://api.example.com/v1/chat".to_string()
            )
        );

        // Leading/trailing whitespace
        let result = parse_key_val("  key  =  value  ").unwrap();
        assert_eq!(result, ("key".to_string(), "value".to_string()));
    }

    #[test]
    fn test_parse_key_val_error_cases() {
        // No equals or space
        let result = parse_key_val("invalidformat");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("no '=' or space found")
        );

        // Empty string
        let result = parse_key_val("");
        assert!(result.is_err());

        // Only whitespace
        let result = parse_key_val("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_key_val_prefers_equals_over_space() {
        // When both = and space are present, = should take precedence
        let result = parse_key_val("key=value with spaces").unwrap();
        assert_eq!(result, ("key".to_string(), "value with spaces".to_string()));
    }
}
