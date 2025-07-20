use anyhow::Result;
use clap::Args;
use clap_complete::engine::ArgValueCompleter;
use futures::StreamExt;
use owo_colors::OwoColorize;
use reedline::{
    ColumnarMenu, Completer, DefaultHinter, DefaultValidator, EditCommand, Emacs,
    FileBackedHistory, KeyCode, KeyModifiers, MenuBuilder, Prompt, PromptEditMode,
    PromptHistorySearch, PromptHistorySearchStatus, Reedline, ReedlineEvent, ReedlineMenu, Signal,
    Suggestion, Vi, default_emacs_keybindings, default_vi_insert_keybindings,
    default_vi_normal_keybindings,
};
use rullm_core::simple::{SimpleLlm, SimpleLlmClient};
use rullm_core::types::{ChatRequestBuilder, ChatRole, ChatStreamEvent};
use std::borrow::Cow;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::{
    args::{Cli, CliConfig, model_completer},
    cli_helpers::resolve_model,
    client,
    output::OutputLevel,
    spinner::Spinner,
};

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
        run_interactive_chat(&client, None, &cli_config, !cli.no_streaming).await?;
        Ok(())
    }
}

/// Custom prompt for the interactive chat
#[derive(Clone)]
struct ChatPrompt {
    provider_name: String,
    multiline_mode: bool,
}

impl ChatPrompt {
    fn new(provider_name: String) -> Self {
        Self {
            provider_name,
            multiline_mode: false,
        }
    }
}

impl Prompt for ChatPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        if self.multiline_mode {
            Cow::Borrowed("... ")
        } else {
            Cow::Owned(format!("{} ", "You:".green().bold()))
        }
    }

    fn render_prompt_right(&self) -> Cow<str> {
        if self.multiline_mode {
            Cow::Borrowed("")
        } else {
            Cow::Owned(format!("[{}]", self.provider_name.blue()))
        }
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<str> {
        match edit_mode {
            PromptEditMode::Default | PromptEditMode::Emacs => Cow::Borrowed("> "),
            PromptEditMode::Vi(vi_mode) => match vi_mode {
                reedline::PromptViMode::Normal => Cow::Borrowed("< "),
                reedline::PromptViMode::Insert => Cow::Borrowed("> "),
            },
            PromptEditMode::Custom(str) => Cow::Owned(format!("({str}) ")),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed("... ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!(
            "({}reverse-search: {}) ",
            prefix, history_search.term
        ))
    }
}

/// Slash command types
#[derive(Debug, Clone)]
enum SlashCommand {
    System(String),
    Reset,
    Help,
    Quit,
    Unknown(String),
}

impl SlashCommand {
    fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        if !input.starts_with('/') {
            return None;
        }

        let parts: Vec<&str> = input[1..].splitn(2, ' ').collect();
        let command = parts[0].to_lowercase();

        Some(match command.as_str() {
            "system" => {
                if parts.len() > 1 {
                    SlashCommand::System(parts[1].to_string())
                } else {
                    SlashCommand::System(String::new())
                }
            }
            "reset" => SlashCommand::Reset,
            "help" => SlashCommand::Help,
            "quit" | "exit" => SlashCommand::Quit,
            _ => SlashCommand::Unknown(command),
        })
    }
}

/// Custom completer for slash commands
#[derive(Clone)]
struct SlashCommandCompleter {
    commands: Vec<String>,
}

impl SlashCommandCompleter {
    fn new() -> Self {
        Self {
            commands: vec![
                "/system".to_string(),
                "/reset".to_string(),
                "/help".to_string(),
                "/quit".to_string(),
                "/exit".to_string(),
            ],
        }
    }
}

impl Completer for SlashCommandCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let start = line[..pos].rfind(' ').map_or(0, |i| i + 1);
        let word = &line[start..pos];

        if !word.starts_with('/') {
            return Vec::new();
        }

        self.commands
            .iter()
            .filter(|cmd| cmd.starts_with(word))
            .map(|cmd| Suggestion {
                value: cmd.clone(),
                description: match cmd.as_str() {
                    "/system" => Some("Set system prompt".to_string()),
                    "/reset" => Some("Clear conversation history".to_string()),
                    "/help" => Some("Show available commands".to_string()),
                    "/quit" => Some("Exit chat".to_string()),
                    "/exit" => Some("Exit chat".to_string()),
                    _ => None,
                },
                style: None,
                extra: None,
                span: reedline::Span::new(start, pos),
                append_whitespace: true,
            })
            .collect()
    }
}

/// Setup keybindings for multiline and tab completion
/// Add common keybindings used by both emacs and vi modes to eliminate duplication
fn add_common_keybindings(keybindings: &mut reedline::Keybindings) {
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Enter,
        ReedlineEvent::SubmitOrNewline,
    );
    keybindings.add_binding(
        KeyModifiers::ALT | KeyModifiers::CONTROL,
        KeyCode::Enter,
        ReedlineEvent::Submit,
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

/// Setup reedline with all features
fn setup_reedline(vim_mode: bool, data_path: &PathBuf) -> Result<Reedline> {
    let completer = Box::new(SlashCommandCompleter::new());

    // Use the interactive menu to select options from the completer
    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));

    // Setup keybindings for multiline and tab completion
    let mut keybindings = default_emacs_keybindings();
    add_common_keybindings(&mut keybindings);

    let edit_mode: Box<dyn reedline::EditMode> = if vim_mode {
        let mut vi_insert_keybindings = default_vi_insert_keybindings();
        let vi_normal_keybindings = default_vi_normal_keybindings();

        // Add our common keybindings to vi insert mode
        add_common_keybindings(&mut vi_insert_keybindings);

        // Add useful emacs shortcuts to vi insert mode for hybrid experience
        vi_insert_keybindings.add_binding(
            KeyModifiers::CONTROL,
            KeyCode::Char('u'),
            ReedlineEvent::Edit(vec![EditCommand::CutFromLineStart]),
        );
        vi_insert_keybindings.add_binding(
            KeyModifiers::CONTROL,
            KeyCode::Char('k'),
            ReedlineEvent::Edit(vec![EditCommand::CutToLineEnd]),
        );
        vi_insert_keybindings.add_binding(
            KeyModifiers::CONTROL,
            KeyCode::Char('w'),
            ReedlineEvent::Edit(vec![EditCommand::CutWordLeft]),
        );
        vi_insert_keybindings.add_binding(
            KeyModifiers::CONTROL,
            KeyCode::Char('a'),
            ReedlineEvent::Edit(vec![EditCommand::MoveToLineStart { select: false }]),
        );
        vi_insert_keybindings.add_binding(
            KeyModifiers::CONTROL,
            KeyCode::Char('e'),
            ReedlineEvent::Edit(vec![EditCommand::MoveToLineEnd { select: false }]),
        );
        vi_insert_keybindings.add_binding(
            KeyModifiers::CONTROL,
            KeyCode::Char('l'),
            ReedlineEvent::ClearScreen,
        );

        Box::new(Vi::new(vi_insert_keybindings, vi_normal_keybindings))
    } else {
        Box::new(Emacs::new(keybindings))
    };

    let history = Box::new(
        FileBackedHistory::with_file(5, data_path.join("history.txt"))
            .expect("Error configuring history with file"),
    );

    let line_editor = Reedline::create()
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_hinter(Box::new(DefaultHinter::default()))
        .with_history(history)
        .with_validator(Box::new(DefaultValidator))
        .with_edit_mode(edit_mode);

    Ok(line_editor)
}

/// Handle slash commands
async fn handle_slash_command(
    command: SlashCommand,
    conversation: &mut Vec<(ChatRole, String)>,
    _client: &SimpleLlmClient,
) -> Result<bool> {
    match command {
        SlashCommand::System(msg) => {
            if msg.is_empty() {
                println!("{}", "Usage: /system <message>".yellow());
                return Ok(false);
            }

            // Remove existing system message if present
            conversation.retain(|(role, _)| *role != ChatRole::System);

            // Add new system message at the beginning
            conversation.insert(0, (ChatRole::System, msg.clone()));

            println!(
                "{} {} {} {}",
                "System".green().bold(),
                "prompt".green(),
                "updated:".green(),
                msg.dimmed()
            );
        }
        SlashCommand::Reset => {
            conversation.clear();
            println!("{}", "Conversation reset.".green());
        }
        SlashCommand::Help => {
            println!("{}", "Available commands:".green().bold());
            println!("  {} - Set system prompt", "/system <message>".yellow());
            println!("  {} - Clear conversation history", "/reset".yellow());
            println!("  {} - Show this help", "/help".yellow());
            println!("  {} - Exit chat", "/quit or /exit".yellow());
        }
        SlashCommand::Quit => {
            return Ok(true);
        }
        SlashCommand::Unknown(cmd) => {
            println!(
                "{} {}. Type {} for help.",
                "Unknown command:".red(),
                cmd.yellow(),
                "/help".yellow()
            );
        }
    }
    Ok(false)
}

/// Run enhanced interactive chat with reedline
pub async fn run_interactive_chat(
    client: &SimpleLlmClient,
    initial_system: Option<&str>,
    config: &CliConfig,
    streaming: bool,
) -> Result<()> {
    println!(
        "{} {} {}",
        "Interactive chat with".green(),
        client.provider_name().blue().bold(),
        "(Ctrl+C to exit)".dimmed()
    );
    println!(
        "{} Type {} for available commands.\n",
        "Tip:".green(),
        "/help".yellow()
    );

    let mut conversation = Vec::new();
    let mut line_editor = setup_reedline(config.config.vi_mode, &config.data_base_path)?;
    let prompt = ChatPrompt::new(client.provider_name().to_string());

    // Track Ctrl+C presses for double-press exit
    let mut last_ctrl_c: Option<Instant> = None;
    const DOUBLE_CTRL_C_TIMEOUT: Duration = Duration::from_secs(2);

    // Add system prompt if provided
    if let Some(system) = initial_system {
        conversation.push((ChatRole::System, system.to_string()));
        println!("{} {}\n", "System:".green().bold(), system.dimmed());
    }

    loop {
        let sig = line_editor.read_line(&prompt)?;
        match sig {
            Signal::Success(input) => {
                // Reset Ctrl+C counter on successful input
                last_ctrl_c = None;

                let input = input.trim();
                if input.is_empty() {
                    continue;
                }

                // Check for slash commands
                if let Some(command) = SlashCommand::parse(input) {
                    if handle_slash_command(command, &mut conversation, client).await? {
                        break; // Quit command
                    }
                    continue;
                }

                // Regular chat message
                conversation.push((ChatRole::User, input.to_string()));

                if streaming {
                    // Show spinner while waiting for first token
                    let spinner = Spinner::new("Assistant:");
                    spinner.start().await;

                    // Small delay to ensure spinner starts
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                    // Build ChatRequest with full conversation + current user message
                    let mut builder = ChatRequestBuilder::new().stream(true);
                    for (role, content) in &conversation {
                        builder = builder.add_message(role.clone(), content);
                    }

                    let request = builder.build();

                    match client.stream_chat_raw(request).await {
                        Ok(mut stream) => {
                            let mut full_response = String::new();
                            let mut first_token = true;

                            while let Some(evt) = stream.next().await {
                                match evt {
                                    Ok(ChatStreamEvent::Token(tok)) => {
                                        if first_token {
                                            // Stop spinner and show Assistant label on first token
                                            spinner.stop_and_replace(&format!(
                                                "{} ",
                                                "Assistant:".blue().bold()
                                            ));
                                            first_token = false;
                                        }
                                        full_response.push_str(&tok);
                                        print!("{tok}");
                                        io::stdout().flush()?;
                                    }
                                    Ok(ChatStreamEvent::Done) => {
                                        println!();
                                        conversation.push((ChatRole::Assistant, full_response));
                                        break;
                                    }
                                    Ok(ChatStreamEvent::Error(msg)) => {
                                        spinner.stop_and_replace(&format!(
                                            "{} {}\n",
                                            "Error:".red().bold(),
                                            msg
                                        ));
                                        break;
                                    }
                                    Err(err) => {
                                        spinner.stop_and_replace(&format!(
                                            "{} {}\n",
                                            "Error:".red().bold(),
                                            err
                                        ));
                                        break;
                                    }
                                }
                            }

                            // Ensure spinner is stopped if no tokens were received
                            if first_token {
                                spinner.stop_and_replace(&format!(
                                    "{} {}\n",
                                    "Assistant:".blue().bold(),
                                    "(No response received)".dimmed()
                                ));
                            }
                        }
                        Err(e) => {
                            spinner.stop_and_replace(&format!("{} {}\n", "Error:".red().bold(), e));
                        }
                    }
                } else {
                    // Non-streaming with spinner
                    let spinner = Spinner::new("Assistant:");
                    spinner.start().await;

                    // Small delay to ensure spinner starts
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                    match client.conversation(conversation.clone()).await {
                        Ok(response) => {
                            spinner.stop_and_replace(&format!(
                                "{} {}\n",
                                "Assistant:".blue().bold(),
                                response
                            ));
                            conversation.push((ChatRole::Assistant, response));
                        }
                        Err(e) => {
                            spinner.stop_and_replace(&format!("{} {}\n", "Error:".red().bold(), e));
                        }
                    }
                }
            }
            Signal::CtrlC => {
                let now = Instant::now();

                if let Some(last_time) = last_ctrl_c {
                    // Check if this is a double Ctrl+C within timeout
                    if now.duration_since(last_time) <= DOUBLE_CTRL_C_TIMEOUT {
                        println!("\n{}", "Goodbye!".green());
                        break;
                    }
                }

                // First Ctrl+C or timeout exceeded - show instruction message
                last_ctrl_c = Some(now);
                println!(
                    "\n{}",
                    "(To exit, press Ctrl+C again or Ctrl+D or enter \"/quit\")".dimmed()
                );
            }
            Signal::CtrlD => {
                println!("\n{}", "Goodbye!".green());
                break;
            }
        }
    }

    Ok(())
}
