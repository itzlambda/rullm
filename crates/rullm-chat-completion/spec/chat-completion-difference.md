# OpenAI Chat Completions API compatibility across major providers

**All five providers—OpenRouter, Google Gemini, Groq, xAI (Grok), and MoonshotAI—explicitly claim OpenAI API compatibility**, but the devil is in the details. Each provider supports only the core `POST /v1/chat/completions` endpoint while omitting OpenAI's newer conversation management endpoints. Building a truly universal client requires understanding the subtle incompatibilities, parameter restrictions, and behavioral quirks unique to each provider.

## The universal truth: core endpoint only

None of the five providers support OpenAI's conversation management endpoints. The retrieve (`GET /v1/chat/completions/{id}`), list (`GET /v1/chat/completions`), update (`POST /v1/chat/completions/{id}`), delete (`DELETE /v1/chat/completions/{id}`), and list messages (`GET /v1/chat/completions/{id}/messages`) endpoints are universally unsupported. Every provider offers only the stateless `POST /v1/chat/completions` endpoint for creating completions, with some offering proprietary alternatives for conversation state management.

## Provider comparison at a glance

| Feature | OpenRouter | Gemini | Groq | xAI (Grok) | MoonshotAI |
|---------|------------|--------|------|------------|------------|
| **Compatibility Level** | Drop-in replacement | Beta, with caveats | "Mostly compatible" | Full (model-dependent) | Full |
| **Base URL** | `https://openrouter.ai/api/v1` | `https://generativelanguage.googleapis.com/v1beta/openai/` | `https://api.groq.com/openai/v1` | `https://api.x.ai/v1` | `https://api.moonshot.ai/v1` |
| **logprobs** | ✅ | ❌ | ❌ | ✅ | ❌ |
| **n > 1** | ✅ | ✅ | ❌ | ✅ | ✅ (with restrictions) |
| **Streaming + JSON** | ✅ | ✅ | ❌ | ❌ | ✅ |
| **presence_penalty** | ✅ | ✅ | ⚠️ Inactive | ⚠️ Non-reasoning only | ❓ Undocumented |
| **frequency_penalty** | ✅ | ✅ | ⚠️ Inactive | ⚠️ Non-reasoning only | ❓ Undocumented |

---

## OpenRouter delivers the most complete compatibility

OpenRouter explicitly positions itself as a "drop-in replacement for OpenAI" and comes closest to delivering on that promise. The platform normalizes requests across **300+ models** from different providers, transforming OpenAI-format tool calls for providers that don't natively support them.

### Base configuration
```python
from openai import OpenAI
client = OpenAI(
    base_url="https://openrouter.ai/api/v1",
    api_key="sk-or-v1-xxx",
    default_headers={
        "HTTP-Referer": "https://your-app.com",
        "X-Title": "Your App Name"
    }
)
```

### Model naming convention
Models require organization prefixes: `openai/gpt-4o`, `anthropic/claude-3.5-sonnet`, `google/gemini-2.0-flash-exp`. Append suffixes for variants: `:free` for free tier, `:nitro` for speed, `:extended` for longer context.

### Critical gotchas developers encounter

**Token counting uses GPT-4o tokenizer universally**, not each model's native tokenizer. The API returns normalized token counts, but billing uses native counts—expect discrepancies when monitoring usage programmatically.

**Streaming includes comment payloads** like `: OPENROUTER PROCESSING` to prevent connection timeouts. Some SSE client implementations fail to parse these correctly, causing crashes in applications like Frigate's embeddings maintainer.

**Reasoning tokens aren't exposed in standard format**. DeepSeek R1 returns `reasoning_content` in the delta, which isn't part of OpenAI's schema—tools expecting standard streaming format won't display thinking indicators.

**Response IDs differ**: OpenRouter returns `gen-xxxxxx` format versus OpenAI's `chatcmpl-xxx`.

### Parameter support matrix
| Category | Supported | Notes |
|----------|-----------|-------|
| Sampling (temperature, top_p, penalties) | ✅ Full | |
| max_tokens | ✅ | max_completion_tokens undocumented |
| stop sequences | ✅ | |
| logprobs/top_logprobs | ✅ | |
| tools/tool_choice | ✅ | Transformed for non-OpenAI providers |
| parallel_tool_calls | ✅ | Default true |
| response_format (json_object, json_schema) | ✅ | |
| stream/stream_options | ✅ | include_usage works |
| reasoning_effort | ⚠️ | Model-specific, passed to provider |

### Provider-specific extensions
OpenRouter adds powerful routing features unavailable elsewhere:
- **`models`**: Array of fallback models for automatic failover
- **`provider.order/only/ignore`**: Control which upstream providers serve requests
- **`provider.require_parameters`**: Ensure provider supports all requested parameters
- **Model variant suffixes**: `:exacto` for curated tool-calling providers

---

## Google Gemini remains in beta with notable gaps

Google launched OpenAI compatibility in November 2024, but explicitly states it's "still in beta while we extend feature support." The implementation covers core functionality but has sharp edges around schema validation and multi-turn tool calls.

### Base configuration
```python
# Gemini API (consumer)
client = OpenAI(
    api_key="YOUR_GEMINI_API_KEY",  # From Google AI Studio
    base_url="https://generativelanguage.googleapis.com/v1beta/openai/"
)

# Vertex AI (enterprise) - requires OAuth token refresh
client = OpenAI(
    api_key=credentials.token,  # Expires in 1 hour!
    base_url=f"https://aiplatform.googleapis.com/v1/projects/{project_id}/locations/global/endpoints/openapi"
)
```

### Model naming
Use bare model names for Gemini API (`gemini-2.5-flash`) or prefixed for Vertex AI (`google/gemini-2.5-flash`).

### Critical gotchas developers encounter

**Logprobs are completely unsupported** through the OpenAI compatibility layer—requests fail with "Unknown name 'logprobs': Cannot find field" despite Gemini's native API supporting them.

**Gemini 3's thought_signature requirement** breaks multi-turn tool calls. The model returns a `thought_signature` field that must be passed back with tool responses, or requests fail with "function call is missing a thought_signature." Most OpenAI-compatible clients don't preserve this field.

**Content filtering returns None messages**. When safety filters trigger, `choices[0].message` may be `None` with `finish_reason: content_filter`, crashing clients that expect a message object.

**Schema validation is stricter than OpenAI**. Unknown fields like `type` inside function objects cause "Unknown name 'type' at 'tools[0].function'" errors. Union types in JSON schema (like `str|int`) that work on OpenAI return "Request contains an invalid argument."

**The endpoint URL changed** from `/v1beta/chat/completions` to `/v1beta/openai/chat/completions`—breaking change for early adopters.

### Parameter support matrix
| Category | Supported | Notes |
|----------|-----------|-------|
| Sampling (temperature, top_p, penalties) | ✅ | |
| max_tokens/max_completion_tokens | ✅ | |
| n (multiple completions) | ✅ | |
| stop sequences | ✅ | |
| logprobs/top_logprobs | ❌ | Fails with error |
| tools/tool_choice | ✅ | Stricter validation |
| response_format | ✅ | json_object and json_schema |
| reasoning_effort | ✅ | minimal/low/medium/high/none |
| web_search_options | ✅ | Maps to GoogleSearch tool |

### Provider-specific extensions
Access Gemini-specific features via `extra_body`:
```python
response = client.chat.completions.create(
    model="gemini-2.5-flash",
    messages=[...],
    extra_body={
        "google": {
            "thinking_config": {
                "thinking_budget": 8192,
                "include_thoughts": True
            }
        }
    }
)
```

---

## Groq trades features for speed

Groq's documentation honestly states they're "**mostly compatible**" with OpenAI—not fully. The platform prioritizes inference speed on custom LPU hardware, but this comes with meaningful parameter restrictions.

### Base configuration
```python
client = OpenAI(
    base_url="https://api.groq.com/openai/v1",
    api_key=os.environ.get("GROQ_API_KEY")
)
```

### Critical gotchas developers encounter

**Multiple completions (`n > 1`) are not supported**—requests return 400 errors. This is a hard limitation, not a bug.

**Logprobs are completely unsupported**—`logprobs`, `top_logprobs`, and `logit_bias` all return 400 errors.

**JSON mode and streaming are mutually exclusive**. Setting `response_format: json_object` with `stream: true` returns "response_format does not support streaming." This breaks many agentic frameworks that expect both simultaneously.

**Penalty parameters are documented but inactive**: "presence_penalty and frequency_penalty are not yet supported by any of our models."

**Temperature 0 becomes 1e-8**—Groq converts exact zero to a near-zero value, potentially causing subtle determinism differences.

**Streaming finish_reason bug** confirmed in November 2025: some models don't properly return `finish_reason` in streaming responses, breaking OpenAI specification compliance.

### Parameter support matrix
| Category | Supported | Notes |
|----------|-----------|-------|
| temperature/top_p | ✅ | temp 0 → 1e-8 |
| presence_penalty/frequency_penalty | ⚠️ | Documented but inactive |
| max_completion_tokens | ✅ | Preferred over max_tokens |
| n (multiple completions) | ❌ | Must be 1 |
| stop sequences | ✅ | Up to 4 sequences |
| logprobs/top_logprobs/logit_bias | ❌ | Returns 400 |
| tools/tool_choice | ✅ | Max 128 functions |
| parallel_tool_calls | ✅ | |
| response_format | ✅ | No streaming with JSON |
| stream | ✅ | Not with JSON mode |
| reasoning_effort | ✅ | Model-dependent |
| service_tier | ✅ | auto/on_demand/flex/performance |

### Provider-specific extensions
Groq adds unique features for their infrastructure:
- **`reasoning_format`**: Control reasoning output (`hidden`, `raw`, `parsed`)
- **`service_tier`**: Priority levels (`flex` for lower priority/cost)
- **Built-in tools**: Web search, code execution, browser automation, Wolfram Alpha
- **Usage breakdown**: Response includes `queue_time`, `prompt_time`, `completion_time`

---

## xAI Grok requires model-specific parameter handling

xAI claims "full compatibility with the OpenAI REST API," but Grok-4 reasoning models restrict several common parameters. Successfully using Grok requires knowing which parameters work with which model generation.

### Base configuration
```python
client = OpenAI(
    api_key="xai-xxx",  # Keys start with xai- prefix
    base_url="https://api.x.ai/v1"
)
```

### Model-specific restrictions are the primary gotcha

**Grok-4 (reasoning models) don't support**:
- `presence_penalty` (returns error)
- `frequency_penalty` (returns error)  
- `stop` sequences (returns error)
- `reasoning_effort` (only grok-3-mini supports this)

**Use `max_completion_tokens` instead of `max_tokens`** for Grok-4. Many tools auto-inject `max_tokens`, causing errors.

**`stream_options` reported as unsupported** in some integrations (n8n), though basic `stream=true` works. The `stream_options.include_usage` parameter may cause "Argument not supported" errors.

**Structured outputs don't work with streaming**—must choose one or the other.

### Parameter support matrix
| Category | Supported | Notes |
|----------|-----------|-------|
| temperature/top_p | ✅ | |
| presence_penalty/frequency_penalty | ⚠️ | Grok-3 only, not reasoning models |
| max_completion_tokens | ✅ | Required for Grok-4 |
| max_tokens | ⚠️ | Works on Grok-2/3, not Grok-4 |
| n (multiple completions) | ✅ | |
| stop sequences | ⚠️ | Grok-3 only, not reasoning models |
| logprobs/top_logprobs | ✅ | |
| tools/tool_choice | ✅ | Max 128-200 functions |
| parallel_tool_calls | ✅ | Default enabled |
| response_format | ✅ | No streaming with json_schema |
| stream | ✅ | |
| reasoning_effort | ⚠️ | grok-3-mini only ("low"/"high") |

### Provider-specific extensions
xAI offers unique search and agentic capabilities:
- **`search_parameters`**: Live web/X/news search (deprecating January 2026)
- **Deferred completions**: Submit request, retrieve result later via `request_id`
- **`x-grok-conv-id` header**: Optimize prompt caching with UUID
- **Reasoning content**: `use_encrypted_content: true` for encrypted reasoning traces

---

## MoonshotAI restricts temperature range significantly

MoonshotAI (Kimi) offers "full OpenAI compatibility" from a Chinese AI company, with both global (`api.moonshot.ai`) and China (`api.moonshot.cn`) endpoints. The main constraint is a **temperature ceiling of 1.0** versus OpenAI's 2.0.

### Base configuration
```python
client = OpenAI(
    api_key="MOONSHOT_API_KEY",
    base_url="https://api.moonshot.ai/v1"  # or api.moonshot.cn for China
)
```

### Critical gotchas developers encounter

**Temperature maximum is 1.0**—values above 1 are clamped. Additionally, if `temperature < 0.3` and `n > 1`, MoonshotAI raises an exception.

**Vision requires base64 only**—`image_url` with HTTP URLs doesn't work; images must be base64-encoded.

**Reasoning content requires special access**. The `kimi-k2-thinking` model returns `reasoning_content` which isn't in OpenAI SDK types—use `hasattr(obj, "reasoning_content")` and `getattr()` to access it safely.

**5-minute request timeout**—longer reasoning or generation returns 504 errors.

### Parameter support matrix
| Category | Supported | Notes |
|----------|-----------|-------|
| temperature | ✅ | Range 0-1 only (recommend 0.6) |
| top_p | ✅ | |
| presence_penalty/frequency_penalty | ❓ | Undocumented |
| max_tokens | ✅ | Up to 32,000 for K2 |
| n | ✅ | Restricted with low temperature |
| stop sequences | ✅ | |
| logprobs/top_logprobs | ❓ | Undocumented |
| tools/tool_choice | ✅ | Up to 128 functions |
| response_format | ✅ | json_object confirmed |
| stream | ✅ | Recommended for thinking models |

### Provider-specific extensions
MoonshotAI offers unique built-in capabilities:
- **`$web_search`**: Official built-in web search tool ($0.005/call)
- **`$date`**: Get current date
- **File API**: Upload documents for extraction and OCR
- **Automatic context caching**: No configuration needed, cached tokens cost 75% less

---

## Building a universal client requires defensive coding

Based on these findings, a universal Chat Completions client should implement:

### Parameter validation by provider
```python
PROVIDER_LIMITS = {
    "groq": {"n_max": 1, "logprobs": False, "json_streaming": False},
    "xai_reasoning": {"presence_penalty": False, "frequency_penalty": False, "stop": False},
    "moonshot": {"temperature_max": 1.0},
    "gemini": {"logprobs": False}
}
```

### Graceful degradation for unsupported features
Strip unsupported parameters rather than failing. For example, remove `logprobs` for Gemini/Groq/MoonshotAI, convert `max_tokens` to `max_completion_tokens` for Grok-4.

### Handle response format variations
- OpenRouter adds `native_finish_reason` field
- Gemini may return `None` message on content filter
- xAI/MoonshotAI add `reasoning_content` field
- Groq adds `x_groq` timing metadata

### Model name translation
Each provider has unique naming conventions:
- OpenRouter: `org/model` (e.g., `openai/gpt-4o`)
- Gemini: bare names (e.g., `gemini-2.5-flash`)
- Groq: vendor prefixes or short IDs (e.g., `llama-3.3-70b-versatile`)
- xAI: version suffixes (e.g., `grok-4`, `grok-4-0709`)
- MoonshotAI: product names (e.g., `kimi-k2-0905-preview`)

---

## Conclusion

For maximum compatibility with minimal friction, **OpenRouter provides the most complete OpenAI API implementation** with automatic transformation for diverse upstream providers. However, its normalized token counting and response format additions require awareness.

**Gemini and Groq have the most significant feature gaps**—no logprobs, and Groq's inability to combine JSON mode with streaming breaks common agentic patterns.

**xAI requires model-aware parameter handling**—code that works with Grok-3 may fail on Grok-4 due to removed parameter support.

**MoonshotAI's temperature restriction** is the most limiting factor, but otherwise provides solid compatibility for standard use cases.

All providers achieve compatibility for the **80% case** of basic chat completions, streaming, and tool calling. The incompatibilities emerge in advanced features like logprobs, multiple completions, and structured output with streaming—exactly the features that power sophisticated AI applications.
