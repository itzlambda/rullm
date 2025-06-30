use crate::args::{Cli, CliConfig};
use crate::output::{self, OutputLevel};
use crate::templates::TemplateStore;
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct TemplatesArgs {
    #[command(subcommand)]
    pub action: TemplateAction,
}

#[derive(Subcommand)]
pub enum TemplateAction {
    /// List all templates
    List,
    /// Show a specific template's details
    Show {
        /// Template name
        name: String,
    },
    /// Remove a template file
    Remove {
        /// Template name to delete
        name: String,
    },
}

impl TemplatesArgs {
    pub async fn run(
        &self,
        output_level: OutputLevel,
        cli_config: &CliConfig,
        _cli: &Cli,
    ) -> Result<()> {
        let mut store = TemplateStore::new(&cli_config.config_base_path);
        store.load()?;

        match &self.action {
            TemplateAction::List => {
                let names = store.list();
                if names.is_empty() {
                    output::note("No templates found.", output_level);
                } else {
                    output::heading("Available templates:", output_level);
                    for name in names {
                        output::note(&format!("  - {name}"), output_level);
                    }
                }
            }
            TemplateAction::Show { name } => {
                if let Some(tpl) = store.get(name) {
                    output::heading(&format!("Template: {name}"), output_level);
                    if let Some(desc) = &tpl.description {
                        output::note(&format!("Description: {desc}"), output_level);
                    }
                    if let Some(sys) = &tpl.system_prompt {
                        output::note("System Prompt:", output_level);
                        output::note(sys, output_level);
                    }
                    output::note("User Prompt:", output_level);
                    output::note(&tpl.user_prompt, output_level);

                    if !tpl.defaults.is_empty() {
                        output::note("\nDefaults:", output_level);
                        for (k, v) in &tpl.defaults {
                            output::note(&format!("  {k} = {v}"), output_level);
                        }
                    }
                } else {
                    output::error(&format!("Template '{name}' not found."), output_level);
                }
            }
            TemplateAction::Remove { name } => match store.delete(name) {
                Ok(true) => output::success(&format!("Removed template '{name}'."), output_level),
                Ok(false) => {
                    output::warning(&format!("Template '{name}' not found."), output_level)
                }
                Err(e) => output::error(
                    &format!("Failed to delete template '{name}': {e}"),
                    output_level,
                ),
            },
        }

        Ok(())
    }
}
