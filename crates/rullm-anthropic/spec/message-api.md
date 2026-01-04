# Anthropic Messages API (high-level)

This is a concise, implementation-focused overview of the Messages API as used by the official SDKs (Go, Python, TypeScript) and the local `reference.md`.

## Endpoints

- `POST /v1/messages`
  - Create a message (non-streaming)
  - For streaming: include `"stream": true` in the request body
- `POST /v1/messages/count_tokens`
  - Count input tokens for a request without generating output
- `POST /v1/messages/batches` and related batch endpoints
  - Asynchronous batch processing for multiple messages requests

## Auth and headers

- Base URL: `https://api.anthropic.com` (overridable via `ANTHROPIC_BASE_URL`)
- Auth: either `X-Api-Key` or `Authorization: Bearer <token>`
- Required version header: `anthropic-version: 2023-06-01`
- Responses include `request-id` header (useful for debugging)

## Core request shape (create message)

Required:
- `model: string`
- `max_tokens: number`
- `messages: MessageParam[]`

Optional (most common):
- `system: string | TextBlockParam[]`
- `metadata: { user_id?: string }`
- `stop_sequences: string[]`
- `temperature: number` (0.0 to 1.0)
- `top_p: number` (0.0 to 1.0)
- `top_k: number`
- `tools: Tool[] | ServerTool[]`
- `tool_choice: ToolChoice`
- `thinking: ThinkingConfig`
- `service_tier: "auto" | "standard_only"`
- `stream: boolean`

Notes:
- `system` is a top-level field; there is no `"system"` role in messages.
- Consecutive messages with the same role are allowed; the server will merge them.
- If the last input message is `assistant`, the response continues from that content.
- There is a documented limit of 100,000 messages per request.

## MessageParam and content blocks

`messages` is an array of `{ role, content }` where `role` is `user` or `assistant`.

`content` can be:
- A string (shorthand for a single text block)
- An array of content blocks

### Common input content block types

- `text`: `{ type: "text", text, cache_control?, citations? }`
- `image`: `{ type: "image", source, cache_control? }`
  - `source` is either base64 (`{ type: "base64", media_type, data }`) or URL (`{ type: "url", url }`)
- `document`: `{ type: "document", source, title?, context?, citations?, cache_control? }`
  - `source` can be base64 PDF, URL PDF, plain text, or embedded content blocks
- `search_result`: `{ type: "search_result", source, title, content: TextBlockParam[], cache_control? }`
- `tool_result`: `{ type: "tool_result", tool_use_id, content?, is_error?, cache_control? }`

Less common / advanced:
- `tool_use`, `server_tool_use`, `web_search_tool_result`
- `thinking`, `redacted_thinking`

Cache control:
- `cache_control` uses `{"type":"ephemeral","ttl":"5m"|"1h"}` to mark caching breakpoints.

## Tools and tool_choice

`tools` can include:
- Custom client tools: `{ name, description?, input_schema }`
- Server tools (built-in types): `bash_20250124`, `text_editor_20250124/20250429/20250728`, `web_search_20250305`

`tool_choice` controls tool usage:
- `{ type: "auto" | "any" | "none" }`
- `{ type: "tool", name }`
- `disable_parallel_tool_use` can be set to force a single tool use.

## Response: Message object

The response message includes:
- `id`, `type: "message"`, `role: "assistant"`, `model`
- `content: ContentBlock[]`
- `stop_reason`: `end_turn | max_tokens | stop_sequence | tool_use | pause_turn | refusal`
- `stop_sequence` (if applicable)
- `usage`:
  - `input_tokens`, `output_tokens`
  - `cache_creation_input_tokens`, `cache_read_input_tokens`
  - `cache_creation` breakdown by TTL
  - `service_tier: "standard" | "priority" | "batch"`
  - `server_tool_use` counts (e.g., web search)

Output `content` blocks can include:
- `text`, `tool_use`, `thinking`, `redacted_thinking`, `server_tool_use`, `web_search_tool_result`

## Streaming behavior

Streaming uses SSE with event objects that include a `type` field. Typical sequence:

1. `message_start` (contains a message skeleton)
2. `content_block_start` (new block)
3. `content_block_delta` (block updates)
   - `text_delta`
   - `input_json_delta` (tool input streaming)
   - `citations_delta`
   - `thinking_delta`
   - `signature_delta`
4. `content_block_stop` (block complete)
5. `message_delta` (stop_reason, stop_sequence, usage updates)
6. `message_stop`

Implementations usually accumulate deltas into a final Message snapshot.

## Count tokens

`POST /v1/messages/count_tokens` accepts the same `messages`, `system`, `tools`, and `tool_choice` structure, but does not generate output. The response is:

- `{ input_tokens: number }`

This count includes text, images, documents, and tool definitions.

## Message batches (optional in SDK)

The batch API lets you submit multiple `/v1/messages` requests for asynchronous processing. Results can be retrieved later and include per-request status and output.

