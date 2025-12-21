# OpenAI Chat Completions API

This document specifies the **Chat Completions** REST API under `POST /v1/chat/completions` (plus stored-completion CRUD endpoints), including request/response shapes, streaming (SSE), and related features (tools/function calling, structured outputs, multimodal inputs, audio, and web search). 

---

## 1) Positioning / when to use Chat Completions
OpenAI’s docs now emphasize the **Responses API** for many new integrations, but **Chat Completions** remains a supported API surface and is still the correct choice if you specifically want `/v1/chat/completions` semantics and object shapes. 

---

## 2) Base URL, auth, headers, and versioning

### 2.1 Base URL
- Base URL: `https://api.openai.com/v1` 

### 2.2 Authentication
- Header auth: `Authorization: Bearer <OPENAI_API_KEY>` 

### 2.3 Core request headers
- `Content-Type: application/json` for JSON requests. 
- Optional account routing headers:
  - `OpenAI-Organization: <org_id>` 
  - `OpenAI-Project: <project_id>` 

### 2.4 Debugging / observability headers
- Responses include a request identifier header you should log (notably `x-request-id`) to correlate failures and support requests. 
- Responses include timing information like `openai-processing-ms`. 

### 2.5 Backward compatibility expectations
- OpenAI publishes a backward-compatibility policy for the API; clients should be resilient to additive fields and new enum values. 

**Client requirement:** Implement JSON decoding as forward-compatible: ignore unknown fields; do not exhaustively match enums without a fallback. 

---

## 3) Error model, retries, and rate limits

### 3.1 Error response shape (high-level)
Errors are returned with a top-level `error` object (containing fields like `message`, `type`, and sometimes `param`/`code`), alongside standard HTTP status codes. 

### 3.2 Recommended retry policy (client-side)
- Retry only **transient** failures (commonly `429`, `500`, `503`) using exponential backoff + jitter; do **not** blindly retry `400`/`401`/`403`/`404` because they’re usually permanent for that request. 
- Always log `x-request-id` (and any client request id you add) for diagnostics. 

### 3.3 Rate limit headers
OpenAI documents rate limits and returns `x-ratelimit-*` headers (covering request and token budgets with limit/remaining/reset patterns). Your client should parse and surface these for adaptive throttling. 

---

## 4) Endpoint inventory (Chat Completions)

### 4.1 Create (optionally stream)
- `POST /v1/chat/completions` — generate an assistant response; supports streaming via SSE; can optionally store the completion. 

### 4.2 Stored completion retrieval & management (requires `store: true` on create)
- `GET /v1/chat/completions/{completion_id}` — retrieve a stored completion. 
- `GET /v1/chat/completions` — list stored completions (pagination + filters). 
- `GET /v1/chat/completions/{completion_id}/messages` — list messages from a stored completion (pagination). 
- `POST /v1/chat/completions/{completion_id}` — update metadata on a stored completion. 
- `DELETE /v1/chat/completions/{completion_id}` — delete a stored completion. 

---

## 5) `POST /v1/chat/completions` — Create

### 5.1 Primary use cases
- Standard “chat” generation: model responds to a conversation history you provide in `messages`. 
- Tool/function calling: model asks your client to call functions; client executes and feeds results back. 
- Streaming UI: token-by-token deltas over SSE. 
- Structured outputs: force JSON or schema-conformant JSON. 
- Multimodal input: images and audio in the conversation (model-dependent). 
- Web search models: model performs a web search and returns citations/annotations. 
- Persisting outputs for later retrieval: set `store: true` and then use stored completion endpoints. 

---

### 5.2 Request body (top-level fields)
**Required**
- `model: string` — model identifier. 
- `messages: array` — conversation inputs (text and/or content parts). 

**Common optional fields (sampling / stopping / token limits)**
- `temperature?: number` — sampling temperature. 
- `top_p?: number` — nucleus sampling. 
- `n?: integer` — number of choices to generate. 
- `stop?: string | string[] | null` — stop sequences; docs note model-specific support limitations (e.g., not supported by some “o” models). 
- `max_completion_tokens?: integer | null` — cap for generated tokens (including reasoning tokens where applicable). 
- `max_tokens?: integer | null` — deprecated; docs note incompatibility with some newer model families. 
- `presence_penalty?: number | null`, `frequency_penalty?: number | null` 

**Logprobs**
- `logprobs?: boolean | null` — request logprobs in output. 
- `top_logprobs?: integer | null` — number of top tokens to return (when `logprobs` is true). 

**Streaming**
- `stream?: boolean | null` — enable SSE. 
- `stream_options?: object | null` — stream options; docs show `include_usage` behavior. 

**Tools / function calling**
- `tools?: array` — tool definitions (notably function tools). 
- `tool_choice?: "none" | "auto" | "required" | object` — tool selection policy (including forcing a specific tool). 
- `parallel_tool_calls?: boolean` — whether model may emit multiple tool calls in one turn. 
- `functions` / `function_call` — deprecated tool/function fields. 

**Structured outputs**
- `response_format?: object` — JSON mode and schema mode. 

**Multimodal output**
- `modalities?: string[]` — request output modalities (model-dependent). 
- `audio?: object | null` — audio output settings when requesting audio modality. 

**Other**
- `reasoning_effort?: string` — reasoning control for supported models (docs note model-specific constraints). 
- `verbosity?: string` — output verbosity control for supported models. 
- `prediction?: object` — “predicted output” optimization payload. 
- `web_search_options?: object` — used with web-search models. 
- `service_tier?: string` — service tier selection. 
- `seed?: integer | null` — deprecated determinism hint. 
- `logit_bias?: object | null` — token-level biasing. 

**Storing & metadata**
- `store?: boolean | null` — enable stored-completion retrieval; docs warn that some large inputs (e.g., large images) may be dropped when storing. 
- `metadata?: object` — user-defined key/value metadata for stored items. 
- `user?: string` — deprecated in favor of newer identifiers (docs reference `safety_identifier` and `prompt_cache_key`). 

---

### 5.3 `messages[]` — conversation schema (client-facing)
Chat Completions are **stateless**: you send the prior turns each request (or a summarized state). 

**Message object (conceptual)**
- `role: string` — typical roles include `system`, `user`, `assistant`, and tool-related roles (exact accepted roles depend on the API mode and model). 
- `content: string | array` — either plain text or a list of typed content parts for multimodal inputs. 

**Content parts (multimodal)**
- Image input uses a content-part pattern documented in the Images/Vision guide for Chat Completions. 
- Audio input uses a content-part pattern documented in the Audio guide for Chat Completions. 

**Client requirement:** Model capabilities differ; your client should not hard-code “text only” assumptions, and should treat message `content` as a tagged union. 

---

## 6) `POST /v1/chat/completions` — Response

### 6.1 Non-streaming response object
A successful create returns a **chat completion object** containing identifiers and an array of `choices`. 

**Top-level (commonly present)**
- `id: string`
- `object: "chat.completion"`
- `created: integer` (unix seconds)
- `model: string`
- `choices: array`
- `usage: object` (token accounting) 

**Choice object (conceptual)**
- `index: integer`
- `message: { role, content, ... }`
- `finish_reason: string | null`
- `logprobs: object | null` (if requested/supported) 

**Client requirement:** Do not assume a single choice; handle `n > 1` by returning multiple candidate messages. 

---

## 7) Streaming (SSE) — `stream: true`

### 7.1 Transport / framing
Chat Completions streaming uses **Server-Sent Events** (SSE) delivering **chat completion chunk** objects incrementally. 

The OpenAI API reference for legacy streaming explicitly describes “data-only SSE” messages and a terminal `data: [DONE]` sentinel; Chat Completions streaming is documented in terms of chunk objects and deltas. For robust clients, support both: end on connection close and/or `[DONE]` sentinel if present. 

### 7.2 Chunk object shape
Each event contains a `chat.completion.chunk` object with `choices[].delta` carrying incremental text/tool-call deltas. 

Key fields:
- `object: "chat.completion.chunk"`
- `id`, `created`, `model`
- `choices[]: { index, delta, finish_reason? }`
- Optional `usage` when using `stream_options` to include usage. 

### 7.3 Delta accumulation rules (client requirement)
- Concatenate `choices[i].delta.content` fragments in-order.
- Treat tool-call deltas as structured fragments that must be assembled into a complete tool call before execution.
- Terminate a choice when `finish_reason` becomes non-null. 

---

## 8) Tools / function calling (Chat Completions)

### 8.1 Use case
Let the model request structured, executable actions (API calls, database queries, etc.) by emitting tool calls; your client executes them and returns tool outputs back into the conversation. 

### 8.2 Request fields (high-level)
- Provide available tools in `tools`.
- Control selection with `tool_choice`.
- Allow or disallow multiple calls with `parallel_tool_calls`. 

### 8.3 Response behavior (high-level)
- Assistant messages may include tool-call descriptors instead of (or in addition to) normal text content.
- Client must translate tool calls into actual executions and then append tool results as subsequent messages and call the API again. 

---

## 9) Structured outputs (`response_format`)

### 9.1 Use case
- Enforce machine-parseable JSON output (basic JSON mode) or schema-conformant JSON (structured outputs) for deterministic integration with downstream code. 

### 9.2 Modes (documented)
- JSON mode via a `response_format` object (older JSON mode).
- Schema-based structured outputs via a `response_format` object (JSON Schema). 

**Client requirement:** Treat `response_format` as a tagged union; do not assume only one sub-variant. 

---

## 10) Multimodal inputs (images + audio) and audio outputs

### 10.1 Image inputs (vision)
- Chat Completions supports image input via content parts as documented in the Images/Vision guide when `api-mode=chat`. 
- Client should support:
  - Remote URLs
  - Base64 “data:” URLs
  - Any per-image options described in the guide (e.g., detail level), model-dependent. 

### 10.2 Audio inputs and outputs
- Audio input: send base64-encoded audio as typed content parts (guide documents the required fields). 
- Audio output: request audio via `modalities` + `audio` settings (voice/format), then decode audio bytes from the response. 

**Client requirement:** Audio and image support are model-dependent; surface capability errors clearly (do not silently fall back unless the caller asked you to). 

---

## 11) Web search (Chat Completions)

### 11.1 Use case
For web-search-capable models, allow the model to retrieve information from the web before responding, returning citation annotations suitable for UI rendering and attribution. 

### 11.2 Request fields
- Use `web_search_options` alongside a web-search model. 

### 11.3 Response fields (citations / annotations)
- The Web Search tool guide documents citation annotations (including URL citations) and how to render them. 

**Client requirement:** Preserve and expose annotations separately from text so callers can render citations reliably even if the visible text format changes. 

---

## 12) Stored completions (`store: true`) — retrieval, listing, message listing, metadata update, delete

### 12.1 Use case
- Persist responses for later inspection, evaluation, or building UIs that revisit prior outputs without keeping your own transcript store. 

### 12.2 Key behaviors
- `store: true` on create enables subsequent retrieval/listing endpoints.
- Update endpoint is for metadata updates on stored items.
- Messages endpoint returns the stored message list with pagination controls. 

### 12.3 Pagination (high-level)
List endpoints support typical cursor pagination parameters like `after`, plus `limit` and `order`. 

---

## 13) Practical client requirements checklist

### 13.1 HTTP layer
- Keep-alive connections; configurable request timeout; request body size limits; gzip/deflate handling as supported by your HTTP library. 

### 13.2 JSON decoding
- Tolerate unknown fields and enum expansions; treat tagged unions (`content`, `response_format`, tools) as extensible. 

### 13.3 Streaming
- Implement a correct SSE parser:
  - parse `data:` lines into JSON chunks
  - assemble `delta` fragments per `choice.index`
  - handle both “connection close” and `[DONE]` style termination defensively. 

### 13.4 Tool calling
- Support multiple tool calls (including parallel), and tool-call assembly in both non-streaming and streaming modes. 

### 13.5 Errors & rate limits
- Parse error objects; map to typed errors; classify retryable vs permanent.
- Parse `x-ratelimit-*` headers for adaptive throttling and caller visibility. 

---

If you want, I can convert this into a single “contract” section (request/response JSON Schema-like tables for every field and nested object) purely as documentation—still no code.
