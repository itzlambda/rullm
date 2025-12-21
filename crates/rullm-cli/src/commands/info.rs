use anyhow::Result;
use clap::Args;

use crate::{
    args::{Cli, CliConfig},
    auth,
    commands::env_var_status,
    constants::*,
    output::OutputLevel,
};

#[derive(Args)]
pub struct InfoArgs {
    // Info command has no arguments
}

impl InfoArgs {
    pub async fn run(
        &self,
        output_level: OutputLevel,
        cli_config: &CliConfig,
        _cli: &Cli,
    ) -> Result<()> {
        let config_path = cli_config.config_base_path.join(CONFIG_FILE_NAME);
        let models_path = cli_config.data_base_path.join(MODEL_FILE_NAME);
        let auth_path = auth::auth_config_path(&cli_config.config_base_path);
        let templates_path = cli_config.config_base_path.join(TEMPLATES_DIR_NAME);

        // crate::output::heading("Config files:", output_level);
        crate::output::note(
            &format!("config file: {}", config_path.display()),
            output_level,
        );
        crate::output::note(&format!("auth file: {}", auth_path.display()), output_level);
        crate::output::note(
            &format!("models cache file: {}", models_path.display()),
            output_level,
        );
        crate::output::note(
            &format!("templates dir: {}", templates_path.display()),
            output_level,
        );

        crate::output::heading("\nEnv Vars:", output_level);
        crate::output::note(
            &format!("OPENAI_API_KEY = {}", env_var_status("OPENAI_API_KEY")),
            output_level,
        );
        crate::output::note(
            &format!(
                "ANTHROPIC_API_KEY = {}",
                env_var_status("ANTHROPIC_API_KEY")
            ),
            output_level,
        );
        crate::output::note(
            &format!(
                "GOOGLE_AI_API_KEY = {}",
                env_var_status("GOOGLE_AI_API_KEY")
            ),
            output_level,
        );

        crate::output::heading("\nVersion info:", output_level);
        crate::output::note(
            &format!("version: {}", env!("CARGO_PKG_VERSION")),
            output_level,
        );

        Ok(())
    }
}
