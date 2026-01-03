# Rust Port Notes for OpenAI Chat Completions

This document captures implementation guidance for a standalone Rust SDK for
OpenAI Chat Completions, based on:
- `crates/rullm-openai/spec/chat-completion.md` and `chat-completion2.md`
- openai-go and openai-node (generated from the OpenAPI spec by Stainless)
- codex-rs (Chat Completions streaming support inside the Codex CLI)

The goal is to expose a reusable, standalone Chat Completions client, not tied
to the Codex CLI.

## 1) API Surface to Implement
Match the OpenAI SDK patterns (Go/Node) at minimum:

- `POST /chat/completions` (create, non-streaming)
- `POST /chat/completions` (create, streaming)
- `GET /chat/completions/{id}` (retrieve stored completion)
- `GET /chat/completions` (list stored completions)
- `POST /chat/completions/{id}` (update metadata)
- `DELETE /chat/completions/{id}` (delete)
- `GET /chat/completions/{id}/messages` (list stored messages)

Recommended shape for Rust:
- `ChatCompletionsClient::create(params) -> ChatCompletion`
- `ChatCompletionsClient::stream(params) -> Stream<ChatCompletionChunk>`
- `ChatCompletionsClient::retrieve(id)` / `list(params)` / `update(id, params)`
- `ChatCompletionsClient::delete(id)`
- `ChatCompletionsClient::list_messages(id, params)`

## 2) Core Type Map (Request/Response)

### 2.1 Request Types
Define a `ChatCompletionCreateParams` struct that mirrors the Go/Node field set.
Include all current parameters, even if some are deprecated, to preserve API
compatibility:

- Required: `model`, `messages`
- Sampling: `temperature`, `top_p`, `presence_penalty`, `frequency_penalty`
- Tokens: `max_completion_tokens`, `max_tokens` (deprecated)
- Output count: `n`
- Stopping: `stop` (string or array)
- Logprobs: `logprobs`, `top_logprobs`
- Tools: `tools`, `tool_choice`, `parallel_tool_calls`
- Structured outputs: `response_format` (json_schema/json_object/text)
- Audio output: `modalities`, `audio`
- Web search: `web_search_options`
- Predicted outputs: `prediction`
- Prompt caching: `prompt_cache_key`, `prompt_cache_retention`
- Safety: `safety_identifier` (replace `user`)
- Storage: `store`, `metadata`
- Service tier: `service_tier`
- Reasoning: `reasoning_effort`, `verbosity`
- Streaming: `stream`, `stream_options`

Support `null` and omitted fields where the API allows them.

### 2.2 Message Types
`messages` is a union by `role`. Suggested Rust modeling:

- `enum ChatCompletionMessageParam` tagged by `role`
  - `System`, `Developer`, `User`, `Assistant`, `Tool`, `Function (deprecated)`

Common fields:
- `content` for most roles
- `name` optional for `system`, `developer`, `user`, `assistant`
- `tool_call_id` required for `tool` role
- `tool_calls` or `function_call` (deprecated) for assistant messages
- `audio` is allowed on assistant messages

Content is a union:
- `String`
- `Array<ContentPart>`

### 2.3 Content Parts
`ContentPart` is a union by `type`. From openai-node/openai-go:
- `text` { text }
- `image_url` { image_url: { url, detail? } }
- `input_audio` { input_audio: { data, format } }
- `file` { file: { file_id | file_data, filename? } }
- `refusal` (assistant-only content part)

### 2.4 Tools and Tool Calls
Define tool and tool call unions with explicit `type` tags:

Tools (`tools` in request):
- `function` { function: FunctionDefinition }
- `custom` { custom: { name, description?, format? } }
  - `format`: `text` or `grammar` (with `definition` and `syntax`)

Tool calls (in responses and deltas):
- `function` { id, function: { name, arguments } }
- `custom` { id, custom: { name, input } }

Tool choice options (`tool_choice`) are a union:
- string: `none`, `auto`, `required`
- named tool choice: `{ type: "function", function: { name } }`
- custom named tool choice: `{ type: "custom", custom: { name } }`
- allowed tools: `{ type: "allowed_tools", allowed_tools: { mode, tools } }`

Also keep deprecated fields:
- `functions` and `function_call` (request)
- `function_call` (assistant message/stream delta)

### 2.5 Response Types
Non-streaming response: `ChatCompletion`:
- `id`, `object: "chat.completion"`, `created`, `model`
- `choices[]`: `index`, `message`, `finish_reason`, optional `logprobs`
- `usage` (prompt/completion/total + details)
- `service_tier`, `system_fingerprint` (deprecated)

Streaming response: `ChatCompletionChunk`:
- `id`, `object: "chat.completion.chunk"`, `created`, `model`
- `choices[]` with `delta` objects
- `usage` optional (final usage chunk if `include_usage`)

`delta` fields can include:
- `role`, `content`, `refusal`, `tool_calls`, `function_call` (deprecated)
- Logprobs per choice

### 2.6 Usage and Logprobs
Usage should include detail fields (when present):
- completion tokens: `accepted_prediction_tokens`, `rejected_prediction_tokens`,
  `reasoning_tokens`, `audio_tokens`
- prompt tokens: `cached_tokens`, `audio_tokens`

Logprobs include per-token info for both content and refusal.

## 3) Serde Modeling Tips

- Use `#[serde(tag = "role", rename_all = "snake_case")]` for message unions.
- Use `#[serde(tag = "type", rename_all = "snake_case")]` for content parts and
  tool/tool_call unions.
- For `content`, `stop`, `tool_choice`, and `response_format`, use `#[serde(untagged)]`
  enums to support string vs array or object unions.
- Preserve forward compatibility by:
  - `#[serde(default)]` for optional fields
  - `#[serde(flatten)]` to capture unknown fields in responses
  - avoiding strict enum exhaustiveness where new variants may appear

## 4) Streaming and SSE Handling

### 4.1 SSE decoding
- The API uses `text/event-stream` with `data: {json}` and `data: [DONE]`.
- Implement a tolerant SSE parser that:
  - buffers partial chunks
  - ignores empty/comment lines
  - ends on `[DONE]` or socket close
  - treats `error` objects inside `data` as terminal errors

### 4.2 Delta accumulation
Follow openai-go and codex-rs patterns:
- Concatenate `delta.content` and `delta.refusal` fragments in order.
- For `tool_calls`, merge by `index` and `id` and concatenate
  `function.arguments` fragments.
- Handle missing indices (codex-rs maps by `id` or last index).
- Support multiple parallel tool calls (do not assume `index == 0`).
- Keep `finish_reason` per choice.
- Accumulate logprobs and usage (usage often reported only at the final chunk).

Suggested helper: a `ChatCompletionAccumulator` (like openai-go) that merges
chunks into a full `ChatCompletion`, plus convenience helpers for detecting
when content or tool calls have just completed.

### 4.3 Stream options
`stream_options` includes:
- `include_usage` (final usage-only chunk)
- `include_obfuscation` (extra fields on deltas, must be ignored if unknown)

## 5) Structured Outputs and Parsing Helpers

OpenAI SDKs provide helpers to parse structured outputs:
- `response_format` with `type: json_schema`
- `strict` in function definitions to enforce schema adherence

Optional convenience in Rust:
- Provide a helper that parses `choice.message.content` into a typed struct
  when `response_format` is `json_schema`.
- Provide a helper that parses tool call arguments into JSON when `strict` is
  true or when the caller opts in.

These are optional, but common in openai-node (`parse` and tool runner helpers).

## 6) Error Handling and Resilience

- Map HTTP error responses to a structured `ErrorObject` (message/type/param/code).
- Bubble `x-request-id` and rate limit headers up to the caller.
- Accept unknown enum values and ignore unknown fields.
- Do not hard-fail on unsupported parameters; let the API reject if needed.

## 7) Notes from codex-rs

codex-rs includes a dedicated chat completion SSE parser:
- It is robust to missing tool call indices
- It concatenates tool arguments across deltas
- It emits reasoning deltas when present (`delta.reasoning` may be a string or
  nested object)
- It treats `finish_reason == length` as a context window error

This logic is a good reference for a resilient streaming implementation.

## 8) Gaps vs current rullm-core OpenAI types

The current rullm-core types cover only a subset of the modern API. The Rust
port should add:
- `developer` role
- `audio` input and output types
- `file` content parts
- `refusal` content parts
- `custom` tools
- `tool_choice` variants for allowed tools
- `web_search_options` and `annotations`
- `prediction` and prompt caching fields
- `reasoning_effort`, `verbosity`, `service_tier`
- `prompt_cache_key`, `prompt_cache_retention`, `safety_identifier`
- `stream_options.include_usage` and `include_obfuscation`

## 9) Tests to Include

- Non-streaming: full response decode with tool calls and annotations
- Streaming: content deltas, refusal deltas, tool call argument assembly
- Streaming: `include_usage` final chunk with empty choices
- Tool choice unions and stop unions serialize correctly
- Content part unions (text, image_url, input_audio, file)

