# OpenAI Chat Completions API - High-Level Spec

This is a high-level overview of the OpenAI Chat Completions REST API. For full
field-level details, see `chat-completion.md` and `chat-completion2.md`.

## Positioning
- Endpoint family: `/v1/chat/completions`
- Status: supported but legacy; newer integrations often use the Responses API.
- Still required when you want classic chat-completion object shapes or stored
  completion CRUD endpoints.

## Endpoints
- POST `/v1/chat/completions` - create a completion (optionally streaming)
- GET `/v1/chat/completions/{completion_id}` - retrieve stored completion
- GET `/v1/chat/completions` - list stored completions (pagination)
- POST `/v1/chat/completions/{completion_id}` - update stored completion metadata
- DELETE `/v1/chat/completions/{completion_id}` - delete stored completion
- GET `/v1/chat/completions/{completion_id}/messages` - list stored messages

## Auth and Headers
- Authorization: `Authorization: Bearer <API_KEY>`
- Optional routing: `OpenAI-Organization`, `OpenAI-Project`
- Content-Type: `application/json`
- Useful response headers: `x-request-id`, `openai-processing-ms`, `x-ratelimit-*`

## Core Request Shape
```json
{
  "model": "gpt-4o",
  "messages": [...],
  "stream": false
}
```

### Messages and Content
Messages are role-tagged objects. `content` is either a string or an array of
content parts.

Roles (non-exhaustive):
- `system` (legacy instructions)
- `developer` (preferred for reasoning models)
- `user`
- `assistant`
- `tool`
- `function` (deprecated)

Content parts (union by `type`):
- `text` `{ type: "text", text: "..." }`
- `image_url` `{ type: "image_url", image_url: { url, detail? } }`
- `input_audio` `{ type: "input_audio", input_audio: { data, format } }`
- `file` `{ type: "file", file: { file_id | file_data, filename? } }`
- `refusal` (assistant-only content part)

Assistant messages may omit `content` and instead include `tool_calls`.
Tool responses use role `tool` and include `tool_call_id`.

## Common Request Parameters (high-level)
- Sampling: `temperature`, `top_p`, `presence_penalty`, `frequency_penalty`
- Tokens: `max_completion_tokens`, `max_tokens` (deprecated)
- Output count: `n`
- Stopping: `stop`
- Logprobs: `logprobs`, `top_logprobs`
- Tools: `tools`, `tool_choice`, `parallel_tool_calls`
- Structured outputs: `response_format` (`json_schema` or `json_object`)
- Audio output: `modalities`, `audio`
- Web search: `web_search_options`
- Predicted outputs: `prediction`
- Prompt caching: `prompt_cache_key`, `prompt_cache_retention`
- Safety: `safety_identifier` (replaces `user`)
- Storage: `store`, `metadata`
- Service tiers: `service_tier`
- Reasoning: `reasoning_effort`, `verbosity`
- Streaming: `stream`, `stream_options`

## Non-Streaming Response Shape
Chat completion object:
- `id`, `object: "chat.completion"`, `created`, `model`
- `choices[]`: each includes `message`, `finish_reason`, optional `logprobs`
- `message`: `role: assistant`, `content` or `refusal`, `tool_calls`, `audio`,
  optional `annotations` (web search)
- `usage`: `prompt_tokens`, `completion_tokens`, `total_tokens` + details
- `service_tier`, `system_fingerprint` (deprecated)

Finish reasons can include: `stop`, `length`, `tool_calls`, `content_filter`,
`function_call` (deprecated).

## Streaming (SSE)
- Enable with `stream: true`.
- The server emits SSE events whose data is a `chat.completion.chunk` object.
- Each chunk has `choices[].delta` with partial data:
  - `role`, `content`, `refusal`, `tool_calls`, `function_call` (deprecated)
- Tool call arguments arrive as streamed string fragments.
- `stream_options` supports:
  - `include_usage` (final usage-only chunk)
  - `include_obfuscation` (adds obfuscation fields to normalize payload sizes)
- Stream ends with `data: [DONE]` or connection close.

## Errors and Rate Limits
- Errors return a top-level `error` object with fields like `message`, `type`,
  `param`, `code`.
- Streaming may emit an error object inside the SSE `data` payload.
- Rate limit headers provide request and token budgets; clients should parse and
  surface them.
