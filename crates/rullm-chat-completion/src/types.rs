use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

// =============================================================================
// Core Newtypes
// =============================================================================

/// Model identifier (e.g., "gpt-4o", "gpt-4-turbo").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelId(pub Arc<str>);

impl ModelId {
    /// Create a new model ID.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

impl<T: Into<Arc<str>>> From<T> for ModelId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

impl std::ops::Deref for ModelId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Message role (system, user, assistant, tool, developer).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Role(pub Arc<str>);

impl Role {
    /// System role for legacy instructions.
    pub const SYSTEM: &'static str = "system";
    /// Developer role for reasoning model instructions.
    pub const DEVELOPER: &'static str = "developer";
    /// User role for user messages.
    pub const USER: &'static str = "user";
    /// Assistant role for model responses.
    pub const ASSISTANT: &'static str = "assistant";
    /// Tool role for tool call results.
    pub const TOOL: &'static str = "tool";

    /// Create a new role.
    pub fn new(role: impl Into<Arc<str>>) -> Self {
        Self(role.into())
    }

    /// Create a system role.
    pub fn system() -> Self {
        Self(Arc::from(Self::SYSTEM))
    }

    /// Create a developer role.
    pub fn developer() -> Self {
        Self(Arc::from(Self::DEVELOPER))
    }

    /// Create a user role.
    pub fn user() -> Self {
        Self(Arc::from(Self::USER))
    }

    /// Create an assistant role.
    pub fn assistant() -> Self {
        Self(Arc::from(Self::ASSISTANT))
    }

    /// Create a tool role.
    pub fn tool() -> Self {
        Self(Arc::from(Self::TOOL))
    }
}

impl std::ops::Deref for Role {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// =============================================================================
// Messages
// =============================================================================

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message author.
    pub role: Role,
    /// The content of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,
    /// Optional name of the author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Arc<str>>,
    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Arc<[ToolCall]>>,
    /// ID of the tool call this message is responding to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<Arc<str>>,
    /// Audio output from the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AssistantAudio>,
    /// Deprecated function call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
    /// Additional fields for forward compatibility.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Message {
    /// Create a system message.
    pub fn system(text: impl Into<Arc<str>>) -> Self {
        Self {
            role: Role::system(),
            content: Some(MessageContent::Text(text.into())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            audio: None,
            function_call: None,
            extra: Map::new(),
        }
    }

    /// Create a developer message.
    pub fn developer(text: impl Into<Arc<str>>) -> Self {
        Self {
            role: Role::developer(),
            content: Some(MessageContent::Text(text.into())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            audio: None,
            function_call: None,
            extra: Map::new(),
        }
    }

    /// Create a user message with text content.
    pub fn user(text: impl Into<Arc<str>>) -> Self {
        Self {
            role: Role::user(),
            content: Some(MessageContent::Text(text.into())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            audio: None,
            function_call: None,
            extra: Map::new(),
        }
    }

    /// Create a user message with content parts.
    pub fn user_parts(parts: impl Into<Arc<[ContentPart]>>) -> Self {
        Self {
            role: Role::user(),
            content: Some(MessageContent::Parts(parts.into())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            audio: None,
            function_call: None,
            extra: Map::new(),
        }
    }

    /// Create an assistant message with text content.
    pub fn assistant(text: impl Into<Arc<str>>) -> Self {
        Self {
            role: Role::assistant(),
            content: Some(MessageContent::Text(text.into())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            audio: None,
            function_call: None,
            extra: Map::new(),
        }
    }

    /// Create an assistant message with tool calls.
    pub fn assistant_tool_calls(tool_calls: impl Into<Arc<[ToolCall]>>) -> Self {
        Self {
            role: Role::assistant(),
            content: None,
            name: None,
            tool_calls: Some(tool_calls.into()),
            tool_call_id: None,
            audio: None,
            function_call: None,
            extra: Map::new(),
        }
    }

    /// Create a tool result message.
    pub fn tool(tool_call_id: impl Into<Arc<str>>, content: impl Into<Arc<str>>) -> Self {
        Self {
            role: Role::tool(),
            content: Some(MessageContent::Text(content.into())),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            audio: None,
            function_call: None,
            extra: Map::new(),
        }
    }
}

/// Message content - either plain text or an array of content parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Plain text content.
    Text(Arc<str>),
    /// Array of content parts (for multimodal messages).
    Parts(Arc<[ContentPart]>),
}

impl MessageContent {
    /// Get the text content if this is a text message.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(text) => Some(text),
            MessageContent::Parts(_) => None,
        }
    }

    /// Get all text from the content, concatenating text parts.
    pub fn text(&self) -> Option<String> {
        match self {
            MessageContent::Text(text) => Some(text.to_string()),
            MessageContent::Parts(parts) => {
                let texts: Vec<&str> = parts
                    .iter()
                    .filter_map(|p| match p {
                        ContentPart::Text { text } => Some(text.as_ref()),
                        _ => None,
                    })
                    .collect();
                if texts.is_empty() {
                    None
                } else {
                    Some(texts.join(""))
                }
            }
        }
    }
}

// =============================================================================
// Content Parts
// =============================================================================

/// A content part in a multimodal message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Text content.
    Text { text: Arc<str> },
    /// Image URL content.
    ImageUrl { image_url: ImageUrlPart },
    /// Audio input content.
    InputAudio { input_audio: InputAudioPart },
    /// File content.
    File { file: FilePart },
    /// Refusal content (assistant only).
    Refusal { refusal: Arc<str> },
}

impl ContentPart {
    /// Create a text content part.
    pub fn text(text: impl Into<Arc<str>>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create an image URL content part.
    pub fn image_url(url: impl Into<Arc<str>>) -> Self {
        Self::ImageUrl {
            image_url: ImageUrlPart {
                url: url.into(),
                detail: None,
            },
        }
    }

    /// Create an image URL content part with detail level.
    pub fn image_url_with_detail(url: impl Into<Arc<str>>, detail: impl Into<Arc<str>>) -> Self {
        Self::ImageUrl {
            image_url: ImageUrlPart {
                url: url.into(),
                detail: Some(detail.into()),
            },
        }
    }
}

/// Image URL part details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrlPart {
    /// The URL of the image.
    pub url: Arc<str>,
    /// Detail level ("low", "high", "auto").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<Arc<str>>,
}

/// Audio input part details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudioPart {
    /// Base64-encoded audio data.
    pub data: Arc<str>,
    /// Audio format ("wav", "mp3", etc.).
    pub format: Arc<str>,
}

/// File part details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePart {
    /// File ID if using file API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<Arc<str>>,
    /// Base64-encoded file data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<Arc<str>>,
    /// Optional filename.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<Arc<str>>,
}

// =============================================================================
// Audio
// =============================================================================

/// Audio output from the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantAudio {
    /// Audio ID.
    pub id: Arc<str>,
    /// Expiration timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// Base64-encoded audio data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Arc<str>>,
    /// Transcript of the audio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<Arc<str>>,
}

/// Audio configuration for the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Voice to use for audio output.
    pub voice: Arc<str>,
    /// Audio output format.
    pub format: Arc<str>,
}

// =============================================================================
// Tools
// =============================================================================

/// Tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolDefinition {
    /// Function tool.
    Function { function: FunctionDefinition },
}

/// Function definition for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// The name of the function.
    pub name: Arc<str>,
    /// A description of what the function does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Arc<str>>,
    /// The parameters the function accepts (JSON Schema).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
    /// Whether to enable strict mode for parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl FunctionDefinition {
    /// Create a new function definition.
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            description: None,
            parameters: None,
            strict: None,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<Arc<str>>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the parameters schema.
    pub fn with_parameters(mut self, parameters: Value) -> Self {
        self.parameters = Some(parameters);
        self
    }

    /// Enable strict mode.
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = Some(strict);
        self
    }
}

/// A tool call from the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The ID of the tool call.
    pub id: Arc<str>,
    /// The type of tool call (always "function" for now).
    #[serde(rename = "type")]
    pub call_type: Arc<str>,
    /// The function being called.
    pub function: FunctionCall,
}

impl ToolCall {
    /// Parse the function arguments as JSON.
    pub fn arguments_json(&self) -> Result<Value, serde_json::Error> {
        serde_json::from_str(&self.function.arguments)
    }

    /// Parse the function arguments as a typed value.
    pub fn arguments_as<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.function.arguments)
    }
}

/// A function call (name and arguments).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The name of the function.
    pub name: Arc<str>,
    /// The arguments to pass to the function (JSON string).
    pub arguments: Arc<str>,
}

/// Tool choice specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Simple mode ("auto", "none", "required").
    Mode(Arc<str>),
    /// Specific function to call.
    Function {
        #[serde(rename = "type")]
        choice_type: Arc<str>,
        function: ToolChoiceFunction,
    },
}

impl ToolChoice {
    /// Auto mode - let the model decide.
    pub fn auto() -> Self {
        Self::Mode(Arc::from("auto"))
    }

    /// None mode - don't call any tools.
    pub fn none() -> Self {
        Self::Mode(Arc::from("none"))
    }

    /// Required mode - must call a tool.
    pub fn required() -> Self {
        Self::Mode(Arc::from("required"))
    }

    /// Force a specific function.
    pub fn function(name: impl Into<Arc<str>>) -> Self {
        Self::Function {
            choice_type: Arc::from("function"),
            function: ToolChoiceFunction { name: name.into() },
        }
    }
}

/// Function specification for tool choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChoiceFunction {
    /// The name of the function to call.
    pub name: Arc<str>,
}

// =============================================================================
// Response Format
// =============================================================================

/// Response format specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Plain text response.
    Text,
    /// JSON object response.
    JsonObject,
    /// JSON schema response.
    JsonSchema { json_schema: JsonSchemaFormat },
}

impl ResponseFormat {
    /// Create a text response format.
    pub fn text() -> Self {
        Self::Text
    }

    /// Create a JSON object response format.
    pub fn json_object() -> Self {
        Self::JsonObject
    }

    /// Create a JSON schema response format.
    pub fn json_schema(name: impl Into<Arc<str>>, schema: Value) -> Self {
        Self::JsonSchema {
            json_schema: JsonSchemaFormat {
                name: name.into(),
                description: None,
                schema,
                strict: None,
            },
        }
    }
}

/// JSON schema format specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaFormat {
    /// The name of the schema.
    pub name: Arc<str>,
    /// Description of the schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Arc<str>>,
    /// The JSON schema.
    pub schema: Value,
    /// Whether to enable strict mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

// =============================================================================
// Stop Sequences
// =============================================================================

/// Stop sequence(s) for generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Stop {
    /// Single stop sequence.
    Single(Arc<str>),
    /// Multiple stop sequences.
    Multiple(Arc<[Arc<str>]>),
}

impl Stop {
    /// Create a single stop sequence.
    pub fn single(s: impl Into<Arc<str>>) -> Self {
        Self::Single(s.into())
    }

    /// Create multiple stop sequences.
    pub fn multiple(sequences: impl IntoIterator<Item = impl Into<Arc<str>>>) -> Self {
        Self::Multiple(sequences.into_iter().map(Into::into).collect())
    }
}

// =============================================================================
// Advanced Options
// =============================================================================

/// Stream options for streaming requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamOptions {
    /// Include usage information in the final chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}

/// Prediction for predicted outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    /// The type of prediction (always "content").
    #[serde(rename = "type")]
    pub prediction_type: Arc<str>,
    /// The predicted content.
    pub content: MessageContent,
}

/// Web search options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchOptions {
    /// Whether to enable web search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_context_size: Option<Arc<str>>,
    /// User location for search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_location: Option<UserLocation>,
}

/// User location for web search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLocation {
    /// Approximate location type.
    #[serde(rename = "type")]
    pub location_type: Arc<str>,
    /// Approximate location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approximate: Option<ApproximateLocation>,
}

/// Approximate location details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproximateLocation {
    /// City name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<Arc<str>>,
    /// Country code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<Arc<str>>,
    /// Region/state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<Arc<str>>,
    /// Timezone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<Arc<str>>,
}

// =============================================================================
// Request
// =============================================================================

/// Chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// Model to use for completion.
    pub model: ModelId,
    /// Messages for the conversation.
    pub messages: Arc<[Message]>,

    // Sampling parameters
    /// Temperature for sampling (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p (nucleus) sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Number of completions to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>,
    /// Presence penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// Frequency penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    // Token limits
    /// Maximum tokens to generate (preferred).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    /// Maximum tokens (deprecated, use max_completion_tokens).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    // Logprobs
    /// Whether to return log probabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    /// Number of top log probabilities to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u32>,
    /// Token bias map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<Map<String, Value>>,

    // Tools
    /// Tool definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Arc<[ToolDefinition]>>,
    /// Tool choice specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Whether to allow parallel tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Deprecated function definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Arc<[FunctionDefinition]>>,

    // Response formatting
    /// Response format specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    // Multimodal
    /// Output modalities (e.g., ["text", "audio"]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Arc<[Arc<str>]>>,
    /// Audio configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioConfig>,

    // Advanced
    /// Enable streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Stream options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
    /// Predicted output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction: Option<Prediction>,
    /// Web search options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_options: Option<WebSearchOptions>,
    /// Reasoning effort level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<Arc<str>>,
    /// Service tier preference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<Arc<str>>,
    /// Whether to store the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    /// Metadata for stored completions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,

    // Identifiers
    /// Seed for deterministic sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// User identifier (deprecated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<Arc<str>>,

    // Escape hatch
    /// Extra fields for provider-specific extensions.
    #[serde(flatten)]
    pub extra_body: Map<String, Value>,
}

impl ChatCompletionRequest {
    /// Create a new chat completion request.
    pub fn new(model: impl Into<ModelId>, messages: impl Into<Arc<[Message]>>) -> Self {
        Self {
            model: model.into(),
            messages: messages.into(),
            temperature: None,
            top_p: None,
            n: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            max_completion_tokens: None,
            max_tokens: None,
            logprobs: None,
            top_logprobs: None,
            logit_bias: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            functions: None,
            response_format: None,
            modalities: None,
            audio: None,
            stream: None,
            stream_options: None,
            prediction: None,
            web_search_options: None,
            reasoning_effort: None,
            service_tier: None,
            store: None,
            metadata: None,
            seed: None,
            user: None,
            extra_body: Map::new(),
        }
    }
}

// =============================================================================
// Response
// =============================================================================

/// Chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletion {
    /// Unique identifier for this completion.
    pub id: Arc<str>,
    /// Object type (always "chat.completion").
    pub object: Arc<str>,
    /// Unix timestamp when the completion was created.
    pub created: u64,
    /// Model used for the completion.
    pub model: ModelId,
    /// Completion choices.
    pub choices: Arc<[ChatChoice]>,
    /// Token usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// Service tier used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<Arc<str>>,
    /// System fingerprint (deprecated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<Arc<str>>,
    /// Additional fields for forward compatibility.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl ChatCompletion {
    /// Get the first choice's text content.
    pub fn first_text(&self) -> Option<&str> {
        self.choices.first().and_then(|c| {
            c.message
                .as_ref()
                .and_then(|m| m.content.as_ref())
                .and_then(|content| content.as_text())
        })
    }

    /// Get tool calls from the first choice.
    pub fn tool_calls(&self) -> Option<&[ToolCall]> {
        self.choices
            .first()
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.tool_calls.as_ref())
            .map(|tc| tc.as_ref())
    }

    /// Parse the first choice's content as JSON.
    pub fn parse_json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        let text = self.first_text().unwrap_or("");
        serde_json::from_str(text)
    }

    /// Get the finish reason for the first choice.
    pub fn finish_reason(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .map(|s| s.as_ref())
    }
}

/// A completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    /// The index of this choice.
    pub index: u32,
    /// The generated message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    /// The reason the model stopped generating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<Arc<str>>,
    /// Log probabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Logprobs>,
    /// Additional fields.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Log probabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logprobs {
    /// Log probabilities for each token.
    pub content: Option<Arc<[TokenLogprob]>>,
    /// Refusal log probabilities.
    pub refusal: Option<Arc<[TokenLogprob]>>,
}

/// Log probability for a single token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLogprob {
    /// The token.
    pub token: Arc<str>,
    /// Log probability of this token.
    pub logprob: f64,
    /// Byte representation.
    pub bytes: Option<Arc<[u8]>>,
    /// Top log probabilities.
    pub top_logprobs: Option<Arc<[TopLogprob]>>,
}

/// Top log probability entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopLogprob {
    /// The token.
    pub token: Arc<str>,
    /// Log probability.
    pub logprob: f64,
    /// Byte representation.
    pub bytes: Option<Arc<[u8]>>,
}

/// Token usage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt.
    pub prompt_tokens: u32,
    /// Number of tokens in the completion.
    pub completion_tokens: u32,
    /// Total number of tokens.
    pub total_tokens: u32,
    /// Detailed prompt token breakdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    /// Detailed completion token breakdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
}

/// Detailed prompt token breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTokensDetails {
    /// Cached tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
    /// Audio tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<u32>,
}

/// Detailed completion token breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTokensDetails {
    /// Reasoning tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
    /// Audio tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<u32>,
    /// Accepted prediction tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_prediction_tokens: Option<u32>,
    /// Rejected prediction tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_prediction_tokens: Option<u32>,
}

// =============================================================================
// Streaming Types
// =============================================================================

/// A streaming chunk from the chat completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier for this completion.
    pub id: Arc<str>,
    /// Object type (always "chat.completion.chunk").
    pub object: Arc<str>,
    /// Unix timestamp when the chunk was created.
    pub created: u64,
    /// Model used for the completion.
    pub model: ModelId,
    /// Chunk choices.
    pub choices: Arc<[ChatChunkChoice]>,
    /// Token usage (only in final chunk with include_usage).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// Service tier used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<Arc<str>>,
    /// System fingerprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<Arc<str>>,
    /// Additional fields.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A choice in a streaming chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunkChoice {
    /// The index of this choice.
    pub index: u32,
    /// The delta (partial message).
    pub delta: ChunkDelta,
    /// The reason the model stopped generating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<Arc<str>>,
    /// Log probabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Logprobs>,
    /// Additional fields.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Delta content in a streaming chunk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChunkDelta {
    /// The role (usually only in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    /// Content fragment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Arc<str>>,
    /// Refusal fragment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<Arc<str>>,
    /// Tool call fragments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Arc<[ToolCallDelta]>>,
    /// Deprecated function call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCallDelta>,
    /// Additional fields.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Tool call delta in streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    /// The index of this tool call.
    pub index: u32,
    /// Tool call ID (usually only in first chunk for this call).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Arc<str>>,
    /// Tool type.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub call_type: Option<Arc<str>>,
    /// Function call delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionCallDelta>,
}

/// Function call delta in streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallDelta {
    /// Function name (usually only in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Arc<str>>,
    /// Arguments fragment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Arc<str>>,
}
