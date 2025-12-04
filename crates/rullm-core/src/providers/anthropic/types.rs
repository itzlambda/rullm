//! Complete Anthropic Messages API types
//!
//! This module contains comprehensive type definitions for the Anthropic Messages API,
//! including all parameters and features supported by Claude models.

use serde::{Deserialize, Serialize};

/// Messages API request with all Anthropic parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesRequest {
    /// The model to use (e.g., "claude-3-opus-20240229")
    pub model: String,

    /// Input messages for the conversation
    pub messages: Vec<Message>,

    /// The maximum number of tokens to generate
    pub max_tokens: u32,

    /// System prompt(s) to guide the model's behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,

    /// Metadata about the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// Custom sequences that will cause the model to stop generating
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Whether to incrementally stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Amount of randomness injected into the response (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Use nucleus sampling (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Only sample from the top K options for each subsequent token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    /// Definitions of tools that the model may use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// How the model should use the provided tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message sender
    pub role: Role,

    /// The content of the message
    pub content: MessageContent,
}

/// Role of the message sender
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User message
    User,
    /// Assistant message (model response)
    Assistant,
}

/// Message content can be text or array of content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content
    Text(String),
    /// Array of content blocks (for multimodal inputs, tool use, etc.)
    Blocks(Vec<ContentBlock>),
}

/// A block of content within a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text { text: String },
    /// Image content
    Image { source: ImageSource },
    /// Tool use (request to call a tool)
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool result (response from a tool)
    ToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

/// Image source (base64 or URL)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64-encoded image
    Base64 { media_type: String, data: String },
}

/// System prompt can be a string or array of text blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemPrompt {
    /// Simple text system prompt
    Text(String),
    /// Array of system text blocks
    Blocks(Vec<SystemBlock>),
}

/// A text block in the system prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemBlock {
    #[serde(rename = "type")]
    pub block_type: String, // "text"
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

/// Cache control for prompt caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub cache_type: String, // "ephemeral"
}

/// Request metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// An external identifier for the user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Name of the tool
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// JSON schema for the tool's input
    pub input_schema: serde_json::Value,
}

/// Tool choice configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Let the model decide
    Auto {
        #[serde(rename = "type")]
        choice_type: String, // "auto"
    },
    /// Model must use a tool
    Any {
        #[serde(rename = "type")]
        choice_type: String, // "any"
    },
    /// Force a specific tool
    Tool {
        #[serde(rename = "type")]
        choice_type: String, // "tool"
        name: String,
    },
}

/// Messages API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesResponse {
    /// Unique object identifier
    pub id: String,
    /// Object type (always "message")
    #[serde(rename = "type")]
    pub response_type: String,
    /// Conversational role of the generated message
    pub role: Role,
    /// Content blocks in the response
    pub content: Vec<ContentBlock>,
    /// The model that handled the request
    pub model: String,
    /// The reason we stopped generating
    pub stop_reason: Option<StopReason>,
    /// Which custom stop sequence was generated (if any)
    pub stop_sequence: Option<String>,
    /// Token usage information
    pub usage: Usage,
}

/// Reason for stopping generation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Natural end of message
    EndTurn,
    /// Hit a custom stop sequence
    StopSequence,
    /// Reached max_tokens
    MaxTokens,
    /// Model wants to use a tool
    ToolUse,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of input tokens
    pub input_tokens: u32,
    /// Number of output tokens
    pub output_tokens: u32,
    /// Number of tokens read from cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    /// Number of tokens used to create cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

/// Streaming event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Start of message
    MessageStart { message: MessageStartData },
    /// Start of content block
    ContentBlockStart {
        index: u32,
        content_block: ContentBlockStart,
    },
    /// Incremental content
    ContentBlockDelta { index: u32, delta: Delta },
    /// End of content block
    ContentBlockStop { index: u32 },
    /// End of message
    MessageDelta {
        delta: MessageDeltaData,
        usage: Usage,
    },
    /// End of stream
    MessageStop,
    /// Ping event (keep-alive)
    Ping,
    /// Error event
    Error { error: ErrorData },
}

/// Message start data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStartData {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: Role,
    pub content: Vec<serde_json::Value>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

/// Content block start
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockStart {
    Text { text: String },
    ToolUse { id: String, name: String },
}

/// Delta (incremental change)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Delta {
    /// Text delta
    TextDelta { text: String },
    /// Tool input delta
    InputJsonDelta { partial_json: String },
}

/// Message delta data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeltaData {
    pub stop_reason: Option<StopReason>,
    pub stop_sequence: Option<String>,
}

/// Error data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

// Builder for MessagesRequest
impl MessagesRequest {
    pub fn new(model: impl Into<String>, messages: Vec<Message>, max_tokens: u32) -> Self {
        Self {
            model: model.into(),
            messages,
            max_tokens,
            system: None,
            metadata: None,
            stop_sequences: None,
            stream: None,
            temperature: None,
            top_p: None,
            top_k: None,
            tools: None,
            tool_choice: None,
        }
    }

    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(SystemPrompt::Text(system.into()));
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    pub fn with_stop_sequences(mut self, stop_sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(stop_sequences);
        self
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }
}

// Helper methods for creating messages
impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(content.into()),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(content.into()),
        }
    }

    pub fn user_with_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Blocks(blocks),
        }
    }

    pub fn assistant_with_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Blocks(blocks),
        }
    }
}

impl ContentBlock {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    pub fn image_base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource::Base64 {
                media_type: media_type.into(),
                data: data.into(),
            },
        }
    }
}
