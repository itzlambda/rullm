# Chat Completion APIs: Cross-Provider Comparison Guide

The AI API landscape has coalesced around OpenAI's design patterns, but significant differences remain beneath the surface. This guide maps the common ground and critical divergences developers need to navigate when building multi-provider applications.

## OpenAI Compatibility Spectrum

Three providers—Groq, OpenRouter, and OpenAI itself—share an identical request/response schema, making code portability straightforward. Anthropic and Google Gemini diverge significantly, each with unique terminology and structural choices.

| Provider | OpenAI Compatible | Migration Complexity |
|----------|-------------------|---------------------|
| **OpenAI** | Baseline reference | N/A |
| **Groq** | Yes (drop-in) | Change base URL + API key |
| **OpenRouter** | Yes (drop-in) | Change base URL + API key |
| **Anthropic** | No | Requires schema rewrite |
| **Google Gemini** | No | Requires schema rewrite |

To use OpenAI's Python SDK with Groq or OpenRouter, only the base URL changes:

```python
from openai import OpenAI
client = OpenAI(
    base_url="https://api.groq.com/openai/v1",  # or "https://openrouter.ai/api/v1"
    api_key="YOUR_API_KEY"
)
```

### Design Philosophy by Provider

- **Anthropic**: Safety-first with strict schema enforcement. Enforces alternating user/assistant roles to prevent jailbreaking. System prompts elevated to top-level parameter for higher authority. Version header required for enterprise stability.
- **Google Gemini**: Multimodal-native. Uses `parts`-based architecture treating text, images, video, and audio as equivalent units. Designed for enterprise cloud integration via Vertex AI.
- **Groq**: Velocity-centric. Mimics OpenAI specification exactly for frictionless adoption—drop-in compatibility prioritized over architectural novelty.
- **OpenRouter**: Normalization layer. Abstracts ecosystem fragmentation behind unified OpenAI-compatible interface with routing intelligence.

## Endpoints and Authentication

All providers use REST APIs with JSON payloads, but authentication headers and endpoint paths differ.

| Provider | Base URL | Endpoint | Auth Header |
|----------|----------|----------|-------------|
| **OpenAI** | `api.openai.com` | `/v1/chat/completions` | `Authorization: Bearer $KEY` |
| **Anthropic** | `api.anthropic.com` | `/v1/messages` | `x-api-key: $KEY` + `anthropic-version: 2023-06-01` |
| **Google Gemini** | `generativelanguage.googleapis.com` | `/v1beta/models/{model}:generateContent` | `x-goog-api-key: $KEY` or OAuth |
| **Groq** | `api.groq.com` | `/openai/v1/chat/completions` | `Authorization: Bearer $KEY` |
| **OpenRouter** | `openrouter.ai` | `/api/v1/chat/completions` | `Authorization: Bearer $KEY` |

**Key differences:**
- Anthropic requires `anthropic-version` header on every request—forces clients to pin to specific schema version
- OpenRouter accepts optional `HTTP-Referer` and `X-Title` headers for community rankings

### Google's Bifurcated Authentication

Google offers two distinct authentication paths:

1. **Google AI Studio (Prototyping)**: Simple API key via `x-goog-api-key` header
2. **Vertex AI (Enterprise)**: Google Cloud IAM with OAuth 2.0 access tokens via Service Accounts

Code written for AI Studio often requires significant refactoring for Vertex AI deployment—a friction point absent with other providers.

## Request Structure

### Message Format

The most impactful difference is **how system prompts are handled**.

**OpenAI/Groq/OpenRouter format:**
```json
{
  "model": "gpt-4o",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ]
}
```

**Anthropic format:**
```json
{
  "model": "claude-sonnet-4-5",
  "max_tokens": 1024,
  "system": "You are a helpful assistant.",
  "messages": [
    {"role": "user", "content": "Hello!"}
  ]
}
```

**Google Gemini format:**
```json
{
  "systemInstruction": {"parts": [{"text": "You are a helpful assistant."}]},
  "contents": [
    {"role": "user", "parts": [{"text": "Hello!"}]}
  ]
}
```

**Terminology mapping:**
| Concept | OpenAI/Groq/OpenRouter | Anthropic | Gemini |
|---------|------------------------|-----------|--------|
| Message list | `messages` | `messages` | `contents` |
| Message content | `content` | `content` | `parts` |
| Assistant role | `assistant` | `assistant` | `model` |
| System prompt | Message with `role: system` | Top-level `system` | `systemInstruction` |

**Anthropic strict alternation**: Anthropic enforces rigorous alternation between user and assistant roles. A sequence of `user, user` is invalid (400 error). Client must merge consecutive same-role messages.

### Control Parameters

| Parameter | OpenAI | Anthropic | Gemini | Groq |
|-----------|--------|-----------|--------|------|
| `max_tokens` | Optional | **Required** | Optional (`maxOutputTokens`) | Optional |
| `temperature` | 0-2 (default 1) | 0-1 (default 1) | 0-2 (default 1) | 0-2 (default 1) |
| `top_p` | ✓ | ✓ | ✓ (`topP`) | ✓ |
| `top_k` | ✗ | ✓ | ✓ (`topK`) | ✗ |
| `frequency_penalty` | ✓ | ✗ | ✓ | ✗ |
| `presence_penalty` | ✓ | ✗ | ✓ | ✗ |

**Critical**: Anthropic requires `max_tokens` on every request—catches many developers migrating from OpenAI.

**Groq limitations**: Does not support `frequency_penalty`, `presence_penalty`, `logprobs`, or `n > 1`. Requests using these return 400 errors.

### Thinking/Reasoning Configuration

- **Anthropic (Claude 3.7+)**: `thinking` block with `budget_tokens` parameter—explicitly reserves capacity for chain-of-thought
- **Gemini 2.0**: `thinking_config` with levels (`"low"`, `"high"`)—reasoning depth as configuration toggle

## Response Structure

**OpenAI/Groq/OpenRouter response:**
```json
{
  "id": "chatcmpl-abc123",
  "choices": [{
    "index": 0,
    "message": {"role": "assistant", "content": "Hello!"},
    "finish_reason": "stop"
  }],
  "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
}
```

**Anthropic response:**
```json
{
  "id": "msg_01XFD...",
  "content": [{"type": "text", "text": "Hello!"}],
  "stop_reason": "end_turn",
  "usage": {"input_tokens": 10, "output_tokens": 5}
}
```

**Google Gemini response:**
```json
{
  "candidates": [{
    "content": {"parts": [{"text": "Hello!"}], "role": "model"},
    "finishReason": "STOP"
  }],
  "usageMetadata": {"promptTokenCount": 10, "candidatesTokenCount": 5}
}
```

### Stop/Finish Reasons

| Reason | OpenAI/Groq | Anthropic | Gemini |
|--------|-------------|-----------|--------|
| Natural completion | `stop` | `end_turn` | `STOP` |
| Token limit | `length` | `max_tokens` | `MAX_TOKENS` |
| Tool call | `tool_calls` | `tool_use` | (function call in content) |
| Content filter | `content_filter` | — | `SAFETY` |
| Copyright | — | — | `RECITATION` |

**Gemini-specific**: `RECITATION` triggers when output is too similar to copyrighted training data, blocking the response.

## Streaming

All providers use Server-Sent Events (SSE), but event structure differs significantly.

**OpenAI/Groq streaming chunk:**
```
data: {"choices":[{"delta":{"content":"Hello"}}]}
data: {"choices":[{"delta":{"content":" there"}}]}
data: [DONE]
```

**Anthropic streaming events:**
```
event: message_start
data: {"type":"message_start","message":{...}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text"}}

event: content_block_delta
data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello"}}

event: message_stop
data: {"type":"message_stop"}
```

Anthropic's verbose event system provides richer metadata—separate events for tool use, thinking blocks—but requires different parsing logic.

**Gemini**: Streams partial `GenerateContentResponse` objects via `streamGenerateContent?alt=sse`. May emit "empty" chunks containing only citation metadata or safety ratings—client must filter for actual text content.

## Tool Use / Function Calling

All providers support JSON Schema-based tool definitions, but schemas differ.

### Tool Definitions

**OpenAI/Groq/OpenRouter:**
```json
{"type": "function", "function": {"name": "get_weather", "parameters": {...}}}
```

**Anthropic:**
```json
{"name": "get_weather", "input_schema": {...}}
```

**Google Gemini:**
```json
{"functionDeclarations": [{"name": "get_weather", "parameters": {...}}]}
```

Note: Anthropic uses `input_schema` instead of `parameters`.

### Tool Invocation (Model's Request)

**OpenAI/Groq/OpenRouter** — `tool_calls` array with unique `id`:
```json
{"tool_calls": [{"id": "call_abc", "function": {"name": "get_weather", "arguments": "{\"city\":\"London\"}"}}]}
```

**Anthropic** — `tool_use` content block (can follow text blocks for chain-of-thought):
```json
{"type": "tool_use", "id": "toolu_01...", "name": "get_weather", "input": {"city": "London"}}
```

**Gemini** — `functionCall` part:
```json
{"parts": [{"functionCall": {"name": "get_weather", "args": {"city": "London"}}}]}
```

### Result Submission (Client's Response)

**OpenAI/Groq/OpenRouter** — Dedicated `tool` role:
```json
{"role": "tool", "tool_call_id": "call_abc", "content": "25°C"}
```

**Anthropic** — `tool_result` block in `user` message:
```json
{"role": "user", "content": [{"type": "tool_result", "tool_use_id": "toolu_01...", "content": "25°C"}]}
```

**Gemini** — `functionResponse` part:
```json
{"role": "user", "parts": [{"functionResponse": {"name": "get_weather", "response": {"result": "25°C"}}}]}
```

**Key difference**: Anthropic has no separate "tool" role—the user reports tool results.

### Tool Choice Options

All providers support `auto`, `none`, and forced tool selection. Anthropic adds `any` (must use at least one tool). Parallel tool calls supported by OpenAI, Groq, and Anthropic (default enabled).

## Multimodality

### Image Transmission

**Anthropic** — Base64 encoding (bandwidth-intensive):
```json
{"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": "..."}}
```

**Groq** — OpenAI `image_url` format (URL or base64):
```json
{"type": "image_url", "image_url": {"url": "https://..." }}
```

**Gemini** — Supports inline `inline_data` (base64) or `file_data` (Cloud Storage URI):
```json
{"file_data": {"mime_type": "video/mp4", "file_uri": "gs://my-bucket/video.mp4"}}
```

Gemini's `file_data` allows processing hours of video/audio—impossible via base64.

### Media Support Matrix

| Media Type | OpenAI | Anthropic | Gemini | Groq |
|------------|--------|-----------|--------|------|
| Images | ✓ | ✓ | ✓ | ✓ (model-dependent) |
| Video | ✗ | ✗ (frames as images) | ✓ Native | ✗ |
| Audio | ✓ | ✗ | ✓ Native | Separate endpoint |
| Documents (PDF) | ✓ | ✓ | ✓ | ✗ |

Gemini is currently the only provider supporting native video and audio in the main chat endpoint.

## Rate Limits

Rate limit communication varies significantly.

| Provider | Headers | Notes |
|----------|---------|-------|
| **Groq** | `x-ratelimit-*` | Standard (requests, tokens, reset time) |
| **Anthropic** | `anthropic-ratelimit-*` | Separates input-tokens from output-tokens limits |
| **OpenRouter** | `x-openrouter-credits` | Credit-based |
| **Gemini** | N/A | Uses Google Cloud Quota dashboards; 429 errors contain `retry-after` |

## Pricing

All providers charge per-token with separate input/output rates.

| Provider | Example Model | Input (per 1M) | Output (per 1M) |
|----------|---------------|----------------|-----------------|
| OpenAI | GPT-4o | ~$2.50 | ~$10.00 |
| Anthropic | Claude 3.5 Sonnet | $3.00 | $15.00 |
| Google | Gemini 2.5 Flash | $0.15 | $0.60 |
| Groq | Llama 3.3 70B | $0.59 | $0.79 |

**Discounts**: Anthropic offers 90% off for cached content. Groq offers 50% off for batch processing.

**Free tiers**: Google AI Studio, Groq, and OpenRouter (`:free` suffix models).

## Unique Provider Features

- **OpenAI**: Structured Outputs with strict JSON Schema enforcement, Batch API with 50% discount
- **Anthropic**: Extended thinking with configurable token budgets, prompt caching (90% cost reduction), computer use tools
- **Google Gemini**: Built-in Google Search grounding, native code execution, 2M token context, video/audio processing up to 2 hours
- **Groq**: 394-1000+ tokens/second via LPU hardware, timing metrics in usage response
- **OpenRouter**: 400+ models, automatic fallbacks, model routing (`:floor` cheapest, `:nitro` fastest), zero-markup pricing

## SDK Availability

| Provider | Python | TypeScript | Go | Java | Other |
|----------|--------|------------|----|----|-------|
| OpenAI | ✓ | ✓ | Beta | — | — |
| Anthropic | ✓ | ✓ | ✓ | ✓ | Ruby, C# (beta) |
| Google | ✓ | ✓ | ✓ | ✓ | Dart, Swift, Kotlin |
| Groq | ✓ | ✓ | — | — | OpenAI SDK compatible |
| OpenRouter | ✓ Beta | ✓ | — | — | OpenAI SDK compatible |

For Groq and OpenRouter, using the OpenAI SDK with modified base URL is recommended.

## Migration Strategies

### Practical Patterns

1. **Use OpenAI-compatible providers for easy switching**: Groq and OpenRouter share code paths with OpenAI. Abstract only base URL and API key.

2. **Create provider-specific adapters for Anthropic/Gemini**: Structural differences require transformation layers. Map `system` messages to Anthropic's top-level field, convert `assistant` to `model` for Gemini.

3. **Normalize on OpenAI response format**: Parse provider responses into common structure. OpenRouter already does this for 400+ models.

4. **Handle parameter gaps gracefully**: Remove unsupported parameters (like `frequency_penalty` for Groq) rather than letting requests fail.

5. **Consider OpenRouter as unification layer**: For multi-provider needs, provides single API surface with automatic fallbacks.

### Strategic Recommendations

| Use Case | Recommended Provider |
|----------|---------------------|
| Safety-critical, complex instructions | Anthropic (strict schema, prompt caching) |
| Heavy media analysis (video/audio) | Gemini (native support, 2M context) |
| Real-time, latency-critical | Groq (LPU speed) |
| Multi-model access, reduced operational overhead | OpenRouter (unified gateway) |

## Summary

The Chat Completion API has evolved into a standard architectural pattern, but implementation remains fragmented. Key migration hurdles:

1. **System prompt handling** — Message vs top-level parameter vs nested config
2. **Required parameters** — Anthropic's mandatory `max_tokens`
3. **Response parsing** — `choices` vs `content` vs `candidates`
4. **Tool use handshakes** — Different roles and ID handling

For maximum flexibility, abstract provider-specific code behind a common interface, or leverage OpenRouter's unified gateway.
