# Idiomatic Rust Client Design for OpenAI Chat Completions (Multi-Provider)

This document defines a Rust client design for the OpenAI **Chat Completions**
API with first-class support for OpenAI-compatible providers (OpenRouter,
Gemini, Groq, xAI, MoonshotAI). It prioritizes developer experience, forward
compatibility, and graceful handling of provider differences.

This is a design spec only. It references the request/response shapes and
compatibility notes in `spec/chat-completion*.md`.

---

## 1) Goals and Non-Goals

**Goals**
- Ergonomic API for common use (`client.chat().model(...).user(...)`).
- Full coverage of Chat Completions parameters and response shapes.
- Streaming support with correct SSE parsing and delta accumulation.
- Forward-compatible JSON decoding (unknown fields and enum values tolerated).
- Provider-aware parameter handling with graceful degradation.
- Clean integration with Rust async ecosystems.

**Non-Goals**
- Implement the Responses API (this client is Chat Completions focused).
- Enforce strict compile-time correctness for role/field combos (runtime
  validation is optional and configurable).

---

## 2) Module Layout (Suggested)

```
crates/rullm-openai/
  src/
    client.rs          // ChatCompletionsClient + HTTP wiring
    config.rs          // ClientConfig, ProviderProfile, CapabilityResolver
    types.rs           // Request/response structs, message/content/tool types
    streaming.rs       // SSE decoder + ChatCompletionStream + accumulator
    error.rs           // Error types + retry classification
    compat.rs          // Parameter policy + capability rules
    util.rs            // Small helpers (headers, url, serialization)
```

---

## 3) Client API Surface

### 3.1 Primary Client

```
pub struct ChatCompletionsClient { /* cloneable */ }

impl ChatCompletionsClient {
    pub fn new(config: ClientConfig) -> Result<Self, ClientError>;

    // Core endpoint (non-streaming)
    pub async fn create(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ApiResponse<ChatCompletion>, ClientError>;

    // Core endpoint (streaming)
    pub async fn stream(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionStream, ClientError>;

    // Stored completions (OpenAI only; gated by capability profile)
    pub async fn retrieve(&self, id: &str) -> Result<ApiResponse<ChatCompletion>, ClientError>;
    pub async fn list(&self, params: ListParams) -> Result<ApiResponse<ChatCompletionList>, ClientError>;
    pub async fn update(&self, id: &str, params: UpdateParams) -> Result<ApiResponse<ChatCompletion>, ClientError>;
    pub async fn delete(&self, id: &str) -> Result<ApiResponse<DeleteResponse>, ClientError>;
    pub async fn list_messages(&self, id: &str, params: ListParams)
        -> Result<ApiResponse<ChatMessageList>, ClientError>;

    // DX convenience
    pub fn chat(&self) -> ChatRequestBuilder;
}
```

### 3.2 Convenience Builder

```
let resp = client.chat()
    .model("gpt-4o")
    .system("You are concise.")
    .user("Summarize this")
    .temperature(0.2)
    .send()
    .await?;

let stream = client.chat()
    .model("gpt-4o")
    .user("Stream this")
    .stream()
    .await?;
```

Design notes:
- The builder collects a `Vec<Message>` and converts to `Arc<[Message]>`
  on `send`/`stream`.
- `send()` returns `ApiResponse<ChatCompletion>`.
- `stream()` returns `ChatCompletionStream`.

---

## 4) Configuration and Provider Profiles

### 4.1 ClientConfig

```
pub struct ClientConfig {
    pub api_key: Arc<str>,
    pub base_url: Url,
    pub default_headers: HeaderMap,
    pub timeout: Duration,
    pub provider: ProviderProfile,
    pub parameter_policy: ParameterPolicy,
    pub capability_resolver: Arc<dyn CapabilityResolver>,
}
```

### 4.2 ProviderProfile (Built-in)

`ProviderProfile` supplies defaults and capability constraints.

```
pub enum ProviderKind { OpenAI, OpenRouter, Gemini, Groq, Xai, Moonshot, Custom }

pub struct ProviderProfile {
    pub kind: ProviderKind,
    pub base_url: Url,
    pub supports_stored_completions: bool,
    pub capabilities: Capabilities,
    pub model_rules: Vec<ModelRule>,
}
```

**Built-in profiles** include known constraints (from
`chat-completion-difference.md`):
- Groq: `n=1`, no logprobs, JSON mode cannot stream.
- Gemini: no logprobs, stricter schema validation.
- xAI: reasoning models disallow penalties and stop; no JSON streaming.
- Moonshot: temperature max 1.0, image URLs base64 only.
- OpenRouter: accepts most params, adds comment SSE lines.

### 4.3 CapabilityResolver

```
pub trait CapabilityResolver: Send + Sync {
    fn capabilities_for(&self, model: &ModelId) -> Capabilities;
}
```

`Capabilities` is a simple struct with booleans + numeric limits, e.g.
`supports_logprobs`, `supports_streaming_json`, `temperature_max`, `supports_n`.

### 4.4 ParameterPolicy

```
pub enum ParameterPolicy {
    StrictError,       // reject unsupported parameters
    WarnAndStrip,      // drop unsupported parameters and emit warnings
    PassThrough,       // send as-is (let server reject)
}
```

The client emits a `CompatibilityReport` (warnings, applied transforms) via
`ApiResponse::meta` so users can log or test for mismatches.

---

## 5) Core Types (Request/Response)

### 5.1 Identifiers and Common Newtypes

Use `Arc<str>` for immutable strings:

```
pub struct ModelId(pub Arc<str>);

pub struct Role(pub Arc<str>);
impl Role {
    pub const SYSTEM: Role = Role::static_str("system");
    pub const DEVELOPER: Role = Role::static_str("developer");
    pub const USER: Role = Role::static_str("user");
    pub const ASSISTANT: Role = Role::static_str("assistant");
    pub const TOOL: Role = Role::static_str("tool");
}
```

Using string newtypes avoids breaking when new roles appear.

### 5.2 Messages and Content

```
#[derive(Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Option<MessageContent>,
    pub name: Option<Arc<str>>,
    pub tool_calls: Option<Arc<[ToolCall]>>,
    pub tool_call_id: Option<Arc<str>>,
    pub audio: Option<AssistantAudio>,
    pub function_call: Option<FunctionCall>, // deprecated
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[serde(untagged)]
pub enum MessageContent {
    Text(Arc<str>),
    Parts(Arc<[ContentPart]>),
}
```

Convenience constructors:
- `Message::system(text)`
- `Message::developer(text)`
- `Message::user(text_or_parts)`
- `Message::assistant(text_or_parts)`
- `Message::tool(tool_call_id, content)`

### 5.3 Content Parts

```
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: Arc<str> },
    ImageUrl { image_url: ImageUrlPart },
    InputAudio { input_audio: InputAudioPart },
    File { file: FilePart },
    Refusal { refusal: Arc<str> },
    #[serde(other)]
    Other,
}
```

Note: If `serde(other)` is too lossy, use a `RawContentPart` fallback that
preserves `type` and payload via `serde_json::Value`.

### 5.4 Tools and Tool Calls

```
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolDefinition {
    Function { function: FunctionDefinition },
    Custom { custom: CustomToolDefinition },
    #[serde(other)]
    Other,
}

#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolCall {
    Function { id: Arc<str>, function: FunctionCall },
    Custom { id: Arc<str>, custom: CustomToolCall },
    #[serde(other)]
    Other,
}
```

Tool choice uses an untagged enum:
```
#[serde(untagged)]
pub enum ToolChoice {
    Mode(ToolChoiceMode),
    Function { r#type: Arc<str>, function: ToolChoiceFunction },
    Custom { r#type: Arc<str>, custom: ToolChoiceCustom },
    AllowedTools { r#type: Arc<str>, allowed_tools: AllowedToolsSpec },
}
```

### 5.5 Request Struct

```
pub struct ChatCompletionRequest {
    pub model: ModelId,
    pub messages: Arc<[Message]>,

    // sampling + stopping
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub n: Option<u32>,
    pub stop: Option<Stop>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,

    // tokens
    pub max_completion_tokens: Option<u32>,
    pub max_tokens: Option<u32>, // deprecated

    // logprobs
    pub logprobs: Option<bool>,
    pub top_logprobs: Option<u32>,
    pub logit_bias: Option<Map<String, i32>>,

    // tools
    pub tools: Option<Arc<[ToolDefinition]>>,
    pub tool_choice: Option<ToolChoice>,
    pub parallel_tool_calls: Option<bool>,
    pub functions: Option<Arc<[FunctionDefinition]>>, // deprecated
    pub function_call: Option<FunctionCall>, // deprecated

    // response formatting
    pub response_format: Option<ResponseFormat>,

    // multimodal + audio
    pub modalities: Option<Arc<[Arc<str>]> >,
    pub audio: Option<AudioConfig>,

    // advanced features
    pub stream: Option<bool>,
    pub stream_options: Option<StreamOptions>,
    pub prediction: Option<Prediction>,
    pub web_search_options: Option<WebSearchOptions>,
    pub reasoning_effort: Option<Arc<str>>,
    pub verbosity: Option<Arc<str>>,
    pub service_tier: Option<Arc<str>>,
    pub store: Option<bool>,
    pub metadata: Option<Map<String, Value>>,

    // identifiers
    pub seed: Option<u64>,
    pub user: Option<Arc<str>>, // deprecated
    pub safety_identifier: Option<Arc<str>>,
    pub prompt_cache_key: Option<Arc<str>>,
    pub prompt_cache_retention: Option<Arc<str>>,

    // escape hatch
    pub extra_body: Option<Map<String, Value>>,
}
```

Notes:
- Use `Arc<[T]>` for immutable arrays (messages, tools, modalities).
- Use `Option` for nullable/omitted fields.
- `extra_body` for provider-specific extensions (Gemini thinking config, etc).

### 5.6 Response Structs

```
pub struct ChatCompletion {
    pub id: Arc<str>,
    pub object: Arc<str>,
    pub created: u64,
    pub model: ModelId,
    pub choices: Arc<[ChatChoice]>,
    pub usage: Option<Usage>,
    pub service_tier: Option<Arc<str>>,
    pub system_fingerprint: Option<Arc<str>>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

pub struct ChatChoice {
    pub index: u32,
    pub message: Option<Message>,
    pub finish_reason: Option<Arc<str>>,
    pub logprobs: Option<Logprobs>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
```

Streaming chunk:

```
pub struct ChatCompletionChunk {
    pub id: Arc<str>,
    pub object: Arc<str>,
    pub created: u64,
    pub model: ModelId,
    pub choices: Arc<[ChatChunkChoice]>,
    pub usage: Option<Usage>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
```

---

## 6) Streaming and Accumulation

### 6.1 SSE Parser
- Accept `data:` lines only (ignore comments and empty lines).
- Terminate on `[DONE]` or EOF.
- Surface `error` objects embedded in SSE data.

### 6.2 Accumulator

Provide `ChatCompletionAccumulator` to merge chunks into a final
`ChatCompletion`:
- Concatenate `delta.content` fragments.
- Merge tool call arguments per `tool_call.id` (not just index).
- Track `finish_reason` per choice.
- Handle usage-only final chunk (`choices` empty).

Stream API:
```
pub struct ChatCompletionStream {
    pub fn accumulator(self) -> ChatCompletionAccumulator;
}
```

---

## 7) Provider Compatibility Strategy

### 7.1 Capability-aware Request Shaping

Before sending, apply per-provider and per-model rules:
- Strip or reject unsupported fields (depending on `ParameterPolicy`).
- Transform deprecated/compat fields (e.g., `max_tokens` -> `max_completion_tokens`).
- Clamp values (e.g., Moonshot temperature <= 1.0).

### 7.2 Compatibility Report

```
pub struct CompatibilityReport {
    pub stripped_fields: Vec<&'static str>,
    pub transformed_fields: Vec<(&'static str, &'static str)>,
    pub warnings: Vec<Arc<str>>,
}
```

`ApiResponse<T>` includes `meta.compatibility: Option<CompatibilityReport>`.

### 7.3 Response Variations
- Preserve provider extensions via `#[serde(flatten)] extra` on response types.
- Expose raw JSON for clients that need direct access:
  `ApiResponse::raw_json()`.

---

## 8) Error Handling and Metadata

### 8.1 Error Types

```
pub enum ClientError {
    Http(HttpError),
    Api(ApiError),
    Deserialize(DeserializeError),
    Stream(StreamError),
    Capability(CapabilityError),
}
```

`ApiError` wraps the server `error` object and includes the HTTP status code.

### 8.2 Response Metadata

```
pub struct ResponseMeta {
    pub request_id: Option<Arc<str>>,
    pub ratelimit: Option<RateLimitInfo>,
    pub compatibility: Option<CompatibilityReport>,
    pub latency_ms: Option<u64>,
}

pub struct ApiResponse<T> {
    pub data: T,
    pub meta: ResponseMeta,
}
```

---

## 9) Developer Experience Helpers

- `ChatCompletion::first_text()` returns the first text content (if any).
- `ChatCompletion::tool_calls()` returns tool calls from the first choice.
- `MessageContent::text()` returns `Option<&str>`.
- `ToolCall::arguments_json()` parses JSON arguments to `serde_json::Value`.
- `ChatCompletion::parse_json<T: Deserialize>()` for structured outputs.

All helpers must avoid panics; return `Result` with detailed error types.

---

## 10) Testing Plan (Minimal)

- JSON decode for non-streaming response with tools and annotations.
- SSE stream parsing with content + tool call delta assembly.
- Usage-only final chunk when `include_usage` is set.
- Provider capability stripping and warnings.
- Unknown fields preserved via `extra`.

---

## 11) Rust Idioms and Safety Notes

- Avoid `unwrap()` in production code.
- Use `Arc<[T]>` and `Arc<str>` for immutable shared data.
- Prefer `From/TryFrom` for conversions and `Result` for fallible APIs.
- Avoid wildcard enum imports and `super::` in module paths.
- No global mutable state; configuration is explicit.

---

## 12) Summary of the Design Approach

This design favors **flexible, forward-compatible types** with an ergonomic
builder for the common case. Provider differences are handled centrally via
capability profiles and parameter policies, making the client useful across
OpenAI-compatible endpoints without forcing users to learn each provider's
quirks. The streaming implementation is resilient, and the DX helpers make
structured output and tool calling pleasant without hiding critical behavior.
