//! Minimal CLI helper functions for reuse by other frontends
//!
//! These functions provide basic model resolution logic that can be used
//! by different CLI implementations or frontends without depending on
//! the full rullm-cli crate.

use anyhow::Result;
use std::io::Read;

/// Helper function to resolve model priority: global CLI model, command-specific model, or default
///
/// This function implements the standard model resolution logic:
/// 1. Use global CLI model if provided
/// 2. Fall back to command-specific model if provided  
/// 3. Fall back to default model from config
/// 4. Error if none are available
pub fn resolve_model(
    global_model: &Option<String>,
    cmd_model: &Option<String>,
    default_model: &Option<String>,
) -> Result<String> {
    global_model
        .clone()
        .or_else(|| cmd_model.clone())
        .or_else(|| default_model.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Model is required. Use --model in format 'provider:model_name' (e.g., openai:gpt-4o) or set a default_model in config"
            )
        })
}

/// Helper function to resolve model for direct queries (global or default only)
///
/// This is a simpler version of resolve_model that only considers global
/// and default models, used for direct query scenarios where there's no
/// command-specific model option.
pub fn resolve_direct_query_model(
    global_model: &Option<String>,
    default_model: &Option<String>,
) -> Result<String> {
    global_model
        .clone()
        .or_else(|| default_model.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Model is required for direct queries. Use --model in format 'provider:model_name' (e.g., openai:gpt-4o) or set a default_model in config"
            )
        })
}

/// Merges piped stdin and an optional query argument into a single query string.
///
/// - If stdin is piped and contains data, reads it.
///   - If a query argument is also provided, appends it to the stdin content (with a newline if needed).
///   - If only stdin is present, returns its content.
/// - If only a query argument is present, returns it.
/// - If neither is present, returns None.
///
/// This is used to support CLI usage like:
///   cat foo.py | rullm 'explain this'   // stdin + arg
///   cat foo.py | rullm                  // stdin only
///   rullm 'explain this'                // arg only
pub fn merge_stdin_and_query(query: Option<String>) -> Option<String> {
    let mut stdin_buf = String::new();
    let stdin_piped = !atty::is(atty::Stream::Stdin)
        && std::io::stdin().read_to_string(&mut stdin_buf).is_ok()
        && !stdin_buf.trim().is_empty();

    match (stdin_piped, query) {
        (true, Some(arg_query)) => {
            // Both stdin and query arg: append arg to stdin
            let combined = if stdin_buf.ends_with('\n') {
                format!("{stdin_buf}{arg_query}")
            } else {
                format!("{stdin_buf}\n{arg_query}")
            };
            Some(combined)
        }
        (true, None) => {
            // Only stdin
            Some(stdin_buf)
        }
        (false, Some(arg_query)) => {
            // Only query arg
            Some(arg_query)
        }
        (false, None) => {
            // Neither
            None
        }
    }
}
