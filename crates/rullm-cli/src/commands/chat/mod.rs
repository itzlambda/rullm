mod completer;
pub mod interactive;
mod prompt;
pub mod slash_command;

use anyhow::Result;
use clap::Args;
use clap_complete::engine::ArgValueCompleter;
use reedline::{
    ColumnarMenu, DefaultHinter, DefaultValidator, EditCommand, Emacs, FileBackedHistory, KeyCode,
    KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Vi,
    default_emacs_keybindings, default_vi_insert_keybindings, default_vi_normal_keybindings,
};
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

use crate::args::{Cli, CliConfig, model_completer};
use crate::cli_helpers::resolve_model;
use crate::client;
use crate::output::OutputLevel;

pub use completer::SlashCommandCompleter;
pub use interactive::run_interactive_chat;
pub use prompt::ChatPrompt;

#[derive(Args)]
pub struct ChatArgs {
    /// Model to use in format: provider/model-name (e.g., openai/gpt-4, gemini/gemini-pro, anthropic/claude-3-sonnet)
    #[arg(short, long, add = ArgValueCompleter::new(model_completer))]
    pub model: Option<String>,
}

impl ChatArgs {
    pub async fn run(
        &self,
        _output_level: OutputLevel,
        cli_config: &CliConfig,
        cli: &Cli,
    ) -> Result<()> {
        let model_str = resolve_model(&cli.model, &self.model, &cli_config.config.default_model)?;
        let client = client::from_model(&model_str, cli, cli_config)?;
        run_interactive_chat(&client, None, cli_config, !cli.no_streaming).await?;
        Ok(())
    }
}

fn add_common_keybindings(keybindings: &mut reedline::Keybindings) {
    keybindings.add_binding(
        KeyModifiers::ALT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
}

fn setup_reedline(vim_mode: bool, data_path: &Path) -> Result<Reedline> {
    let completer = Box::new(SlashCommandCompleter::new());
    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));
    let edit_mode: Box<dyn reedline::EditMode> = if vim_mode {
        let mut vi_insert_keybindings = default_vi_insert_keybindings();
        let mut vi_normal_keybindings = default_vi_normal_keybindings();
        add_common_keybindings(&mut vi_insert_keybindings);
        add_common_keybindings(&mut vi_normal_keybindings);
        Box::new(Vi::new(vi_insert_keybindings, vi_normal_keybindings))
    } else {
        let mut emacs_keybindings = default_emacs_keybindings();
        add_common_keybindings(&mut emacs_keybindings);
        Box::new(Emacs::new(emacs_keybindings))
    };
    let history = Box::new(
        FileBackedHistory::with_file(5, data_path.join("history.txt"))
            .expect("Error configuring history with file"),
    );
    let temp_file = NamedTempFile::new()?;
    let editor = get_preferred_editor();
    let line_editor = Reedline::create()
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_hinter(Box::new(DefaultHinter::default()))
        .with_history(history)
        .with_validator(Box::new(DefaultValidator))
        .with_buffer_editor(Command::new(editor), temp_file.path().to_path_buf())
        .with_edit_mode(edit_mode);
    Ok(line_editor)
}

pub(crate) fn get_preferred_editor() -> String {
    std::env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string())
}
