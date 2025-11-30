//! Complete Google Gemini API types
//!
//! This module contains comprehensive type definitions for the Google Gemini API.

use serde::{Deserialize, Serialize};

/// Generate content request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentRequest {
    /// Contents of the conversation
    pub contents: Vec<Content>,

    /// System instruction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,

    /// Generation configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,

    /// Safety settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,

    /// Tool configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Tool configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
}

/// Content block with role and parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Role of the content author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>, // "user" or "model"

    /// Parts of the content
    pub parts: Vec<Part>,
}

/// A part of the content (text, inline data, function call, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    /// Text part
    Text { text: String },
    /// Inline data (image, etc.)
    InlineData { inline_data: InlineData },
    /// Function call
    FunctionCall { function_call: FunctionCall },
    /// Function response
    FunctionResponse { function_response: FunctionResponse },
}

/// Inline data (base64-encoded image, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineData {
    /// MIME type
    pub mime_type: String,
    /// Base64-encoded data
    pub data: String,
}

/// Function call from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Name of the function
    pub name: String,
    /// Arguments as JSON
    pub args: serde_json::Value,
}

/// Function response to the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    /// Name of the function that was called
    pub name: String,
    /// Response from the function
    pub response: serde_json::Value,
}

/// Generation configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Temperature (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Max output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,

    /// Top-p (nucleus sampling)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Top-k
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    /// Response MIME type (for JSON mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,

    /// Response schema (for structured output)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,
}

/// Safety setting
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct SafetySetting {
    /// Safety category
    pub category: SafetyCategory,
    /// Threshold for blocking
    pub threshold: SafetyThreshold,
}

/// Safety categories
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SafetyCategory {
    HarmCategoryHarassment,
    HarmCategoryHateSpeech,
    HarmCategorySexuallyExplicit,
    HarmCategoryDangerousContent,
}

/// Safety thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SafetyThreshold {
    BlockNone,
    BlockOnlyHigh,
    BlockMediumAndAbove,
    BlockLowAndAbove,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// Function declarations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_declarations: Option<Vec<FunctionDeclaration>>,
}

/// Function declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    /// Name of the function
    pub name: String,
    /// Description
    pub description: String,
    /// Parameters as JSON schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    /// Function calling config
    pub function_calling_config: FunctionCallingConfig,
}

/// Function calling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallingConfig {
    /// Mode (AUTO, ANY, NONE)
    pub mode: String,
    /// Allowed function names (if mode is ANY)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_function_names: Option<Vec<String>>,
}

/// Generate content response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    /// Candidates
    pub candidates: Vec<Candidate>,

    /// Prompt feedback
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_feedback: Option<PromptFeedback>,

    /// Usage metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
}

/// A candidate response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// Content of the candidate
    pub content: Content,

    /// Finish reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,

    /// Safety ratings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<SafetyRating>>,

    /// Citation metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation_metadata: Option<CitationMetadata>,

    /// Token count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<u32>,

    /// Index
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
}

/// Finish reasons
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinishReason {
    Stop,
    MaxTokens,
    Safety,
    Recitation,
    Other,
}

/// Safety rating
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRating {
    /// Category
    pub category: SafetyCategory,
    /// Probability
    pub probability: String,
    /// Blocked
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<bool>,
}

/// Citation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationMetadata {
    /// Citation sources
    pub citation_sources: Vec<CitationSource>,
}

/// Citation source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationSource {
    /// Start index
    pub start_index: u32,
    /// End index
    pub end_index: u32,
    /// URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// License
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

/// Prompt feedback (for blocked requests)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptFeedback {
    /// Block reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<String>,

    /// Safety ratings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

/// Usage metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    /// Prompt token count
    pub prompt_token_count: u32,
    /// Candidates token count
    pub candidates_token_count: u32,
    /// Total token count
    pub total_token_count: u32,
}

// Builder methods
impl GenerateContentRequest {
    pub fn new(contents: Vec<Content>) -> Self {
        Self {
            contents,
            system_instruction: None,
            generation_config: None,
            safety_settings: None,
            tools: None,
            tool_config: None,
        }
    }

    pub fn with_system(mut self, system: String) -> Self {
        self.system_instruction = Some(Content {
            role: None,
            parts: vec![Part::Text { text: system }],
        });
        self
    }

    pub fn with_generation_config(mut self, config: GenerationConfig) -> Self {
        self.generation_config = Some(config);
        self
    }
}

impl Content {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Some("user".to_string()),
            parts: vec![Part::Text { text: text.into() }],
        }
    }

    pub fn model(text: impl Into<String>) -> Self {
        Self {
            role: Some("model".to_string()),
            parts: vec![Part::Text { text: text.into() }],
        }
    }

    pub fn user_with_parts(parts: Vec<Part>) -> Self {
        Self {
            role: Some("user".to_string()),
            parts,
        }
    }

    pub fn model_with_parts(parts: Vec<Part>) -> Self {
        Self {
            role: Some("model".to_string()),
            parts,
        }
    }
}

impl Part {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    pub fn image(mime_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::InlineData {
            inline_data: InlineData {
                mime_type: mime_type.into(),
                data: data.into(),
            },
        }
    }
}
