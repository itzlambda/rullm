//! Output formatting and logging utilities
//!
//! This module provides colored output functions and logging initialization
//! for the CLI application following modern CLI conventions.

use owo_colors::OwoColorize;
use std::env;

/// Output level for controlling what gets displayed
#[derive(Debug, Clone, Copy)]
pub enum OutputLevel {
    /// Show all output (normal mode)
    Normal,
    /// Show only errors (quiet mode)
    Quiet,
    /// Show extra debug information (verbose mode)
    Verbose,
}

impl OutputLevel {
    /// Check if user-facing messages should be shown (excludes errors/hints which always show)
    pub fn show_user(&self) -> bool {
        matches!(self, Self::Normal | Self::Verbose)
    }
}

/// Check if colored output should be disabled
fn colors_disabled() -> bool {
    // Check multiple conditions for color disabling
    env::var("NO_COLOR").is_ok()
        || env::var("TERM").is_ok_and(|t| t == "dumb")
        || !atty::is(atty::Stream::Stderr) // Use stderr since we're using eprintln!
}

/// Generic helper to print colored messages, eliminating duplication
fn print_colored<T>(msg: &str, styled_msg: T, output_level: OutputLevel, always_show: bool)
where
    T: std::fmt::Display,
{
    if always_show || output_level.show_user() {
        if !colors_disabled() {
            eprintln!("{styled_msg}");
        } else {
            eprintln!("{msg}");
        }
    }
}

/// Print a heading with bold formatting
pub fn heading(msg: &str, output_level: OutputLevel) {
    print_colored(msg, msg.bold(), output_level, false);
}

/// Print a note message with default formatting (no prefix)
pub fn note(msg: &str, output_level: OutputLevel) {
    if output_level.show_user() {
        eprintln!("{msg}");
    }
}

/// Print a success message with green color (no prefix)
pub fn success(msg: &str, output_level: OutputLevel) {
    print_colored(msg, msg.green(), output_level, false);
}

/// Print a progress message with cyan color and ellipsis
pub fn progress(msg: &str, output_level: OutputLevel) {
    if output_level.show_user() {
        let progress_msg = if msg.ends_with("...") || msg.ends_with("…") {
            msg.to_string()
        } else {
            format!("{msg}…")
        };

        print_colored(&progress_msg, progress_msg.cyan(), output_level, false);
    }
}

/// Print a warning message with "Warning:" prefix in yellow
pub fn warning(msg: &str, output_level: OutputLevel) {
    if output_level.show_user() {
        let warning_msg = format!("Warning: {msg}");
        if !colors_disabled() {
            eprintln!("{} {}", "Warning:".yellow().bold(), msg.yellow());
        } else {
            eprintln!("{warning_msg}");
        }
    }
}

/// Print an error message with "Error:" prefix in red (always shown)
pub fn error(msg: &str, _output_level: OutputLevel) {
    // Always show errors, even in quiet mode
    let error_msg = format!("Error: {msg}");
    if !colors_disabled() {
        eprintln!("{} {}", "Error:".red().bold(), msg.red());
    } else {
        eprintln!("{error_msg}");
    }
}

/// Print a hint message with "Hint:" prefix in blue (always shown)
pub fn hint(msg: &str, _output_level: OutputLevel) {
    // Always show hints, even in quiet mode
    let hint_msg = format!("Hint: {msg}");
    if !colors_disabled() {
        eprintln!("{} {}", "Hint:".blue().bold(), msg.blue());
    } else {
        eprintln!("{hint_msg}");
    }
}

/// Print an error with suggested action
pub fn error_with_suggestion(msg: &str, suggestion: &str, output_level: OutputLevel) {
    error(msg, output_level);
    hint(suggestion, output_level);
}

/// Format a provider name with colors
pub fn format_provider(provider: &str) -> String {
    if colors_disabled() {
        provider.to_string()
    } else {
        provider.magenta().bold().to_string()
    }
}

/// Format a model name with colors
pub fn format_model(model: &str) -> String {
    if colors_disabled() {
        model.to_string()
    } else {
        model.cyan().to_string()
    }
}

/// Format a command or option with colors
pub fn format_command(cmd: &str) -> String {
    if colors_disabled() {
        format!("`{cmd}`")
    } else {
        format!("`{}`", cmd.yellow().bold())
    }
}
