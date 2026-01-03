# Anthropic Messages Rust Client - Implementation Design

This document proposes an idiomatic Rust client for the Anthropic Messages API.
It is based on `spec/message-api.md`, `spec/implementation.md`, and patterns in
rullm-core. The design emphasizes ergonomic builders, strong typing, streaming
helpers, and clean error handling.

## 1) Goals and non-goals

Goals
- Feature parity with official Anthropic SDKs for the Messages API.
- Excellent developer experience: easy defaults, expressive builders, helpers for
  common tasks, and easy streaming consumption.
- Forward compatibility: tolerate unknown enum values and fields.

Non-goals (initial release)
- A cross-provider abstraction layer for non-Anthropic APIs (OpenAI/Gemini/etc.).
- Full Bedrock/Vertex implementations (can be added later).

## 2) Package layout (proposed)

```
crates/rullm-anthropic/src/
  client.rs        // Client, ClientBuilder, RequestOptions
  config.rs        // env helpers, base url, auth modes
  error.rs         // AnthropicError, ErrorObject
  messages/        // requests, responses, types
    mod.rs
    types.rs       // content blocks, tools, streaming events
    stream.rs      // SSE parsing + accumulator
  models.rs        // list/get models
  batches.rs       // create/get/list/cancel/delete/results
  transport.rs     // HttpTransport trait + reqwest impl
  lib.rs           // re-exports
```

Notes
- Keep the public surface in `lib.rs` small and intentional.
- Prefer `crate::` paths (avoid `super::`).
- Avoid `pub use` unless re-exporting external dependencies.

## 3) Client configuration and auth

### 3.1 ClientBuilder
Provide a builder with explicit fields and env defaults:

- `Client::builder()` -> `ClientBuilder`
- `Client::from_env()` -> uses:
  - `ANTHROPIC_API_KEY`
  - `ANTHROPIC_AUTH_TOKEN`
  - `ANTHROPIC_BASE_URL`

Auth modes:
- API key: `x-api-key: <key>`
- OAuth token: `Authorization: Bearer <token>`

Required headers:
- `anthropic-version: 2023-06-01`
- `content-type: application/json`

Recommended builder fields:
- `api_key: Option<Arc<str>>`
- `auth_token: Option<Arc<str>>`
- `base_url: Arc<str>` (default `https://api.anthropic.com`)
- `timeout: Duration` (global default)
- `max_retries: u32`
- `beta: Vec<Arc<str>>` (optional `anthropic-beta` header)
- `default_headers: HeaderMap` (merge-able)

### 3.2 RequestOptions (per-request override)
A small options struct to keep the API uniform across clients (even if the
provider APIs differ):

- `timeout: Option<Duration>`
- `extra_headers: HeaderMap`
- `extra_query: Vec<(Arc<str>, Arc<str>)>`
- `extra_body: serde_json::Map<String, Value>`

This mirrors `extra_headers/extra_query/extra_body` patterns in other SDKs.

## 4) Messages API surface

### 4.1 Primary entry points
Expose a sub-client similar to official SDKs:

- `Client::messages()` -> `MessagesClient`
- `MessagesClient::create(req, opts)` -> `Message`
- `MessagesClient::stream(req, opts)` -> `MessageStream`
- `MessagesClient::count_tokens(req, opts)` -> `CountTokensResponse`
- `MessagesClient::batches()` -> `BatchesClient`

### 4.2 Builder ergonomics
Provide a builder for the request that favors clarity:

```
MessagesRequest::builder("claude-3-5-sonnet-20241022")
  .max_tokens(1024)
  .system("You are helpful")
  .message(Message::user("Hello"))
  .temperature(0.7)
  .build()?;
```

Design notes
- `system` is top-level (no system role in messages).
- Accept `system` as `SystemContent` (string or text blocks).
- `messages` accept `MessageContent` (string shorthand or blocks).

### 4.3 Type modeling overview

Request
- `MessagesRequest { model, max_tokens, messages, system?, metadata?, stop_sequences?, temperature?, top_p?, top_k?, tools?, tool_choice?, thinking?, service_tier?, stream? }`

Response
- `Message { id, type, role, model, content, stop_reason?, stop_sequence?, usage }`

Use `serde` tagging:
- `#[serde(tag = "type", rename_all = "snake_case")]` for content blocks
- `#[serde(untagged)]` for `string | [blocks]` unions

## 5) Content blocks and tools

### 5.1 ContentBlockParam (input)
Support all common and advanced blocks:
- `text`
- `image` (base64 or url)
- `document` (pdf base64/url, plain text, or embedded blocks)
- `search_result`
- `tool_result`
- advanced: `tool_use`, `server_tool_use`, `web_search_tool_result`,
  `thinking`, `redacted_thinking`

### 5.2 ContentBlock (output)
Support output blocks:
- `text`, `tool_use`, `thinking`, `redacted_thinking`, `server_tool_use`,
  `web_search_tool_result`

### 5.3 Tools
Use a union for custom and server tools:
- Custom: `{ name, description?, input_schema }`
- Server tools: `bash_20250124`, `text_editor_20250124/20250429/20250728`,
  `web_search_20250305`

Tool choice union:
- `auto | any | none | tool(name)`
- `disable_parallel_tool_use: bool`

## 6) Streaming design

### 6.1 Raw SSE
Streaming uses SSE with event `type`:
- `message_start`
- `content_block_start`
- `content_block_delta`
- `content_block_stop`
- `message_delta`
- `message_stop`

Implement a tolerant SSE parser:
- buffer partial chunks
- ignore empty/comment lines
- stop on stream close
- surface JSON parse errors as `AnthropicError::Serialization`

### 6.2 MessageStream helper
Provide a higher-level stream wrapper that merges deltas into a full message.

Proposed API:
- `MessageStream::events()` -> raw `StreamEvent`
- `MessageStream::text_stream()` -> `impl Stream<Item = Result<Arc<str>, Error>>`
- `MessageStream::final_message()` -> `Result<Message, Error>` (awaits completion)

Use a `MessageAccumulator` internally:
- append text deltas
- merge tool input JSON fragments
- update usage/stop_reason

### 6.3 Tool input JSON deltas
Maintain both:
- `partial_json: String`
- `parsed: Option<Value>` (best-effort)

Parsing strategy:
- append fragment on each delta
- attempt `serde_json::from_str` after each update
- keep the last successful parse

This avoids a hard dependency on a partial JSON parser while still offering
useful intermediate values.

## 7) Timeout policy

The official SDKs enforce a non-streaming timeout policy. Mirror it:

- Default non-stream timeout: 10 minutes
- `expected_time = 1h * max_tokens / 128000`
- If `expected_time > 10m`, require streaming
- Maintain a `MODEL_NONSTREAMING_TOKENS` map (from SDKs)

Expose this as:
- `ClientConfig::non_streaming_policy`
- `MessagesRequest::validate_non_streaming(&policy)`

Allow opt-out via `RequestOptions::allow_long_non_streaming`.

## 8) Error handling

Use a structured error enum and preserve request_id:

```
enum AnthropicError {
  Api { status: StatusCode, request_id: Option<Arc<str>>, error: ErrorObject },
  Transport(reqwest::Error),
  Serialization(String, Box<dyn std::error::Error + Send + Sync>),
  Timeout,
  InvalidRequest(String),
}
```

`ErrorObject` mirrors the response:
- `type`, `message` (plus optional `param` when present)

Always surface `request-id` header in errors and responses.

## 9) Rust ergonomics and idioms

- Avoid panics in library code. No `unwrap`/`expect` in production paths.
- Use `Arc<str>` and `Arc<[T]>` for immutable data cloned often.
- Prefer `From`/`TryFrom` for conversions rather than custom `to_*` methods.
- Provide `Option<&T>` accessors instead of `&Option<T>`.
- Use `&str`/`&[T]` in accessors instead of `&String`/`&Vec<T>`.

## 10) Example usage (final API shape)

Non-streaming:
```
let client = Client::from_env()?;
let req = MessagesRequest::builder("claude-3-5-sonnet-20241022")
    .max_tokens(512)
    .system("You are helpful")
    .message(Message::user("Explain Rust lifetimes."))
    .temperature(0.7)
    .build()?;

let msg = client.messages().create(req, RequestOptions::default()).await?;
let text = msg.text(); // helper to join text blocks
```

Streaming:
```
let stream = client.messages().stream(req, RequestOptions::default()).await?;
let mut text = String::new();
let mut s = stream.text_stream();
while let Some(chunk) = s.next().await {
    text.push_str(&chunk?);
}
let final_msg = stream.final_message().await?;
```
