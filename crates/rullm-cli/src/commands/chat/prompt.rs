use owo_colors::OwoColorize;
use reedline::{Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus};
use std::borrow::Cow;

#[derive(Clone)]
pub struct ChatPrompt {
    pub multiline_mode: bool,
}

impl ChatPrompt {
    pub(crate) fn new() -> Self {
        Self {
            multiline_mode: false,
        }
    }
}

impl Prompt for ChatPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        if self.multiline_mode {
            Cow::Borrowed("... ")
        } else {
            Cow::Owned(format!("{} ", "You:".green().bold()))
        }
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<'_, str> {
        match edit_mode {
            PromptEditMode::Default | PromptEditMode::Emacs => Cow::Borrowed("> "),
            PromptEditMode::Vi(vi_mode) => match vi_mode {
                reedline::PromptViMode::Normal => Cow::Borrowed("< "),
                reedline::PromptViMode::Insert => Cow::Borrowed("> "),
            },
            PromptEditMode::Custom(str) => Cow::Owned(format!("({str}) ")),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("... ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
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
