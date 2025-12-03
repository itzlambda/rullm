//! Auth command handlers for managing credentials.

use anyhow::Result;
use clap::{Args, Subcommand};
use etcetera::BaseStrategy;
use strum::IntoEnumIterator;

use crate::auth::{self, AuthConfig, Credential};
use crate::oauth::{anthropic::AnthropicOAuth, openai::OpenAIOAuth};
use crate::output::OutputLevel;
use crate::provider::Provider;

#[derive(Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub action: AuthAction,
}

#[derive(Subcommand)]
pub enum AuthAction {
    /// Login to a provider (OAuth or API key)
    Login {
        /// Provider name (anthropic, openai, groq, openrouter, google)
        provider: Option<Provider>,
    },
    /// Logout from a provider (remove stored credentials)
    Logout {
        /// Provider name (anthropic, openai, groq, openrouter, google)
        provider: Option<Provider>,
    },
    /// List all credentials and environment variables
    #[command(alias = "ls")]
    List,
}

/// Authentication method selection.
#[derive(Debug, Clone, Copy)]
pub enum AuthMethod {
    OAuth,
    ApiKey,
}

impl AuthArgs {
    pub async fn run(
        &self,
        output_level: OutputLevel,
        config_base_path: &std::path::Path,
    ) -> Result<()> {
        match &self.action {
            AuthAction::Login { provider } => {
                let provider = match provider {
                    Some(p) => p.clone(),
                    None => select_provider()?,
                };

                // Determine available auth methods for the provider
                let method = select_auth_method(&provider)?;

                match method {
                    AuthMethod::OAuth => {
                        let credential = match provider {
                            Provider::Anthropic => {
                                let oauth = AnthropicOAuth::new();
                                oauth.login().await?
                            }
                            Provider::OpenAI => {
                                let oauth = OpenAIOAuth::new();
                                oauth.login().await?
                            }
                            _ => {
                                anyhow::bail!(
                                    "OAuth is not supported for {}. Use API key instead.",
                                    provider
                                );
                            }
                        };

                        // Save the credential
                        let mut auth_config = AuthConfig::load(config_base_path)?;
                        auth_config.set(&provider, credential);
                        auth_config.save(config_base_path)?;

                        crate::output::success(
                            &format!("Successfully logged in to {provider}"),
                            output_level,
                        );
                    }
                    AuthMethod::ApiKey => {
                        let api_key = prompt_api_key(&provider)?;

                        if api_key.is_empty() {
                            anyhow::bail!("API key cannot be empty");
                        }

                        let mut auth_config = AuthConfig::load(config_base_path)?;
                        auth_config.set(&provider, Credential::api(api_key));
                        auth_config.save(config_base_path)?;

                        crate::output::success(
                            &format!("API key for {provider} has been saved"),
                            output_level,
                        );
                    }
                }
            }

            AuthAction::Logout { provider } => {
                let provider = match provider {
                    Some(p) => p.clone(),
                    None => select_provider()?,
                };

                let mut auth_config = AuthConfig::load(config_base_path)?;
                auth_config.remove(&provider);
                auth_config.save(config_base_path)?;

                crate::output::success(
                    &format!("Logged out from {provider}"),
                    output_level,
                );
            }

            AuthAction::List => {
                let auth_config = AuthConfig::load(config_base_path)?;
                print_credentials_list(&auth_config, output_level);
            }
        }

        Ok(())
    }
}

/// Select a provider interactively.
fn select_provider() -> Result<Provider> {
    use std::io::{self, Write};

    println!("\n? Select provider");
    let providers: Vec<Provider> = Provider::iter().collect();

    for (i, provider) in providers.iter().enumerate() {
        println!("  {}) {}", i + 1, format_provider_display(provider));
    }

    print!("\nEnter number (1-{}): ", providers.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let choice: usize = input.trim().parse().map_err(|_| {
        anyhow::anyhow!("Invalid selection")
    })?;

    if choice == 0 || choice > providers.len() {
        anyhow::bail!("Invalid selection");
    }

    Ok(providers[choice - 1].clone())
}

/// Select authentication method for a provider.
fn select_auth_method(provider: &Provider) -> Result<AuthMethod> {
    use std::io::{self, Write};

    // Check if OAuth is available for this provider
    let oauth_available = matches!(provider, Provider::Anthropic | Provider::OpenAI);

    if !oauth_available {
        // Only API key available
        return Ok(AuthMethod::ApiKey);
    }

    println!("\n? Select authentication method");
    println!("  1) OAuth (subscription-based access)");
    println!("  2) API Key");

    print!("\nEnter number (1-2): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    match input.trim() {
        "1" => Ok(AuthMethod::OAuth),
        "2" => Ok(AuthMethod::ApiKey),
        _ => anyhow::bail!("Invalid selection"),
    }
}

/// Prompt for API key input.
fn prompt_api_key(provider: &Provider) -> Result<String> {
    use std::io::{self, Write};

    print!("Enter API key for {provider}: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

/// Format provider name for display.
fn format_provider_display(provider: &Provider) -> &'static str {
    match provider {
        Provider::Anthropic => "Anthropic",
        Provider::OpenAI => "OpenAI",
        Provider::Groq => "Groq",
        Provider::OpenRouter => "OpenRouter",
        Provider::Google => "Google",
    }
}

/// Print the credentials list in a nice format.
fn print_credentials_list(auth_config: &AuthConfig, _output_level: OutputLevel) {
    let mut file_creds: Vec<(Provider, String)> = Vec::new();
    let mut env_creds: Vec<(Provider, String)> = Vec::new();

    for provider in Provider::iter() {
        // Check file credentials
        if let Some(cred) = auth_config.get(&provider) {
            file_creds.push((provider, cred.type_display().to_string()));
        } else {
            // Check environment variable
            let env_key = provider.env_key();
            if std::env::var(env_key).is_ok() {
                env_creds.push((provider, env_key.to_string()));
            }
        }
    }

    // Print file credentials section
    if !file_creds.is_empty() {
        let config_path = auth::auth_config_path(
            &etcetera::choose_base_strategy()
                .unwrap()
                .config_dir()
                .join(crate::constants::BINARY_NAME),
        );
        println!("\n\u{250c}  Credentials {}", config_path.display());
        println!("\u{2502}");

        for (provider, cred_type) in &file_creds {
            println!(
                "\u{25cf}  {} {}",
                format_provider_display(provider),
                cred_type
            );
            println!("\u{2502}");
        }

        println!("\u{2514}  {} credentials", file_creds.len());
    }

    // Print environment variables section
    if !env_creds.is_empty() {
        println!("\n\u{250c}  Environment");
        println!("\u{2502}");

        for (provider, env_key) in &env_creds {
            println!(
                "\u{25cf}  {} {}",
                format_provider_display(provider),
                env_key
            );
            println!("\u{2502}");
        }

        println!("\u{2514}  {} environment variables", env_creds.len());
    }

    if file_creds.is_empty() && env_creds.is_empty() {
        println!("\nNo credentials configured.");
        println!("Use 'rullm auth login' to add credentials.");
    }
}
