use anyhow::Result;
use owo_colors::OwoColorize;
use rullm_core::simple::SimpleLlmClient;
use rullm_core::types::ChatRole;

#[derive(Debug, Clone)]
pub enum SlashCommand {
    System(String),
    Clear,
    Help,
    Quit,
    Edit,
    Unknown(String),
}

impl SlashCommand {
    pub(crate) fn parse(input: &str) -> Option<Self> {
        // special case for some shortcuts
        if input.len() <= 5 {
            match input.to_lowercase().as_str() {
                "quit" | "exit" => return Some(SlashCommand::Quit),
                "help" => return Some(SlashCommand::Help),
                "clear" => return Some(SlashCommand::Clear),
                "edit" => return Some(SlashCommand::Edit),
                _ => {}
            }
        }

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
            "clear" => SlashCommand::Clear,
            "help" => SlashCommand::Help,
            "quit" | "exit" => SlashCommand::Quit,
            "edit" => SlashCommand::Edit,
            _ => SlashCommand::Unknown(command),
        })
    }
}

pub enum HandleCommandResult {
    NoOp,
    Quit,
    Edit(String),
}

pub async fn handle_slash_command(
    command: SlashCommand,
    conversation: &mut Vec<(ChatRole, String)>,
    _client: &SimpleLlmClient,
) -> Result<HandleCommandResult> {
    match command {
        SlashCommand::System(msg) => {
            if msg.is_empty() {
                println!("{}", "Usage: /system <message>".yellow());
                return Ok(HandleCommandResult::NoOp);
            }
            conversation.retain(|(role, _)| *role != ChatRole::System);
            conversation.insert(0, (ChatRole::System, msg.clone()));
            println!(
                "{} {} {} {}",
                "System".green().bold(),
                "prompt".green(),
                "updated:".green(),
                msg.dimmed()
            );
            Ok(HandleCommandResult::NoOp)
        }
        SlashCommand::Clear => {
            conversation.clear();
            println!("{}", "Conversation cleared.".green());
            Ok(HandleCommandResult::NoOp)
        }
        SlashCommand::Help => {
            println!(
                "{} {}",
                "TIP:".green(),
                "Some commands can be used without the leading '/'".dimmed()
            );
            println!("{}", "Available commands:".green().bold());
            println!("  {} - Set system prompt", "/system <message>".yellow());
            println!(
                "  {} - Clear conversation history",
                "/clear (clear)".yellow()
            );
            println!("  {} - Show this help", "/help (help)".yellow());
            println!(
                "  {} - Edit next message in $EDITOR",
                "/edit (edit)".yellow()
            );
            println!("  {} - Exit chat", "/quit or /exit (quit or exit)".yellow());
            Ok(HandleCommandResult::NoOp)
        }
        SlashCommand::Quit => Ok(HandleCommandResult::Quit),
        SlashCommand::Edit => {
            use std::io::Read;
            use std::process::Command;
            use tempfile::NamedTempFile;

            let tmp = NamedTempFile::new()?;
            // Optionally, pre-fill with last user message or blank
            // std::fs::write(tmp.path(), "")?;

            let editor = super::get_preferred_editor();
            let status = Command::new(&editor).arg(tmp.path()).status();

            match status {
                Ok(status) if status.success() => {
                    let mut file = tmp.reopen()?;
                    let mut contents = String::new();
                    file.read_to_string(&mut contents)?;
                    let contents = contents.trim().to_string();
                    if contents.is_empty() {
                        println!("{}", "No input provided in editor.".yellow());
                        Ok(HandleCommandResult::NoOp)
                    } else {
                        Ok(HandleCommandResult::Edit(contents))
                    }
                }
                Ok(_) => {
                    println!("{}", "Editor exited with error".red());
                    Ok(HandleCommandResult::NoOp)
                }
                Err(e) => {
                    println!("{} {}", "Failed to launch editor:".red(), e);
                    println!(
                        "{} {}",
                        "Try setting $EDITOR environment variable or installing neovim".red(),
                        e
                    );
                    Ok(HandleCommandResult::NoOp)
                }
            }
        }
        SlashCommand::Unknown(cmd) => {
            println!(
                "{} {}. Type {} for help.",
                "Unknown command:".red(),
                cmd.yellow(),
                "/help".yellow()
            );
            Ok(HandleCommandResult::NoOp)
        }
    }
}
