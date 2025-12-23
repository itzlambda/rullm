# Chat completion APIs: A cross-provider comparison guide

The AI API landscape has coalesced around OpenAI's design patterns, but significant differences remain beneath the surface. **Groq and OpenRouter offer near-perfect OpenAI compatibility**, while Anthropic and Google use distinct schemas that require code changes when switching providers. This guide maps the common ground and critical divergences developers need to navigate.

## The OpenAI compatibility spectrum

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

## Endpoints and authentication patterns

All five providers use REST APIs with JSON payloads, but authentication headers and endpoint paths differ substantially.

| Provider | Base URL | Endpoint | Auth Header |
|----------|----------|----------|-------------|
| **OpenAI** | `api.openai.com` | `/v1/chat/completions` | `Authorization: Bearer $KEY` |
| **Anthropic** | `api.anthropic.com` | `/v1/messages` | `x-api-key: $KEY` + `anthropic-version: 2023-06-01` |
| **Google Gemini** | `generativelanguage.googleapis.com` | `/v1beta/models/{model}:generateContent` | `x-goog-api-key: $KEY` or OAuth |
| **Groq** | `api.groq.com` | `/openai/v1/chat/completions` | `Authorization: Bearer $KEY` |
| **OpenRouter** | `openrouter.ai` | `/api/v1/chat/completions` | `Authorization: Bearer $KEY` |

Anthropic uniquely requires a version header (`anthropic-version`) on every request. Google offers two authentication paths: API keys for Google AI Studio (simpler) or OAuth/service accounts for Vertex AI (enterprise).

## Message structure diverges at the system prompt

The most impactful difference across providers is **how system prompts are handled**. OpenAI, Groq, and OpenRouter include system instructions as a message with `role: "system"`. Anthropic separates it into a top-level `system` field. Google uses `systemInstruction` as a separate object.

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

Note that Google uses `contents` instead of `messages`, `parts` instead of `content`, and `role: "model"` instead of `role: "assistant"`. These terminology differences require complete request restructuring.

## Required versus optional parameters

A subtle but critical difference: **Anthropic requires `max_tokens`** on every request, while OpenAI treats it as optional (defaulting to model maximum). This catches many developers migrating from OpenAI.

| Parameter | OpenAI | Anthropic | Gemini | Groq |
|-----------|--------|-----------|--------|------|
| `max_tokens` | Optional | **Required** | Optional (`maxOutputTokens`) | Optional |
| `temperature` | 0-2 (default 1) | 0-1 (default 1) | 0-2 (default 1) | 0-2 (default 1) |
| `top_p` | ✓ | ✓ | ✓ (`topP`) | ✓ |
| `top_k` | ✗ | ✓ | ✓ (`topK`) | ✗ |
| `frequency_penalty` | ✓ | ✗ | ✓ | ✗ |
| `presence_penalty` | ✓ | ✗ | ✓ | ✗ |

Groq notably **does not support** `frequency_penalty`, `presence_penalty`, `logprobs`, or `n > 1`—parameters common in OpenAI workflows. Requests using these will return 400 errors.

## Response structures show similar divergence

OpenAI, Groq, and OpenRouter return responses in an identical structure with a `choices` array. Anthropic returns `content` as an array of typed blocks. Google returns `candidates` with nested `content.parts`.

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

Finish reason values also differ: OpenAI uses `stop`, Anthropic uses `end_turn`, and Google uses `STOP`. Tool-triggered stops are `tool_calls` (OpenAI/Groq), `tool_use` (Anthropic), or indicated by function call content in Gemini.

## Streaming implementations vary significantly

All providers use Server-Sent Events (SSE), but event structure differs. OpenAI-compatible APIs send incremental `delta` objects and terminate with `data: [DONE]`. Anthropic uses **typed event streams** with explicit event names like `message_start`, `content_block_delta`, and `message_stop`.

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

event: content_block_delta  
data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello"}}

event: message_stop
data: {"type":"message_stop"}
```

Anthropic's approach provides richer metadata (separate events for tool use, thinking blocks) but requires different parsing logic. Google streams partial `GenerateContentResponse` objects via `streamGenerateContent?alt=sse`.

## Tool calling follows OpenAI's lead with variations

Function/tool calling has achieved reasonable standardization, with all providers supporting JSON Schema-based tool definitions. The structure is nearly identical across OpenAI, Groq, and OpenRouter. Anthropic uses `input_schema` instead of `parameters`, and Google wraps tools in a `functionDeclarations` array.

**OpenAI/Groq tool definition:**
```json
{"type": "function", "function": {"name": "get_weather", "parameters": {...}}}
```

**Anthropic tool definition:**
```json
{"name": "get_weather", "input_schema": {...}}
```

**Google Gemini tool definition:**
```json
{"functionDeclarations": [{"name": "get_weather", "parameters": {...}}]}
```

All providers support `auto`, `none`, and forced tool selection. Anthropic adds `any` (must use at least one tool). Parallel tool calls are supported by OpenAI, Groq, and Anthropic (default enabled).

## Unique features worth noting

Each provider offers distinctive capabilities beyond the baseline API:

- **OpenAI**: Structured Outputs with strict JSON Schema enforcement (`json_schema` response format), Batch API with 50% discount
- **Anthropic**: Extended thinking for Claude 4/3.7 with configurable token budgets, prompt caching with 90% cost reduction on cache hits, computer use tools
- **Google Gemini**: Built-in Google Search grounding, native code execution, video/audio/document processing up to 2 hours of video
- **Groq**: Exceptional speed (**394-1000+ tokens/second**) via custom LPU hardware, timing metrics in usage response
- **OpenRouter**: Access to **400+ models** from all providers, automatic fallbacks, model routing with `:floor` (cheapest) and `:nitro` (fastest) suffixes, zero-markup pricing

## Pricing models share structure but not rates

All providers charge per-token with separate input/output rates. Anthropic charges 90% less for cached content. Groq offers 50% off for batch processing. OpenRouter passes through provider pricing with a 5.5% fee on credit purchases.

| Provider | Example Model | Input (per 1M) | Output (per 1M) |
|----------|---------------|----------------|-----------------|
| OpenAI | GPT-4o | ~$2.50 | ~$10.00 |
| Anthropic | Claude 3.5 Sonnet | $3.00 | $15.00 |
| Google | Gemini 2.5 Flash | $0.15 | $0.60 |
| Groq | Llama 3.3 70B | $0.59 | $0.79 |

Free tiers exist for Google AI Studio, Groq, and OpenRouter (with `:free` suffix models).

## SDK availability and language support

All providers offer first-party Python and TypeScript/JavaScript SDKs. Anthropic and Google provide the broadest language coverage.

| Provider | Python | TypeScript | Go | Java | Other |
|----------|--------|------------|----|----|-------|
| OpenAI | ✓ | ✓ | Beta | — | — |
| Anthropic | ✓ | ✓ | ✓ | ✓ | Ruby, C# (beta) |
| Google | ✓ | ✓ | ✓ | ✓ | Dart, Swift, Kotlin |
| Groq | ✓ | ✓ | — | — | OpenAI SDK compatible |
| OpenRouter | ✓ Beta | ✓ | — | — | OpenAI SDK compatible |

For Groq and OpenRouter, using the OpenAI SDK with a modified base URL is the recommended approach, enabling code reuse across providers.

## Practical migration strategies

When building multi-provider applications, consider these patterns:

1. **Use OpenAI-compatible providers for easy switching**: Groq and OpenRouter can share code paths with OpenAI. Abstract only the base URL and API key.

2. **Create provider-specific adapters for Anthropic/Gemini**: The structural differences require transformation layers. Map `system` messages to Anthropic's top-level field, convert `assistant` to `model` for Gemini.

3. **Normalize on the OpenAI response format**: Parse provider responses into a common structure. OpenRouter already does this for all 400+ models.

4. **Handle parameter gaps gracefully**: Remove unsupported parameters (like `frequency_penalty` for Groq) rather than letting requests fail.

5. **Consider OpenRouter as a unification layer**: For applications needing multiple model providers, OpenRouter provides a single API surface with automatic fallbacks and model routing.

## Conclusion

The chat completion API landscape centers on OpenAI's design patterns, with Groq and OpenRouter offering true compatibility and Anthropic/Google requiring adaptation layers. The key migration hurdles are system prompt handling, required parameters (Anthropic's `max_tokens`), and response parsing differences. For maximum flexibility, applications should abstract provider-specific code behind a common interface, or leverage OpenRouter's unified gateway to access all major models through a single, consistent API.
