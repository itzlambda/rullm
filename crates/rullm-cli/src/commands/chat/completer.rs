use reedline::{Completer, Suggestion};

#[derive(Clone)]
pub struct SlashCommandCompleter {
    pub commands: Vec<String>,
}

impl SlashCommandCompleter {
    pub(crate) fn new() -> Self {
        Self {
            commands: vec![
                "/system".to_string(),
                "/clear".to_string(),
                "/help".to_string(),
                "/quit".to_string(),
                "/exit".to_string(),
                "/edit".to_string(),
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
                    "/clear" => Some("Clear conversation history".to_string()),
                    "/help" => Some("Show available commands".to_string()),
                    "/quit" => Some("Exit chat".to_string()),
                    "/exit" => Some("Exit chat".to_string()),
                    "/edit" => Some("Edit message in $EDITOR".to_string()),
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
