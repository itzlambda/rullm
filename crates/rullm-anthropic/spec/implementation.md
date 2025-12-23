# Rust SDK port notes (implementation guidance)

This document captures what we need to know to port the official Anthropic SDK to Rust. It is based on:
- The local `reference.md` spec
- The Go SDK (strong typed model, helpers, timeout logic)
- The Python and TypeScript SDKs (streaming helpers, ergonomics)

The goal is to provide a Rust API that is feature-parity with the official SDKs while fitting the rullm workspace style.

## 1) Scope and parity targets

Minimum parity for a first pass (mirrors Go/TS/Python):
- Core API client with config and auth
- `messages`:
  - create (non-stream)
  - create with streaming
  - stream helper (aggregates raw SSE events)
  - count_tokens
  - message batches (create/get/list/cancel/delete/results)
- `models` (list/get)
- `completions` (legacy API)
- `beta` resources (optional, via `anthropic-beta` header)

Optional parity:
- Bedrock / Vertex clients (Go/Python include these)
- Helpers for prompt caching
- Deprecated model warnings

## 2) Client configuration and auth

Match official SDK behavior:
- Env vars:
  - `ANTHROPIC_API_KEY`
  - `ANTHROPIC_AUTH_TOKEN`
  - `ANTHROPIC_BASE_URL`
- Base URL default: `https://api.anthropic.com`
- Headers:
  - `anthropic-version: 2023-06-01`
  - `X-Api-Key` OR `Authorization: Bearer <token>`
- Provide per-request overrides:
  - timeout
  - extra headers
  - extra query params
  - extra body fields

Recommendation:
- Build a `Client` struct similar to Go:
  - `Client::new(api_key, auth_token, base_url, options...)`
  - `Client::from_env()`
  - sub-services: `messages`, `models`, `completions`, `beta`

## 3) HTTP layer and retries

The official SDKs include retry support and expose:
- `max_retries`
- default timeout

Rust port should:
- Use `reqwest` (already in workspace)
- Support global and per-request timeout
- Expose retry policy (even a simple fixed retry is OK initially)
- Surface `request-id` header in responses

## 4) Serialization strategy (serde)

The Messages API relies heavily on tagged unions. Use:
- `#[serde(tag = "type", rename_all = "snake_case")]` for unions with a `type` discriminator
- `#[serde(untagged)]` for unions like `string | [blocks]`
- `serde_json::Value` for tool input and JSON Schema fields

Suggested core enums:
- `ContentBlock` (output)
- `ContentBlockParam` (input)
- `ToolUnion` / `ToolChoice`
- `ThinkingConfig`
- `Citation` and citation location variants
- `MessageStreamEvent` and delta variants

Notes from SDKs:
- Go avoids the `string` shorthand for `messages.content` (requires blocks).
- Python/TS accept `string | [blocks]`.
- Rust can support both by using `#[serde(untagged)]` plus helpers that convert strings to text blocks.

## 5) Messages API design in Rust

### 5.1 Request structs
Required:
- `model: String` (or `Model` wrapper)
- `max_tokens: u32`
- `messages: Vec<MessageParam>`

Optional:
- `system: String | Vec<TextBlockParam>`
- `metadata`
- `stop_sequences`
- `temperature`, `top_p`, `top_k`
- `tools`, `tool_choice`
- `thinking`
- `service_tier`
- `stream`

### 5.2 Response structs
`Message` includes:
- `id`, `type`, `role`, `model`
- `content: Vec<ContentBlock>`
- `stop_reason`, `stop_sequence`
- `usage` (input/output tokens, cache fields, service_tier, server_tool_use)

### 5.3 Tooling types
Support both custom and server tools:
- Custom: `name`, `description?`, `input_schema`
- Server tools: `bash_20250124`, `text_editor_20250124/20250429/20250728`, `web_search_20250305`
Tool choice union:
- `auto`, `any`, `tool`, `none`
- `disable_parallel_tool_use` boolean

### 5.4 Content blocks (input)
At minimum:
- `text`
- `image` (base64 or url)
- `document` (pdf base64 or url, plain text, or embedded content)
- `search_result`
- `tool_result`

Advanced (for parity):
- `tool_use` (rare in input, but used for continuity)
- `server_tool_use`
- `web_search_tool_result`
- `thinking`, `redacted_thinking`

## 6) Streaming support

### 6.1 Raw SSE
Streaming sends SSE events, each with a JSON object containing a `type`:
- `message_start`
- `content_block_start`
- `content_block_delta`
- `content_block_stop`
- `message_delta`
- `message_stop`

`content_block_delta` variants:
- `text_delta`
- `input_json_delta` (tool input)
- `citations_delta`
- `thinking_delta`
- `signature_delta`

### 6.2 Stream helper (recommended)
Python/TS expose a higher-level stream helper that:
- Accumulates a `Message` snapshot
- Emits derived events (`text`, `citation`, `thinking`, `input_json`, etc.)
- Provides `text_stream` and `get_final_message()` helpers

Rust port should consider:
- `MessageStream` wrapper that consumes raw SSE events
- `MessageStream::text_stream()` yields only text deltas
- `MessageStream::final_message()` returns accumulated message

### 6.3 Partial JSON parsing
`input_json_delta` sends partial JSON strings.
- TS uses a partial JSON parser
- Python uses `jiter` partial parsing

Rust options:
- Accumulate raw JSON text per tool-use block and parse on each delta
- Use a partial JSON parser crate if available

Maintain both:
- `partial_json` text buffer
- best-effort parsed `serde_json::Value` snapshot

## 7) Timeout behavior (important)

Go and TS enforce a non-streaming timeout:
- Default non-streaming timeout: 10 minutes
- `expected_time = 1h * max_tokens / 128000`
- If `expected_time > 10 minutes`, or `max_tokens` exceeds a model-specific limit, require streaming

Port should:
- Include a `MODEL_NONSTREAMING_TOKENS` map (from SDKs)
- Compute timeout and error when streaming is required
- Allow caller override for timeout

## 8) Errors

Error responses are structured:
- `error: { type, message }`
- `request_id`

Types include:
- `invalid_request_error`
- `authentication_error`
- `billing_error`
- `permission_error`
- `not_found_error`
- `rate_limit_error`
- `timeout_error`
- `api_error`
- `overloaded_error`

Rust error enum should:
- Preserve HTTP status
- Preserve `request_id`
- Keep raw body for debugging

## 9) Pagination

List endpoints use cursor params:
- `after_id`, `before_id`, `limit`

Responses include:
- `data: []`
- `has_more`
- `first_id`, `last_id`

Provide a `Page<T>` with cursor helpers, similar to Go's pagination module.

## 10) Deprecation warnings (optional)

Python/TS warn on deprecated models using a known list.
Rust port can:
- Maintain a `DEPRECATED_MODELS` map
- Emit warnings (log or `eprintln!`)

## 11) Beta support

Go SDK includes `beta` services and uses the `anthropic-beta` header.
For parity:
- Allow optional `betas: Vec<String>` header
- Provide beta resources where needed (models/messages/files)

## 12) Suggested module layout

```
crates/rullm-anthropic/src/
  client.rs            // Client config, auth, request builder
  error.rs             // Error types and mapping
  resources/
    messages.rs        // create, stream, count_tokens
    message_batches.rs // create/get/list/cancel/delete/results
    models.rs
    completions.rs
    beta/...
  types/
    message.rs
    content_block.rs
    tool.rs
    streaming.rs
  streaming/
    sse.rs             // SSE parser or reuse rullm-core
    message_stream.rs  // high-level aggregator
```

This layout mirrors the official SDKs while fitting Rust conventions.

