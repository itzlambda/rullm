//! Types for the Anthropic Messages API
//!
//! This module contains comprehensive type definitions for requests, responses,
//! content blocks, tools, and streaming events.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

// =============================================================================
// Request Types
// =============================================================================

/// Messages API request with all Anthropic parameters
#[derive(Debug, Clone, Serialize)]
pub struct MessagesRequest {
    /// The model to use (e.g., "claude-3-5-sonnet-20241022")
    pub model: Arc<str>,

    /// The maximum number of tokens to generate
    pub max_tokens: u32,

    /// Input messages for the conversation
    pub messages: Vec<Message>,

    /// System prompt(s) to guide the model's behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemContent>,

    /// Metadata about the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// Custom sequences that will cause the model to stop generating
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<Arc<str>>>,

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

    /// Configuration for extended thinking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,

    /// Service tier to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
}

/// Builder for constructing MessagesRequest
#[derive(Debug, Clone)]
pub struct MessagesRequestBuilder {
    model: Arc<str>,
    max_tokens: u32,
    messages: Vec<Message>,
    system: Option<SystemContent>,
    metadata: Option<Metadata>,
    stop_sequences: Option<Vec<Arc<str>>>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    tools: Option<Vec<Tool>>,
    tool_choice: Option<ToolChoice>,
    thinking: Option<ThinkingConfig>,
    service_tier: Option<ServiceTier>,
}

impl MessagesRequestBuilder {
    /// Create a new builder with required fields
    pub fn new(model: impl Into<Arc<str>>, max_tokens: u32) -> Self {
        Self {
            model: model.into(),
            max_tokens,
            messages: Vec::new(),
            system: None,
            metadata: None,
            stop_sequences: None,
            temperature: None,
            top_p: None,
            top_k: None,
            tools: None,
            tool_choice: None,
            thinking: None,
            service_tier: None,
        }
    }

    /// Add a message to the conversation
    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add multiple messages to the conversation
    pub fn messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Set a simple text system prompt
    pub fn system(mut self, content: impl Into<Arc<str>>) -> Self {
        self.system = Some(SystemContent::Text(content.into()));
        self
    }

    /// Set system prompt with blocks (for cache control)
    pub fn system_blocks(mut self, blocks: Vec<SystemBlock>) -> Self {
        self.system = Some(SystemContent::Blocks(blocks));
        self
    }

    /// Set metadata
    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set custom stop sequences
    pub fn stop_sequences(
        mut self,
        sequences: impl IntoIterator<Item = impl Into<Arc<str>>>,
    ) -> Self {
        self.stop_sequences = Some(sequences.into_iter().map(Into::into).collect());
        self
    }

    /// Set temperature (0.0 to 1.0)
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set top_p for nucleus sampling
    pub fn top_p(mut self, p: f32) -> Self {
        self.top_p = Some(p);
        self
    }

    /// Set top_k sampling
    pub fn top_k(mut self, k: u32) -> Self {
        self.top_k = Some(k);
        self
    }

    /// Set available tools
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set tool choice mode
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    /// Enable extended thinking
    pub fn thinking(mut self, config: ThinkingConfig) -> Self {
        self.thinking = Some(config);
        self
    }

    /// Set service tier
    pub fn service_tier(mut self, tier: ServiceTier) -> Self {
        self.service_tier = Some(tier);
        self
    }

    /// Build the request
    pub fn build(self) -> MessagesRequest {
        MessagesRequest {
            model: self.model,
            max_tokens: self.max_tokens,
            messages: self.messages,
            system: self.system,
            metadata: self.metadata,
            stop_sequences: self.stop_sequences,
            stream: None,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            tools: self.tools,
            tool_choice: self.tool_choice,
            thinking: self.thinking,
            service_tier: self.service_tier,
        }
    }
}

impl MessagesRequest {
    /// Create a new builder
    pub fn builder(model: impl Into<Arc<str>>, max_tokens: u32) -> MessagesRequestBuilder {
        MessagesRequestBuilder::new(model, max_tokens)
    }
}

// =============================================================================
// Message Types
// =============================================================================

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message sender
    pub role: Role,

    /// The content of the message
    pub content: MessageContent,
}

impl Message {
    /// Create a user message with text content
    pub fn user(content: impl Into<Arc<str>>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create an assistant message with text content
    pub fn assistant(content: impl Into<Arc<str>>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a user message with content blocks
    pub fn user_with_blocks(blocks: Vec<ContentBlockParam>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Blocks(blocks),
        }
    }

    /// Create an assistant message with content blocks
    pub fn assistant_with_blocks(blocks: Vec<ContentBlockParam>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Blocks(blocks),
        }
    }
}

/// Role of the message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    Text(Arc<str>),
    /// Array of content blocks (for multimodal inputs, tool use, etc.)
    Blocks(Vec<ContentBlockParam>),
}

// =============================================================================
// Content Block Types (Input)
// =============================================================================

/// Input content block (for request messages)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockParam {
    /// Text content
    Text {
        text: Arc<str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    /// Image content
    Image {
        source: ImageSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    /// Document content (PDF, etc.)
    Document {
        source: DocumentSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    /// Tool result (response from a tool call)
    ToolResult {
        tool_use_id: Arc<str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<ToolResultContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
}

impl ContentBlockParam {
    /// Create a text content block
    pub fn text(content: impl Into<Arc<str>>) -> Self {
        Self::Text {
            text: content.into(),
            cache_control: None,
        }
    }

    /// Create a text content block with cache control
    pub fn text_with_cache(content: impl Into<Arc<str>>) -> Self {
        Self::Text {
            text: content.into(),
            cache_control: Some(CacheControl::ephemeral()),
        }
    }

    /// Create an image from base64 data
    pub fn image_base64(media_type: impl Into<Arc<str>>, data: impl Into<Arc<str>>) -> Self {
        Self::Image {
            source: ImageSource::Base64 {
                media_type: media_type.into(),
                data: data.into(),
            },
            cache_control: None,
        }
    }

    /// Create an image from URL
    pub fn image_url(url: impl Into<Arc<str>>) -> Self {
        Self::Image {
            source: ImageSource::Url { url: url.into() },
            cache_control: None,
        }
    }

    /// Create a tool result
    pub fn tool_result(tool_use_id: impl Into<Arc<str>>, content: impl Into<Arc<str>>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: Some(ToolResultContent::Text(content.into())),
            is_error: None,
            cache_control: None,
        }
    }

    /// Create a tool error result
    pub fn tool_error(tool_use_id: impl Into<Arc<str>>, error: impl Into<Arc<str>>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: Some(ToolResultContent::Text(error.into())),
            is_error: Some(true),
            cache_control: None,
        }
    }
}

/// Image source (base64 or URL)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64-encoded image
    Base64 {
        media_type: Arc<str>,
        data: Arc<str>,
    },
    /// URL reference
    Url { url: Arc<str> },
}

/// Document source (base64, URL, or text)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentSource {
    /// Base64-encoded document
    Base64 {
        media_type: Arc<str>,
        data: Arc<str>,
    },
    /// URL reference
    Url { url: Arc<str> },
    /// Plain text document
    Text { text: Arc<str> },
}

/// Tool result content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// Simple text result
    Text(Arc<str>),
    /// Multiple content blocks
    Blocks(Vec<ContentBlockParam>),
}

// =============================================================================
// Content Block Types (Output)
// =============================================================================

/// Output content block (from response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text { text: Arc<str> },

    /// Tool use request
    ToolUse {
        id: Arc<str>,
        name: Arc<str>,
        input: serde_json::Value,
    },

    /// Thinking block (extended thinking)
    Thinking { thinking: Arc<str> },

    /// Redacted thinking (for safety)
    RedactedThinking { data: Arc<str> },

    /// Server tool use (bash, text_editor, web_search)
    ServerToolUse {
        id: Arc<str>,
        name: Arc<str>,
        input: serde_json::Value,
    },

    /// Web search tool result
    WebSearchToolResult {
        tool_use_id: Arc<str>,
        content: serde_json::Value,
    },
}

impl ContentBlock {
    /// Get text content if this is a text block
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }

    /// Get tool use details if this is a tool use block
    pub fn as_tool_use(&self) -> Option<(&str, &str, &serde_json::Value)> {
        match self {
            Self::ToolUse { id, name, input } => Some((id, name, input)),
            _ => None,
        }
    }
}

// =============================================================================
// System Content Types
// =============================================================================

/// System prompt content (string or blocks)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemContent {
    /// Simple text system prompt
    Text(Arc<str>),
    /// Array of system blocks (for cache control)
    Blocks(Vec<SystemBlock>),
}

/// A block in the system prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemBlock {
    /// Block type (always "text")
    #[serde(rename = "type")]
    pub block_type: Arc<str>,

    /// Text content
    pub text: Arc<str>,

    /// Cache control
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl SystemBlock {
    /// Create a text system block
    pub fn text(content: impl Into<Arc<str>>) -> Self {
        Self {
            block_type: Arc::from("text"),
            text: content.into(),
            cache_control: None,
        }
    }

    /// Create a text system block with cache control
    pub fn text_with_cache(content: impl Into<Arc<str>>) -> Self {
        Self {
            block_type: Arc::from("text"),
            text: content.into(),
            cache_control: Some(CacheControl::ephemeral()),
        }
    }
}

/// Cache control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheControl {
    /// Cache type (currently only "ephemeral")
    #[serde(rename = "type")]
    pub cache_type: Arc<str>,
}

impl CacheControl {
    /// Create an ephemeral cache control
    pub fn ephemeral() -> Self {
        Self {
            cache_type: Arc::from("ephemeral"),
        }
    }
}

// =============================================================================
// Metadata Types
// =============================================================================

/// Request metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// An external identifier for the user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Arc<str>>,
}

impl Metadata {
    /// Create metadata with user ID
    pub fn with_user_id(user_id: impl Into<Arc<str>>) -> Self {
        Self {
            user_id: Some(user_id.into()),
        }
    }
}

// =============================================================================
// Tool Types
// =============================================================================

/// Tool definition (custom or server tool)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Tool {
    /// Custom tool defined by user
    Custom(CustomTool),
    /// Server-provided tool
    Server(ServerTool),
}

/// Custom tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTool {
    /// Name of the tool
    pub name: Arc<str>,

    /// Description of what the tool does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Arc<str>>,

    /// JSON schema for the tool's input
    pub input_schema: serde_json::Value,

    /// Cache control
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl CustomTool {
    /// Create a new custom tool
    pub fn new(
        name: impl Into<Arc<str>>,
        description: impl Into<Arc<str>>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: Some(description.into()),
            input_schema,
            cache_control: None,
        }
    }
}

/// Server-provided tool (bash, text_editor, web_search)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerTool {
    /// Bash tool
    #[serde(rename = "bash_20250124")]
    Bash {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<Arc<str>>,
    },

    /// Text editor tool (various versions)
    #[serde(rename = "text_editor_20250124")]
    TextEditor20250124 {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<Arc<str>>,
    },

    #[serde(rename = "text_editor_20250429")]
    TextEditor20250429 {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<Arc<str>>,
    },

    /// Web search tool
    #[serde(rename = "web_search_20250305")]
    WebSearch {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<Arc<str>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        allowed_domains: Option<Vec<Arc<str>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        blocked_domains: Option<Vec<Arc<str>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_uses: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        user_location: Option<UserLocation>,
    },
}

/// User location for web search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<Arc<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<Arc<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<Arc<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<Arc<str>>,
}

/// Tool choice configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    /// Let the model decide whether to use tools
    Auto {
        #[serde(skip_serializing_if = "Option::is_none")]
        disable_parallel_tool_use: Option<bool>,
    },
    /// Model must use at least one tool
    Any {
        #[serde(skip_serializing_if = "Option::is_none")]
        disable_parallel_tool_use: Option<bool>,
    },
    /// Model must not use any tools
    None,
    /// Model must use a specific tool
    Tool {
        name: Arc<str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        disable_parallel_tool_use: Option<bool>,
    },
}

impl ToolChoice {
    /// Create auto tool choice
    pub fn auto() -> Self {
        Self::Auto {
            disable_parallel_tool_use: None,
        }
    }

    /// Create any tool choice
    pub fn any() -> Self {
        Self::Any {
            disable_parallel_tool_use: None,
        }
    }

    /// Create none tool choice
    pub fn none() -> Self {
        Self::None
    }

    /// Create tool-specific choice
    pub fn tool(name: impl Into<Arc<str>>) -> Self {
        Self::Tool {
            name: name.into(),
            disable_parallel_tool_use: None,
        }
    }
}

// =============================================================================
// Thinking and Service Tier
// =============================================================================

/// Extended thinking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingConfig {
    /// Thinking type
    #[serde(rename = "type")]
    pub thinking_type: Arc<str>,

    /// Budget tokens for thinking
    pub budget_tokens: u32,
}

impl ThinkingConfig {
    /// Create enabled thinking config with budget
    pub fn enabled(budget_tokens: u32) -> Self {
        Self {
            thinking_type: Arc::from("enabled"),
            budget_tokens,
        }
    }
}

/// Service tier for request routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTier {
    /// Auto-select tier
    Auto,
    /// Standard tier
    Standard,
}

// =============================================================================
// Response Types
// =============================================================================

/// Messages API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesResponse {
    /// Unique object identifier
    pub id: Arc<str>,

    /// Object type (always "message")
    #[serde(rename = "type")]
    pub response_type: Arc<str>,

    /// Conversational role of the generated message
    pub role: Role,

    /// Content blocks in the response
    pub content: Vec<ContentBlock>,

    /// The model that handled the request
    pub model: Arc<str>,

    /// The reason we stopped generating
    pub stop_reason: Option<StopReason>,

    /// Which custom stop sequence was generated (if any)
    pub stop_sequence: Option<Arc<str>>,

    /// Token usage information
    pub usage: Usage,
}

impl MessagesResponse {
    /// Get all text content joined as a single string
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| block.as_text())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Get all tool use blocks
    pub fn tool_uses(&self) -> Vec<(&str, &str, &serde_json::Value)> {
        self.content
            .iter()
            .filter_map(|block| block.as_tool_use())
            .collect()
    }
}

/// Reason for stopping generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

// =============================================================================
// Count Tokens Types
// =============================================================================

/// Count tokens request
#[derive(Debug, Clone, Serialize)]
pub struct CountTokensRequest {
    /// The model to use for counting
    pub model: Arc<str>,

    /// Messages to count tokens for
    pub messages: Vec<Message>,

    /// System prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemContent>,

    /// Tools to include in count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Tool choice to include in count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Thinking config to include in count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
}

impl CountTokensRequest {
    /// Create a new count tokens request with required fields
    pub fn new(model: impl Into<Arc<str>>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            system: None,
            tools: None,
            tool_choice: None,
            thinking: None,
        }
    }

    /// Set the system prompt
    pub fn with_system(mut self, system: SystemContent) -> Self {
        self.system = Some(system);
        self
    }

    /// Set the tools
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the tool choice
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Set the thinking config
    pub fn with_thinking(mut self, thinking: ThinkingConfig) -> Self {
        self.thinking = Some(thinking);
        self
    }
}

/// Count tokens response
#[derive(Debug, Clone, Deserialize)]
pub struct CountTokensResponse {
    /// Number of input tokens
    pub input_tokens: u32,
}
