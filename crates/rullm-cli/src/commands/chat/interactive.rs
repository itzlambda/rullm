use crate::args::CliConfig;
use anyhow::Result;
use owo_colors::OwoColorize;
use reedline::{EditCommand, Signal};
use rullm_core::SimpleLlm;
use rullm_core::simple::SimpleLlmClient;
use std::time::{Duration, Instant};

pub async fn run_interactive_chat(
    client: &SimpleLlmClient,
    initial_system: Option<&str>,
    config: &CliConfig,
    streaming: bool,
) -> Result<()> {
    use super::slash_command::{HandleCommandResult, SlashCommand, handle_slash_command};
    use super::{ChatPrompt, setup_reedline};

    println!(
        "{} {}/{}",
        "Interactive chat with".green(),
        client.provider_name().blue().bold(),
        client.model_name().blue().bold(),
    );
    println!(
        "{} Type {} for available commands.",
        "Tip:".green(),
        "help".yellow()
    );
    println!(
        "{} Type {} or {} to exit.",
        "Tip:".green(),
        "exit".yellow(),
        "quit".yellow()
    );
    println!();

    let mut conversation = Vec::new();
    let mut line_editor = setup_reedline(config.config.vi_mode, &config.data_base_path)?;
    let prompt = ChatPrompt::new();

    // Track Ctrl+C presses for double-press exit
    let mut last_ctrl_c: Option<Instant> = None;
    const DOUBLE_CTRL_C_TIMEOUT: Duration = Duration::from_secs(2);

    // Add system prompt if provided
    if let Some(system) = initial_system {
        conversation.push((rullm_core::types::ChatRole::System, system.to_string()));
        println!("{} {}\n", "System:".green().bold(), system.dimmed());
    }

    // Helper function to DRY up message sending logic
    async fn process_user_message(
        input: &str,
        conversation: &mut Vec<(rullm_core::types::ChatRole, String)>,
        client: &SimpleLlmClient,
        streaming: bool,
    ) -> Result<()> {
        use crate::spinner::Spinner;
        use futures::StreamExt;
        use owo_colors::OwoColorize;
        use rullm_core::types::{ChatRequestBuilder, ChatRole, ChatStreamEvent};
        use std::io::{self, Write};
        use tokio::time;

        conversation.push((ChatRole::User, input.to_string()));
        if streaming {
            let spinner = Spinner::new("Assistant:");
            spinner.start().await;
            time::sleep(time::Duration::from_millis(10)).await;
            let mut builder = ChatRequestBuilder::new().stream(true);
            for (role, content) in &*conversation {
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
            let spinner = Spinner::new("Assistant:");
            spinner.start().await;
            time::sleep(time::Duration::from_millis(10)).await;
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
        Ok(())
    }

    loop {
        let sig = line_editor.read_line(&prompt)?;
        match sig {
            Signal::Success(input) => {
                last_ctrl_c = None;
                let input = input.trim();
                if input.is_empty() {
                    continue;
                }
                if let Some(command) = SlashCommand::parse(input) {
                    let result = handle_slash_command(command, &mut conversation, client).await?;
                    match result {
                        HandleCommandResult::Quit => {
                            break;
                        }
                        HandleCommandResult::Edit(edited_input) => {
                            line_editor
                                .run_edit_commands(&[EditCommand::InsertString(edited_input)]);
                        }
                        HandleCommandResult::NoOp => {}
                    }
                    continue;
                }
                process_user_message(input, &mut conversation, client, streaming).await?;
            }
            Signal::CtrlC => {
                let now = Instant::now();
                if let Some(last_time) = last_ctrl_c {
                    if now.duration_since(last_time) <= DOUBLE_CTRL_C_TIMEOUT {
                        println!("{}", "Goodbye!".green());
                        break;
                    }
                }
                last_ctrl_c = Some(now);
                println!(
                    "{}",
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
