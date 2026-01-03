# OpenAI Chat Completions API Specification

A comprehensive specification for implementing a client for the OpenAI Chat Completions API.

---

## Table of Contents

1. [Overview](#overview)
2. [Endpoint](#endpoint)
3. [Authentication](#authentication)
4. [Request Structure](#request-structure)
5. [Message Types](#message-types)
6. [Request Parameters](#request-parameters)
7. [Response Structure](#response-structure)
8. [Streaming](#streaming)
9. [Tool/Function Calling](#toolfunction-calling)
10. [Structured Outputs](#structured-outputs)
11. [Vision (Image Input)](#vision-image-input)
12. [Audio Input/Output](#audio-inputoutput)
13. [Web Search](#web-search)
14. [Predicted Outputs](#predicted-outputs)
15. [Error Handling](#error-handling)
16. [Rate Limiting](#rate-limiting)
17. [Model Reference](#model-reference)

---

## Overview

The Chat Completions API generates model responses from a list of messages comprising a conversation. It supports text, images, and audio as inputs and can generate text, audio, and tool calls as outputs.

**Base URL:** `https://api.openai.com/v1`

---

## Endpoint

### Create Chat Completion

```
POST /chat/completions
```

Creates a model response for the given chat conversation.

---

## Authentication

All requests require an API key in the `Authorization` header:

```
Authorization: Bearer YOUR_API_KEY
```

Optional organization header:
```
OpenAI-Organization: YOUR_ORG_ID
```

---

## Request Structure

```json
{
  "model": "gpt-4o",
  "messages": [...],
  // ... additional parameters
}
```

---

## Message Types

Messages are the core of the Chat Completions API. Each message has a `role` and `content`.

### System Message
Sets context and behavioral instructions for the model.
```json
{
  "role": "system",
  "content": "You are a helpful assistant."
}
```

### Developer Message (O-Series Models Only)
Used instead of system messages for reasoning models (o1, o3, o4-mini).
```json
{
  "role": "developer",
  "content": "Instructions for the model behavior."
}
```
**Note:** When using system message with o1/o3 models, it's treated as a developer message. Don't mix both in the same request.

### User Message
Represents input from the user.

**Text only:**
```json
{
  "role": "user",
  "content": "What is the capital of France?"
}
```

**With images (multimodal):**
```json
{
  "role": "user",
  "content": [
    {"type": "text", "text": "What's in this image?"},
    {
      "type": "image_url",
      "image_url": {
        "url": "https://example.com/image.jpg",
        "detail": "auto"  // "low", "high", or "auto"
      }
    }
  ]
}
```

**With audio:**
```json
{
  "role": "user",
  "content": [
    {"type": "text", "text": "What is being said?"},
    {
      "type": "input_audio",
      "input_audio": {
        "data": "<base64-encoded-audio>",
        "format": "wav"  // or "mp3"
      }
    }
  ]
}
```

### Assistant Message
Model-generated response or injected assistant context.
```json
{
  "role": "assistant",
  "content": "The capital of France is Paris."
}
```

**With tool calls:**
```json
{
  "role": "assistant",
  "content": null,
  "tool_calls": [
    {
      "id": "call_abc123",
      "type": "function",
      "function": {
        "name": "get_weather",
        "arguments": "{\"location\": \"Paris\"}"
      }
    }
  ]
}
```

### Tool Message
Result of a tool/function call.
```json
{
  "role": "tool",
  "tool_call_id": "call_abc123",
  "content": "{\"temperature\": 22, \"condition\": \"sunny\"}"
}
```

---

## Request Parameters

### Required Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `model` | string | Model ID to use (e.g., `gpt-4o`, `gpt-4o-mini`, `o1`, `o3`) |
| `messages` | array | List of messages comprising the conversation |

### Optional Parameters - Sampling & Generation

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `max_completion_tokens` | integer | null | Maximum tokens to generate (replaces deprecated `max_tokens`). Required for o-series models. |
| `max_tokens` | integer | null | **Deprecated.** Use `max_completion_tokens` instead. Not compatible with o-series models. |
| `temperature` | number | 1.0 | Sampling temperature (0-2). Higher = more random. **Not supported for reasoning models.** |
| `top_p` | number | 1.0 | Nucleus sampling parameter. **Not supported for reasoning models.** |
| `n` | integer | 1 | Number of completions to generate. |
| `stop` | string/array | null | Up to 4 sequences where the model stops generating. |
| `presence_penalty` | number | 0.0 | Penalizes tokens based on presence in text (-2.0 to 2.0). **Not supported for reasoning models.** |
| `frequency_penalty` | number | 0.0 | Penalizes tokens based on frequency in text (-2.0 to 2.0). **Not supported for reasoning models.** |
| `logit_bias` | object | null | Map of token IDs to bias values (-100 to 100). **Not supported for reasoning models.** |

### Optional Parameters - Advanced

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `seed` | integer | null | For deterministic sampling (best effort). Monitor `system_fingerprint` for backend changes. |
| `logprobs` | boolean | false | Return log probabilities of output tokens. **Not supported for reasoning models.** |
| `top_logprobs` | integer | null | Number of most likely tokens to return (0-20). Requires `logprobs: true`. |
| `user` | string | null | Unique end-user identifier for abuse detection. |
| `stream` | boolean | false | Enable streaming responses via SSE. |
| `stream_options` | object | null | Streaming options: `{"include_usage": true}` to get token usage in final chunk. |

### Optional Parameters - Response Format

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `response_format` | object | null | Controls output format. See [Structured Outputs](#structured-outputs). |

### Optional Parameters - Tools & Functions

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `tools` | array | null | List of tools/functions the model may call. See [Tool Calling](#toolfunction-calling). |
| `tool_choice` | string/object | "auto" | Controls tool use: `"none"`, `"auto"`, `"required"`, or specific function. |
| `parallel_tool_calls` | boolean | true | Allow parallel function calls. Disable for gpt-4.1-nano-2025-04-14. |

### Optional Parameters - Reasoning Models (O-Series)

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `reasoning_effort` | string | "medium" | Reasoning depth: `"minimal"` (gpt-5 only), `"low"`, `"medium"`, `"high"`. |

### Optional Parameters - Multimodal

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `modalities` | array | ["text"] | Output types to generate: `["text"]`, `["text", "audio"]`. Audio requires audio-preview models. |
| `audio` | object | null | Audio output config: `{"voice": "alloy", "format": "wav"}`. Voices: `alloy`, `echo`, `shimmer`. Formats: `wav`, `mp3`, `pcm16` (streaming only). |

### Optional Parameters - Service & Storage

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `service_tier` | string | "auto" | Processing tier: `"auto"`, `"default"`, `"flex"` (50% cheaper, slower), `"priority"` (lower latency, premium). |
| `store` | boolean | false | Store completion for model distillation/evals. |
| `metadata` | object | null | Up to 16 key-value pairs (keys: max 64 chars, values: max 512 chars). |

### Optional Parameters - Web Search

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `web_search_options` | object | null | Enable web search for search-preview models. See [Web Search](#web-search). |

### Optional Parameters - Predicted Outputs

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `prediction` | object | null | Predicted output for faster responses. See [Predicted Outputs](#predicted-outputs). |

---

## Response Structure

### Non-Streaming Response

```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1702685778,
  "model": "gpt-4o-2024-08-06",
  "system_fingerprint": "fp_44709d6fcb",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "The capital of France is Paris.",
        "refusal": null
      },
      "logprobs": null,
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 13,
    "completion_tokens": 7,
    "total_tokens": 20,
    "prompt_tokens_details": {
      "cached_tokens": 0,
      "audio_tokens": 0
    },
    "completion_tokens_details": {
      "reasoning_tokens": 0,
      "audio_tokens": 0,
      "accepted_prediction_tokens": 0,
      "rejected_prediction_tokens": 0
    }
  },
  "service_tier": "default"
}
```

### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier for the completion |
| `object` | string | Always `"chat.completion"` |
| `created` | integer | Unix timestamp of creation |
| `model` | string | Model used for completion |
| `system_fingerprint` | string | Backend configuration fingerprint (for determinism tracking with `seed`) |
| `choices` | array | List of completion choices |
| `usage` | object | Token usage statistics |
| `service_tier` | string | Service tier used (may differ from requested) |

### Choice Object

| Field | Type | Description |
|-------|------|-------------|
| `index` | integer | Index of this choice |
| `message` | object | The generated message |
| `finish_reason` | string | Why generation stopped |
| `logprobs` | object/null | Log probability information (if requested) |

### Finish Reasons

| Value | Description |
|-------|-------------|
| `stop` | Natural stop or stop sequence reached |
| `length` | Max token limit reached |
| `tool_calls` | Model requested tool execution |
| `content_filter` | Content filtered by safety systems |
| `function_call` | **Deprecated.** Function call requested |

### Usage Object

| Field | Type | Description |
|-------|------|-------------|
| `prompt_tokens` | integer | Tokens in the prompt |
| `completion_tokens` | integer | Tokens generated |
| `total_tokens` | integer | Total tokens used |
| `prompt_tokens_details.cached_tokens` | integer | Cached prompt tokens |
| `prompt_tokens_details.audio_tokens` | integer | Audio input tokens |
| `completion_tokens_details.reasoning_tokens` | integer | Hidden reasoning tokens (o-series) |
| `completion_tokens_details.audio_tokens` | integer | Audio output tokens |
| `completion_tokens_details.accepted_prediction_tokens` | integer | Accepted predicted tokens |
| `completion_tokens_details.rejected_prediction_tokens` | integer | Rejected predicted tokens |

---

## Streaming

Enable streaming with `stream: true`. Responses are sent as Server-Sent Events (SSE).

### Request
```json
{
  "model": "gpt-4o",
  "messages": [...],
  "stream": true,
  "stream_options": {"include_usage": true}
}
```

### Response Headers
```
Content-Type: text/event-stream
Transfer-Encoding: chunked
```

### Chunk Format

```json
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","system_fingerprint":"fp_44709d6fcb","choices":[{"index":0,"delta":{"role":"assistant","content":""},"logprobs":null,"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","system_fingerprint":"fp_44709d6fcb","choices":[{"index":0,"delta":{"content":"Hello"},"logprobs":null,"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","system_fingerprint":"fp_44709d6fcb","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"stop"}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","choices":[],"usage":{"prompt_tokens":9,"completion_tokens":12,"total_tokens":21}}

data: [DONE]
```

### Key Differences from Non-Streaming

| Aspect | Non-Streaming | Streaming |
|--------|---------------|-----------|
| Object type | `chat.completion` | `chat.completion.chunk` |
| Content field | `message.content` | `delta.content` |
| Role field | `message.role` | `delta.role` (first chunk only) |
| Usage | Always included | Only with `stream_options.include_usage: true`, in final chunk |
| Termination | Single response | `data: [DONE]` event |

---

## Tool/Function Calling

Tools allow the model to call external functions. The API returns tool call requests; execution is your responsibility.

### Defining Tools

```json
{
  "model": "gpt-4o",
  "messages": [...],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "get_weather",
        "description": "Get the current weather for a location",
        "strict": true,
        "parameters": {
          "type": "object",
          "properties": {
            "location": {
              "type": "string",
              "description": "City and state, e.g., San Francisco, CA"
            },
            "unit": {
              "type": "string",
              "enum": ["celsius", "fahrenheit"]
            }
          },
          "required": ["location", "unit"],
          "additionalProperties": false
        }
      }
    }
  ],
  "tool_choice": "auto"
}
```

### Tool Choice Options

| Value | Description |
|-------|-------------|
| `"none"` | Don't call any tools |
| `"auto"` | Model decides whether to call tools |
| `"required"` | Must call at least one tool |
| `{"type": "function", "function": {"name": "my_func"}}` | Force specific function |

### Strict Mode

Setting `strict: true` ensures function calls reliably adhere to the schema. Requirements:
- `additionalProperties: false` for each object
- All fields in `properties` must be in `required`
- Optional fields: add `null` as a type option

### Tool Call Response

```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "content": null,
      "tool_calls": [
        {
          "id": "call_abc123",
          "type": "function",
          "function": {
            "name": "get_weather",
            "arguments": "{\"location\":\"Paris\",\"unit\":\"celsius\"}"
          }
        }
      ]
    },
    "finish_reason": "tool_calls"
  }]
}
```

### Providing Tool Results

```json
{
  "model": "gpt-4o",
  "messages": [
    {"role": "user", "content": "What's the weather in Paris?"},
    {
      "role": "assistant",
      "content": null,
      "tool_calls": [{"id": "call_abc123", "type": "function", "function": {"name": "get_weather", "arguments": "{\"location\":\"Paris\",\"unit\":\"celsius\"}"}}]
    },
    {
      "role": "tool",
      "tool_call_id": "call_abc123",
      "content": "{\"temperature\": 22, \"condition\": \"sunny\"}"
    }
  ]
}
```

---

## Structured Outputs

Force the model to output valid JSON matching a schema.

### JSON Mode (Basic)

```json
{
  "model": "gpt-4o",
  "messages": [...],
  "response_format": {"type": "json_object"}
}
```

### JSON Schema Mode (Strict)

```json
{
  "model": "gpt-4o",
  "messages": [...],
  "response_format": {
    "type": "json_schema",
    "json_schema": {
      "name": "person_response",
      "strict": true,
      "schema": {
        "type": "object",
        "properties": {
          "name": {"type": "string"},
          "age": {"type": "integer"},
          "email": {"type": ["string", "null"]}
        },
        "required": ["name", "age", "email"],
        "additionalProperties": false
      }
    }
  }
}
```

### Schema Requirements

- Root cannot be `anyOf` type
- All fields must be in `required`
- `additionalProperties: false` required
- Some JSON Schema keywords not supported (e.g., `format` for dates)

### Refusals

When the model refuses to generate structured output:
```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "content": null,
      "refusal": "I cannot provide information about that topic."
    }
  }]
}
```

---

## Vision (Image Input)

### Image URL

```json
{
  "role": "user",
  "content": [
    {"type": "text", "text": "What's in this image?"},
    {
      "type": "image_url",
      "image_url": {
        "url": "https://example.com/image.jpg",
        "detail": "auto"
      }
    }
  ]
}
```

### Base64 Image

```json
{
  "type": "image_url",
  "image_url": {
    "url": "data:image/jpeg;base64,{base64_encoded_data}",
    "detail": "high"
  }
}
```

### Detail Levels

| Value | Description | Token Cost |
|-------|-------------|------------|
| `low` | Fixed low resolution | Base cost (e.g., 85 tokens) |
| `high` | Full resolution, tiled | Variable based on dimensions |
| `auto` | Model decides | Variable |

### Token Calculation (High Detail)

1. Scale to fit 2048x2048 (maintaining aspect ratio)
2. Scale shortest side to 768px
3. Count 512px tiles
4. Cost = (tiles × 170) + 85 tokens (approximate)

---

## Audio Input/Output

Requires `gpt-4o-audio-preview` or `gpt-4o-mini-audio-preview` models.

### Request with Audio Output

```json
{
  "model": "gpt-4o-audio-preview",
  "modalities": ["text", "audio"],
  "audio": {
    "voice": "alloy",
    "format": "wav"
  },
  "messages": [...]
}
```

### Audio Configuration

| Field | Options | Description |
|-------|---------|-------------|
| `voice` | `alloy`, `echo`, `shimmer` | Voice for audio output |
| `format` | `wav`, `mp3`, `pcm16` | Output format. `pcm16` only for streaming. |

### Audio Input Message

```json
{
  "role": "user",
  "content": [
    {
      "type": "input_audio",
      "input_audio": {
        "data": "<base64-encoded-audio>",
        "format": "wav"
      }
    }
  ]
}
```

**Max audio file size:** 20 MB

---

## Web Search

Available with search-preview models: `gpt-4o-search-preview`, `gpt-4o-mini-search-preview`, `gpt-5-search-api`.

### Basic Request

```json
{
  "model": "gpt-4o-search-preview",
  "web_search_options": {},
  "messages": [
    {"role": "user", "content": "What happened in tech news today?"}
  ]
}
```

### With Options

```json
{
  "model": "gpt-4o-search-preview",
  "web_search_options": {
    "search_context_size": "medium",
    "user_location": {
      "type": "approximate",
      "approximate": {
        "country": "US",
        "city": "San Francisco",
        "region": "California",
        "timezone": "America/Los_Angeles"
      }
    }
  },
  "messages": [...]
}
```

### Search Context Size

| Value | Description |
|-------|-------------|
| `low` | Faster, cheaper, less accurate |
| `medium` | Default balance |
| `high` | More thorough, slower, more expensive |

### Response Annotations

```json
{
  "choices": [{
    "message": {
      "content": "According to recent news [1]...",
      "annotations": [
        {
          "type": "url_citation",
          "start_index": 28,
          "end_index": 31,
          "url": "https://example.com/article",
          "title": "Article Title"
        }
      ]
    }
  }]
}
```

---

## Predicted Outputs

Reduce latency when most output is known ahead of time. Supported on GPT-4o and GPT-4o-mini.

### Request

```json
{
  "model": "gpt-4o",
  "messages": [
    {"role": "user", "content": "Update the email address to john@new.com"},
    {"role": "user", "content": "Current data: {\"name\": \"John\", \"email\": \"john@old.com\"}"}
  ],
  "prediction": {
    "type": "content",
    "content": "{\"name\": \"John\", \"email\": \"john@old.com\"}"
  }
}
```

### Limitations

- Text-only (no audio/images)
- Not compatible with: `n > 1`, `logprobs`, `presence_penalty > 0`
- Rejected tokens are still billed
- May increase costs if predictions are poor matches

### Tracking Prediction Usage

Check `usage.completion_tokens_details`:
- `accepted_prediction_tokens`: Tokens that matched
- `rejected_prediction_tokens`: Tokens that didn't match (still billed)

---

## Error Handling

### Error Response Format

```json
{
  "error": {
    "message": "Invalid API key",
    "type": "invalid_request_error",
    "param": null,
    "code": "invalid_api_key"
  }
}
```

### HTTP Status Codes

| Code | Description | Action |
|------|-------------|--------|
| 400 | Bad Request | Fix request parameters |
| 401 | Unauthorized | Check API key |
| 403 | Forbidden | Check permissions/organization |
| 404 | Not Found | Check model ID/endpoint |
| 429 | Rate Limited | Implement exponential backoff |
| 500 | Server Error | Retry with backoff |
| 502 | Bad Gateway | Retry with backoff |
| 503 | Service Unavailable | Retry with backoff |

### Common Error Codes

| Code | Description |
|------|-------------|
| `invalid_api_key` | API key is invalid |
| `insufficient_quota` | Quota exceeded |
| `rate_limit_exceeded` | Too many requests |
| `model_not_found` | Model doesn't exist or no access |
| `context_length_exceeded` | Input too long |
| `invalid_request_error` | Malformed request |
| `content_policy_violation` | Content filtered |

### Retry Strategy

```
1. Wait: min(2^attempt * 1000ms, 60000ms) + random_jitter
2. Max attempts: 5
3. On 429: Check x-ratelimit-reset-* headers
4. On 5xx: Always retry
```

---

## Rate Limiting

### Rate Limit Types

| Type | Description |
|------|-------------|
| RPM | Requests per minute |
| RPD | Requests per day |
| TPM | Tokens per minute |
| TPD | Tokens per day |
| IPM | Images per minute |

### Response Headers

| Header | Description |
|--------|-------------|
| `x-ratelimit-limit-requests` | Max requests allowed |
| `x-ratelimit-limit-tokens` | Max tokens allowed |
| `x-ratelimit-remaining-requests` | Requests remaining |
| `x-ratelimit-remaining-tokens` | Tokens remaining |
| `x-ratelimit-reset-requests` | Time until request limit resets |
| `x-ratelimit-reset-tokens` | Time until token limit resets |

### Token Refill

Tokens refill on a rolling 60-second window, not all at once.

---

## Model Reference

### GPT-4o Series

| Model | Context Window | Max Output | Notes |
|-------|----------------|------------|-------|
| `gpt-4o` | 128k | 16k | Latest GPT-4o |
| `gpt-4o-2024-08-06` | 128k | 16k | Structured outputs support |
| `gpt-4o-mini` | 128k | 16k | Faster, cheaper |
| `gpt-4o-audio-preview` | 128k | 16k | Audio I/O support |
| `gpt-4o-search-preview` | 128k | 16k | Web search |

### GPT-4.1 Series

| Model | Context Window | Max Output | Notes |
|-------|----------------|------------|-------|
| `gpt-4.1` | 1M | ~32k | Extended context |
| `gpt-4.1-mini` | 1M | ~32k | Smaller, faster |
| `gpt-4.1-nano` | 1M | ~32k | Smallest |

### O-Series (Reasoning Models)

| Model | Context Window | Max Output | Notes |
|-------|----------------|------------|-------|
| `o1` | 200k | 100k | Advanced reasoning |
| `o1-mini` | 128k | 65k | Faster reasoning |
| `o1-preview` | 128k | 32k | Preview version |
| `o3` | 200k | 100k | Latest reasoning |
| `o3-mini` | 200k | 100k | Smaller reasoning |
| `o4-mini` | 200k | 100k | Newest mini |

### GPT-5 Series

| Model | Context Window | Max Output | Notes |
|-------|----------------|------------|-------|
| `gpt-5` | 128k+ | Varies | Flagship model |
| `gpt-5-mini` | 128k | 16k | Smaller variant |
| `gpt-5-search-api` | 128k | 16k | Web search |

### Parameter Support by Model Type

| Parameter | GPT-4o/4.1 | O-Series | Notes |
|-----------|------------|----------|-------|
| `temperature` | ✅ | ❌ | |
| `top_p` | ✅ | ❌ | |
| `presence_penalty` | ✅ | ❌ | |
| `frequency_penalty` | ✅ | ❌ | |
| `logprobs` | ✅ | ❌ | |
| `logit_bias` | ✅ | ❌ | |
| `max_tokens` | ✅ | ❌ | Deprecated |
| `max_completion_tokens` | ✅ | ✅ | Preferred |
| `reasoning_effort` | ❌ | ✅ | |
| `developer` role | ❌ | ✅ | Use `system` for GPT models |

---

## Implementation Checklist

### Core Features
- [ ] Basic text completion
- [ ] Multi-turn conversations
- [ ] System/developer messages
- [ ] Streaming with SSE parsing
- [ ] Token usage tracking

### Advanced Features
- [ ] Tool/function calling
- [ ] Structured outputs (JSON mode & JSON Schema)
- [ ] Vision (image input)
- [ ] Audio input/output
- [ ] Web search integration
- [ ] Predicted outputs

### Error Handling
- [ ] HTTP error responses
- [ ] Rate limit headers parsing
- [ ] Exponential backoff retry
- [ ] Content filter detection

### Model-Specific
- [ ] Reasoning model parameters
- [ ] Service tier selection
- [ ] Seed for reproducibility

---

## Sources

- [Chat Completions API Reference](https://platform.openai.com/docs/api-reference/chat/)
- [Chat Completions Guide](https://platform.openai.com/docs/guides/chat-completions)
- [Function Calling Guide](https://platform.openai.com/docs/guides/function-calling)
- [Structured Outputs Guide](https://platform.openai.com/docs/guides/structured-outputs)
- [Images and Vision Guide](https://platform.openai.com/docs/guides/images-vision)
- [Audio and Speech Guide](https://platform.openai.com/docs/guides/audio)
- [Web Search Guide](https://platform.openai.com/docs/guides/tools-web-search)
- [Reasoning Models Guide](https://platform.openai.com/docs/guides/reasoning)
- [Error Codes Guide](https://platform.openai.com/docs/guides/error-codes)
- [Rate Limits Guide](https://platform.openai.com/docs/guides/rate-limits)
- [Models Overview](https://platform.openai.com/docs/models)
- [OpenAI Cookbook - Using Logprobs](https://cookbook.openai.com/examples/using_logprobs)
- [OpenAI Cookbook - Reproducible Outputs](https://cookbook.openai.com/examples/reproducible_outputs_with_the_seed_parameter)

