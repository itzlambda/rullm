//! Minimal CLI helper functions for reuse by other frontends
//!
//! These functions provide basic model resolution logic that can be used
//! by different CLI implementations or frontends without depending on
//! the full rullm-cli crate.

use anyhow::Result;

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
                "Model is required. Use --model in format 'provider/model_name' (e.g., openai/gpt-4o) or set a default_model in config"
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
                "Model is required for direct queries. Use --model in format 'provider/model_name' (e.g., openai/gpt-4o) or set a default_model in config"
            )
        })
}
