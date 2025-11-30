// Binary entry point for rullm-cli

mod aliases;
mod api_keys;
mod args;
mod cli_client;
mod cli_helpers;
mod client;
mod commands;
mod config;
mod constants;
mod output;
mod provider;
mod spinner;
mod templates;

use anyhow::Result;
use args::{Cli, CliConfig};
use clap::{CommandFactory, Parser};
use cli_helpers::resolve_direct_query_model;
use commands::Commands;
use output::OutputLevel;
use templates::resolve_template_prompts;

use crate::cli_helpers::merge_stdin_and_query;

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}

/// Parse CLI arguments, load configuration and dispatch to the requested
/// sub-command.
pub async fn run() -> Result<()> {
    // Enable shell completion generation when the user sets COMPLETE=fish etc.
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();
    let mut cli = Cli::parse();
    cli.query = merge_stdin_and_query(cli.query.take());

    let mut cli_config = CliConfig::load();

    let output_level = if cli.quiet {
        OutputLevel::Quiet
    } else if cli.verbose {
        OutputLevel::Verbose
    } else {
        OutputLevel::Normal
    };

    // Validate that global -m is only used appropriately
    if cli.model.is_some() {
        match &cli.command {
            Some(Commands::Info(_))
            | Some(Commands::Keys(_))
            | Some(Commands::Alias(_))
            | Some(Commands::Completions(_)) => {
                use clap::error::ErrorKind;

                let mut cmd = Cli::command();
                cmd.error(
                    ErrorKind::UnknownArgument,
                    "unexpected argument '-m/--model' found",
                )
                .exit();
            }
            _ => {} // Allow for chat, models, or direct query
        }
    }

    // Validate that template flag is only used for quick-query mode
    if cli.template.is_some() && cli.command.is_some() {
        use clap::error::ErrorKind;

        let mut cmd = Cli::command();
        cmd.error(
            ErrorKind::UnknownArgument,
            "unexpected argument '-t/--template' found when using subcommands",
        )
        .exit();
    }

    // Handle commands
    match &cli.command {
        Some(Commands::Chat(args)) => args.run(output_level, &cli_config, &cli).await?,
        Some(Commands::Models(args)) => args.run(output_level, &mut cli_config, &cli).await?,
        Some(Commands::Info(args)) => args.run(output_level, &cli_config, &cli).await?,
        Some(Commands::Keys(args)) => args.run(output_level, &mut cli_config, &cli).await?,
        Some(Commands::Alias(args)) => args.run(output_level, &cli_config, &cli).await?,
        Some(Commands::Completions(args)) => args.run(output_level, &cli_config, &cli).await?,
        Some(Commands::Templates(args)) => args.run(output_level, &cli_config, &cli).await?,
        None => {
            if let Some(query) = &cli.query {
                let model_str =
                    resolve_direct_query_model(&cli.model, &cli_config.config.default_model)?;
                let client = client::from_model(&model_str, &cli, &cli_config)?;

                // Handle template if provided
                let (system_prompt, final_query) = if let Some(template_name) = &cli.template {
                    resolve_template_prompts(template_name, query, &cli_config.config_base_path)?
                } else {
                    (cli.system, query.clone())
                };

                commands::run_single_query(
                    &client,
                    &final_query,
                    system_prompt.as_deref(),
                    !cli.no_streaming,
                )
                .await
                .map_err(anyhow::Error::from)?;
            } else {
                eprintln!("Error: No query provided. Use --help for usage information.");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
