use anyhow::Result;
use clap::{Args, Subcommand};
use strum::IntoEnumIterator;

use crate::provider::Provider;
use crate::{
    api_keys::ApiKeys,
    args::{Cli, CliConfig},
    output::OutputLevel,
};

#[derive(Args)]
pub struct KeysArgs {
    #[command(subcommand)]
    pub action: KeysAction,
}

#[derive(Subcommand)]
pub enum KeysAction {
    /// Set an API key for a provider
    Set {
        /// Provider name (openai, anthropic, google)
        provider: Provider,
        /// API key (if not provided, will read from stdin)
        #[arg(short, long)]
        key: Option<String>,
    },
    /// Delete an API key for a provider
    Delete {
        /// Provider name (openai, anthropic, google)
        provider: Provider,
    },
    /// List which providers have API keys set
    List,
}

impl KeysArgs {
    pub async fn run(
        &self,
        output_level: OutputLevel,
        cli_config: &mut CliConfig,
        _cli: &Cli,
    ) -> Result<()> {
        match &self.action {
            KeysAction::Set { provider, key } => {
                let api_key = if let Some(key) = key {
                    key.clone()
                } else {
                    use std::io::{self, Write};
                    print!("Enter API key for {provider}: ");
                    io::stdout().flush()?;

                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    input.trim().to_string()
                };

                if api_key.is_empty() {
                    return Err(anyhow::anyhow!("API key cannot be empty"));
                }

                let api_keys = &mut cli_config.api_keys;
                ApiKeys::set_api_key_for_provider(provider, api_keys, &api_key);
                cli_config.save_api_keys()?;

                crate::output::success(
                    &format!("API key for {provider} has been saved"),
                    output_level,
                );
            }
            KeysAction::Delete { provider } => {
                let api_keys = &mut cli_config.api_keys;
                ApiKeys::delete_api_key_for_provider(provider, api_keys);
                cli_config.save_api_keys()?;

                crate::output::success(
                    &format!("API key for {provider} has been deleted"),
                    output_level,
                );
            }
            KeysAction::List => {
                let api_keys = cli_config.api_keys.clone();

                for provider in Provider::iter() {
                    let has_key = match provider {
                        Provider::OpenAI => api_keys.openai_api_key.is_some(),
                        Provider::Anthropic => api_keys.anthropic_api_key.is_some(),
                        Provider::Google => api_keys.google_ai_api_key.is_some(),
                    };

                    if has_key {
                        crate::output::note(
                            &crate::output::format_provider(&provider.to_string()),
                            output_level,
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
