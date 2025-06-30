use crate::args::template_completer;
use crate::args::{Cli, CliConfig};
use crate::output::{self, OutputLevel};
use crate::templates::TemplateStore;
use anyhow::Result;
use clap::{Args, Subcommand};
use clap_complete::engine::ArgValueCompleter;

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
        #[arg(value_name = "NAME", add = ArgValueCompleter::new(template_completer))]
        name: String,
    },
    /// Remove a template file
    Remove {
        /// Template name to delete
        #[arg(value_name = "NAME", add = ArgValueCompleter::new(template_completer))]
        name: String,
    },
    /// Edit a template file in $EDITOR
    Edit {
        /// Template name to edit
        #[arg(value_name = "NAME", add = ArgValueCompleter::new(template_completer))]
        name: String,
    },
    /// Create a new template
    Create {
        /// Template name
        name: String,
        /// User prompt (use quotes if contains spaces)
        #[arg(long, short = 'u')]
        user_prompt: String,
        /// Optional system prompt
        #[arg(long, short = 's')]
        system_prompt: Option<String>,
        /// Optional description
        #[arg(long, short = 'd')]
        description: Option<String>,
        /// Default placeholder values in key=value format. Can be repeated.
        #[arg(long = "default", value_parser = parse_default_kv)]
        defaults: Vec<(String, String)>,
        /// Overwrite if template already exists
        #[arg(long, short = 'f')]
        force: bool,
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

                    if let Some(user) = &tpl.user_prompt {
                        output::note("User Prompt:", output_level);
                        output::note(user, output_level);
                    }

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
            TemplateAction::Edit { name } => {
                use std::env;
                use std::process::Command;
                use std::process::Stdio;

                if !store.contains(name) {
                    output::error(&format!("Template '{name}' not found."), output_level);
                    return Ok(());
                }

                let file_path = store.templates_dir().join(format!("{name}.toml"));
                let editor = env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());

                let status = Command::new(&editor)
                    .arg(&file_path)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status();

                match status {
                    Ok(s) if s.success() => {
                        // Reload the store to refresh in-memory state
                        if let Err(e) = store.load() {
                            output::warning(
                                &format!("Edited, but failed to reload templates: {e}"),
                                output_level,
                            );
                        } else {
                            output::success(&format!("Edited template '{name}'."), output_level);
                        }
                    }
                    Ok(s) => {
                        output::error(&format!("Editor exited with status: {s}"), output_level);
                    }
                    Err(e) => {
                        output::error(&format!("Failed to launch editor: {e}"), output_level);
                    }
                }
            }
            TemplateAction::Create {
                name,
                user_prompt,
                system_prompt,
                description,
                defaults,
                force,
            } => {
                // Check if already exists
                if store.contains(name) && !*force {
                    output::warning(
                        &format!("Template '{name}' already exists. Use --force to overwrite."),
                        output_level,
                    );
                    return Ok(());
                }

                let mut template =
                    crate::templates::Template::new(name.clone(), user_prompt.clone());
                template.system_prompt = system_prompt.clone();
                template.description = description.clone();
                // Insert defaults
                for (k, v) in defaults {
                    template.defaults.insert(k.clone(), v.clone());
                }

                match store.save(&template) {
                    Ok(_) => output::success(&format!("Saved template '{name}'."), output_level),
                    Err(e) => output::error(
                        &format!("Failed to save template '{name}': {e}"),
                        output_level,
                    ),
                }
            }
        }

        Ok(())
    }
}

fn parse_default_kv(s: &str) -> std::result::Result<(String, String), String> {
    if let Some((k, v)) = s.split_once('=') {
        if k.trim().is_empty() {
            return Err("Key cannot be empty".into());
        }
        Ok((k.trim().to_string(), v.trim().to_string()))
    } else {
        Err("Expected key=value format".into())
    }
}
