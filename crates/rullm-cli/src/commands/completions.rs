use std::io;

use anyhow::Result;
use clap::{Args, CommandFactory, ValueEnum};
use clap_complete::{generate, shells};

use crate::{
    args::{Cli, CliConfig},
    constants::BINARY_NAME,
    output::OutputLevel,
};

#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: Shell,
}

#[derive(Clone, ValueEnum)]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl CompletionsArgs {
    pub async fn run(
        &self,
        _output_level: OutputLevel,
        _cli_config: &CliConfig,
        _cli: &Cli,
    ) -> Result<()> {
        use crate::args::Cli;
        match self.shell {
            Shell::Bash => generate(
                shells::Bash,
                &mut Cli::command(),
                BINARY_NAME,
                &mut io::stdout(),
            ),
            Shell::Zsh => generate(
                shells::Zsh,
                &mut Cli::command(),
                BINARY_NAME,
                &mut io::stdout(),
            ),
            Shell::Fish => generate(
                shells::Fish,
                &mut Cli::command(),
                BINARY_NAME,
                &mut io::stdout(),
            ),
            Shell::PowerShell => generate(
                shells::PowerShell,
                &mut Cli::command(),
                BINARY_NAME,
                &mut io::stdout(),
            ),
            Shell::Elvish => generate(
                shells::Elvish,
                &mut Cli::command(),
                BINARY_NAME,
                &mut io::stdout(),
            ),
        }
        Ok(())
    }
}
