use clap::Subcommand;

use crate::cli_client::CliClient;
use anyhow::Result;
use futures::StreamExt;
use rullm_core::LlmError;
use std::io::{self, Write};

use crate::spinner::Spinner;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod alias;
pub mod chat;
pub mod completions;
pub mod info;
pub mod templates;

pub mod keys;
pub mod models;

// Re-export the command args structs
pub use alias::AliasArgs;
pub use chat::ChatArgs;
pub use completions::CompletionsArgs;
pub use info::InfoArgs;
pub use keys::KeysArgs;
pub use models::ModelsArgs;

// Example strings for after_long_help
const CHAT_EXAMPLES: &str = r#"EXAMPLES:
  rullm chat                               # Start chat with default model
  rullm chat -m openai/gpt-4              # Chat with GPT-4
  rullm chat -m claude                     # Chat using claude alias
  rullm chat -m gemini/gemini-pro          # Chat with Gemini Pro"#;

const MODELS_EXAMPLES: &str = r#"EXAMPLES:
  rullm models list                        # List cached models
  rullm models update -m openai/gpt-4      # Fetch OpenAI models
  rullm models default openai/gpt-4o       # Set default model
  rullm models clear                       # Clear model cache"#;

const KEYS_EXAMPLES: &str = r#"EXAMPLES:
  rullm keys set openai                    # Set OpenAI API key (prompted)
  rullm keys set anthropic -k sk-ant-...  # Set Anthropic key directly
  rullm keys list                          # Show which providers have keys
  rullm keys delete google                 # Remove Google API key"#;

const ALIAS_EXAMPLES: &str = r#"EXAMPLES:
  rullm alias list                         # Show all available aliases
  rullm alias add gpt4 openai/gpt-4        # Create custom alias
  rullm alias show claude                  # Show alias details
  rullm alias remove gpt4                  # Remove custom alias"#;

const INFO_EXAMPLES: &str = r#"EXAMPLES:
  rullm info                               # Show config paths and API key status"#;

const COMPLETIONS_EXAMPLES: &str = r#"EXAMPLES:
  rullm completions bash > ~/.bashrc       # Add bash completions
  rullm completions zsh > ~/.zshrc         # Add zsh completions
  rullm completions fish > ~/.config/fish/completions/rullm.fish"#;

#[derive(Subcommand)]
pub enum Commands {
    /// Start an interactive chat session
    #[command(after_long_help = CHAT_EXAMPLES)]
    Chat(ChatArgs),
    /// Manage or inspect models
    #[command(after_long_help = MODELS_EXAMPLES)]
    Models(ModelsArgs),
    /// Show configuration and system information
    #[command(after_long_help = INFO_EXAMPLES)]
    Info(InfoArgs),
    /// Manage API keys
    #[command(after_long_help = KEYS_EXAMPLES)]
    Keys(KeysArgs),
    /// Manage model aliases
    #[command(after_long_help = ALIAS_EXAMPLES)]
    Alias(AliasArgs),
    /// Generate shell completions
    #[command(after_long_help = COMPLETIONS_EXAMPLES)]
    Completions(CompletionsArgs),
    /// Manage templates
    #[command(
        after_long_help = "EXAMPLES:\n  rullm templates list\n  rullm templates show code-review\n  rullm templates remove old-template"
    )]
    Templates(templates::TemplatesArgs),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelsCache {
    pub last_updated: DateTime<Utc>,
    pub models: Vec<String>,
}

impl ModelsCache {
    fn new(models: Vec<String>) -> Self {
        Self {
            last_updated: Utc::now(),
            models,
        }
    }
}

/// Helper function to check environment variable status, eliminating duplication
fn env_var_status(var_name: &str) -> &'static str {
    if std::env::var(var_name).is_ok() {
        "Present"
    } else {
        "None"
    }
}

pub async fn run_single_query(
    client: &CliClient,
    query: &str,
    system_prompt: Option<&str>,
    streaming: bool,
) -> Result<(), LlmError> {
    if streaming {
        // Use token-by-token streaming for real-time output
        if let Some(_system) = system_prompt {
            // TODO: Need to implement system prompt handling in streaming
            // For now, fall back to non-streaming
            let spinner = Spinner::new("Generating response");
            spinner.start().await;

            // Small delay to ensure spinner starts
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            match client.chat(query).await {
                Ok(response) => {
                    spinner.stop_and_replace(&format!("{response}\n"));
                }
                Err(e) => {
                    spinner.stop_and_replace(&format!("Error: {e}\n"));
                    return Err(e);
                }
            }
        } else {
            // Show spinner while waiting for first token
            let spinner = Spinner::new("Generating response");
            spinner.start().await;

            // Small delay to ensure spinner starts
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Simple query streaming
            let messages = vec![("user".to_string(), query.to_string())];

            match client.stream_chat_raw(messages).await {
                Ok(mut stream) => {
                    let mut first_token = true;
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(token) => {
                                if first_token {
                                    spinner.stop();
                                    first_token = false;
                                }
                                print!("{token}");
                                io::stdout()
                                    .flush()
                                    .map_err(|e| LlmError::unknown(e.to_string()))?;
                            }
                            Err(err) => {
                                spinner.stop_and_replace(&format!("Error: {err}\n"));
                                return Err(err);
                            }
                        }
                    }
                    println!(); // Final newline

                    // Ensure spinner is stopped if no tokens were received
                    if first_token {
                        spinner.stop_and_replace("(No response received)\n");
                    }
                }
                Err(e) => {
                    spinner.stop_and_replace(&format!("Error: {e}\n"));
                    return Err(e);
                }
            }
        }
    } else {
        // Non-streaming path with spinner
        let spinner = Spinner::new("Generating response");
        spinner.start().await;

        // Small delay to ensure spinner starts
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // TODO: Implement system prompt support properly
        let result = client.chat(query).await;

        match result {
            Ok(response) => {
                spinner.stop_and_replace(&format!("{response}\n"));
            }
            Err(e) => {
                spinner.stop_and_replace(&format!("Error: {e}\n"));
                return Err(e);
            }
        }
    }

    Ok(())
}

fn format_duration(duration: chrono::Duration) -> String {
    let days = duration.num_days();
    let hours = duration.num_hours() % 24;

    match (days, hours) {
        (0, h) => format!("{h}h"),
        (1, 0) => "1 day".to_string(),
        (1, h) => format!("1 day {h}h"),
        (d, 0) => format!("{d} days"),
        (d, h) => format!("{d} days {h}h"),
    }
}
