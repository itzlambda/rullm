# LLM Provider Examples

This directory contains examples demonstrating how to use the OpenAI, Anthropic, and Google AI providers in the LLM library.

## Prerequisites

1. **Set up your API keys:**
   ```bash
   # OpenAI
   export OPENAI_API_KEY="sk-your-actual-api-key-here"
   export OPENAI_ORGANIZATION="org-123"  # Optional
   export OPENAI_PROJECT="proj-456"      # Optional
   export OPENAI_BASE_URL="https://custom-endpoint.com/v1"  # Optional
   
   # Anthropic
   export ANTHROPIC_API_KEY="sk-ant-your-actual-api-key"
   export ANTHROPIC_BASE_URL="https://custom-endpoint.com"  # Optional
   
   # Google AI
   export GOOGLE_AI_API_KEY="your-google-ai-api-key"
   export GOOGLE_AI_BASE_URL="https://custom-endpoint.com"  # Optional
   ```

2. **Install dependencies:**
   ```bash
   cargo build
   ```

## Streaming Examples

The streaming API allows you to receive responses in real-time as tokens are generated, providing a more interactive experience for chat applications.

### Overview

All streaming examples use the `chat_completion_stream` method which returns a `StreamResult<ChatStreamEvent>` - a stream of events including tokens, completion signals, and errors. The streaming API uses the same request builders as regular completions but with `.stream(true)` enabled.

### 1. OpenAI Streaming (`openai_stream.rs`)

**Run:** `cargo run --example openai_stream`

**Environment:** Requires `OPENAI_API_KEY`

Demonstrates comprehensive OpenAI streaming with:
- **Simple streaming chat** with real-time token display
- **Multi-turn conversations** with context preservation  
- **Creative writing** with high temperature settings
- **Error handling** for invalid models and network issues
- **Token counting** and performance metrics

**Code snippet:**
```rust
let request = ChatRequestBuilder::new()
    .system("You are a helpful assistant.")
    .user("Tell me a short joke about programming.")
    .temperature(0.7)
    .max_tokens(100)
    .stream(true) // Enable streaming
    .build();

let mut stream = provider
    .chat_completion_stream(request, "gpt-3.5-turbo", None)
    .await;

while let Some(event) = stream.next().await {
    match event? {
        ChatStreamEvent::Token(token) => {
            print!("{}", token);
            std::io::Write::flush(&mut std::io::stdout())?;
        }
        ChatStreamEvent::Done => {
            println!("\n✅ Stream completed");
            break;
        }
        ChatStreamEvent::Error(error) => {
            println!("\n❌ Stream error: {}", error);
            break;
        }
    }
}
```

**Models used:** `gpt-3.5-turbo`, `gpt-4o-mini`, `gpt-4`

### 2. Anthropic Claude Streaming (`anthropic_stream.rs`)

**Run:** `cargo run --example anthropic_stream`

**Environment:** Requires `ANTHROPIC_API_KEY`

Showcases Claude's capabilities with:
- **Philosophical conversations** demonstrating reasoning abilities
- **Creative storytelling** with vivid imagery
- **Code explanation** with technical accuracy
- **Model comparison** across Claude variants
- **Word counting** and content analysis

**Code snippet:**
```rust
let request = ChatRequestBuilder::new()
    .system("You are Claude, a helpful and thoughtful AI assistant.")
    .user("Explain quantum computing in simple terms.")
    .temperature(0.7)
    .max_tokens(150)
    .stream(true)
    .build();

let mut stream = provider
    .chat_completion_stream(request, "claude-3-haiku-20240307", None)
    .await;

// Handle streaming events...
```

**Models used:** `claude-3-haiku-20240307`, `claude-3-sonnet-20240229`, `claude-3-5-sonnet-20241022`, `claude-3-opus-20240229`

**Temperature settings:**
- Technical content: 0.1-0.4 for accuracy
- Creative content: 0.7-1.0 for variety
- Balanced conversation: 0.6-0.7

### 3. Google Gemini Streaming (`gemini_stream.rs`)

**Run:** `cargo run --example gemini_stream`

**Environment:** Requires `GOOGLE_API_KEY`

Highlights Gemini's versatility:
- **Technical explanations** with precision
- **Creative writing** using experimental models
- **Code analysis** and review capabilities
- **Model comparison** between Gemini variants
- **Sentence counting** and response analysis

**Code snippet:**
```rust
let request = ChatRequestBuilder::new()
    .system("You are a helpful AI assistant built by Google.")
    .user("Explain machine learning in simple terms.")
    .temperature(0.7)
    .max_tokens(150)
    .stream(true)
    .build();

let mut stream = provider
    .chat_completion_stream(request, "gemini-1.5-flash", None)
    .await;

// Handle streaming events...
```

**Models used:** 
- `gemini-1.5-flash` (fast responses)
- `gemini-1.5-pro` (balanced performance)  
- `gemini-2.0-flash-exp` (experimental features)

### Streaming API Patterns

**Event handling:**
```rust
while let Some(event) = stream.next().await {
    match event? {
        ChatStreamEvent::Token(token) => {
            // Display token immediately
            print!("{}", token);
            std::io::Write::flush(&mut std::io::stdout())?;
        }
        ChatStreamEvent::Done => {
            // Stream completed successfully
            println!("\n✅ Completed");
            break;
        }
        ChatStreamEvent::Error(error) => {
            // Handle stream-specific errors
            println!("\n❌ Error: {}", error);
            break;
        }
    }
}
```

**Real-time display:**
- Use `print!()` instead of `println!()` for tokens
- Call `std::io::Write::flush()` after each token for immediate display
- Handle partial words and unicode characters gracefully

**Error handling:**
- Network errors are yielded as `Err(LlmError)`
- API errors come as `ChatStreamEvent::Error(String)`
- Always check for both error types in production code

**Performance tips:**
- Use faster models (flash variants) for better streaming experience
- Set appropriate `max_tokens` to prevent long responses
- Consider `top_p` parameter for controlled randomness
- Lower temperature (0.1-0.4) for consistent streaming

### Testing Streaming Examples

```bash
# Build all examples to verify compilation
cargo build --examples

# Test individual streaming examples
cargo run --example openai_stream     # Requires OPENAI_API_KEY
cargo run --example anthropic_stream  # Requires ANTHROPIC_API_KEY  
cargo run --example gemini_stream     # Requires GOOGLE_API_KEY

# Run lint checks
cargo clippy --all-targets --all-features
```

## Examples

### 1. Basic Usage (`openai_basic.rs`)

**Run:** `cargo run --example openai_basic`

Demonstrates:
- Using ConfigBuilder for environment-based configuration
- Basic chat completion request
- Token usage tracking

```rust
// Configuration using ConfigBuilder (recommended)
let config = ConfigBuilder::openai_from_env()?;
let provider = OpenAIProvider::new(config)?;

// Simple request
let request = ChatRequestBuilder::new()
    .system("You are a helpful assistant.")
    .user("What is 2+2?")
    .temperature(0.7)
    .build();

let response = provider.chat_completion(request, "gpt-4").await?;
```

### 2. Simple Examples (`openai_simple.rs`)

**Run:** `cargo run --example openai_simple`

Demonstrates:
- Multiple conversation patterns
- Different models comparison
- Advanced parameter usage (temperature, top_p, penalties)
- Error handling

Key features:
- **Multi-message conversations** with context
- **Model comparison** between GPT-3.5-turbo and GPT-4o-mini
- **Creative writing** with high temperature settings
- **Parameter experimentation** (frequency_penalty, presence_penalty, top_p)

### 3. Configuration Examples (`openai_config.rs`)

**Run:** `cargo run --example openai_config`

Demonstrates:
- Different configuration options
- Organization and project settings
- Custom base URLs (useful for proxies/Azure OpenAI)
- Configuration validation
- Error handling patterns
- Request builder patterns

Key features:
- **Environment-based configuration**
- **Custom endpoints** for enterprise setups
- **Validation and error handling**
- **Health checks** and model availability
- **Request builder patterns** from minimal to full-featured

## Usage Patterns

### Configuration Options

**Recommended: Use ConfigBuilder for environment-based config:**

```rust
use llm_core::config::ConfigBuilder;

// Automatically reads OPENAI_API_KEY, OPENAI_ORGANIZATION, OPENAI_PROJECT, OPENAI_BASE_URL
let config = ConfigBuilder::openai_from_env()?;
let provider = OpenAIProvider::new(config)?;
```

**Alternative: Manual configuration:**

```rust
use llm_core::config::OpenAIConfig;

// Manual configuration
let config = OpenAIConfig::new("sk-your-key")
    .with_organization("org-123")  
    .with_project("proj-456")
    .with_base_url("https://your-custom-endpoint.com/v1");
```

### Request Building

```rust
// Minimal
let request = ChatRequestBuilder::new("gpt-3.5-turbo")
    .user("Hello!")
    .build();

// Full-featured
let request = ChatRequestBuilder::new("gpt-4")
    .system("You are a helpful assistant.")
    .user("Question 1")
    .assistant("Answer 1")
    .user("Question 2")
    .temperature(0.7)
    .max_tokens(150)
    .top_p(0.9)
    .frequency_penalty(0.1)
    .presence_penalty(0.1)
    .stop_sequences(vec!["END".to_string()])
    .build();
```

### Error Handling

```rust
match provider.chat_completion(request).await {
    Ok(response) => {
        println!("Response: {}", response.message.content);
        println!("Tokens: {}", response.usage.total_tokens);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
        // Handle different error types
    }
}
```

## API Reference

### Core Types

- **`OpenAIProvider`** - Main provider implementation
- **`OpenAIConfig`** - Configuration for OpenAI API
- **`ChatRequestBuilder`** - Fluent builder for chat requests
- **`ChatRequest`** - Chat completion request
- **`ChatResponse`** - Chat completion response

### Key Methods

- **`provider.chat_completion(request)`** - Send chat completion
- **`provider.health_check()`** - Test API connectivity
- **`provider.available_models()`** - Get supported models
- **`config.validate()`** - Validate configuration

### Supported Models

- `gpt-4o`
- `gpt-4o-mini`
- `gpt-4-turbo`
- `gpt-4`
- `gpt-3.5-turbo`

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `temperature` | `f32` | Controls randomness (0.0-2.0) |
| `max_tokens` | `u32` | Maximum response length |
| `top_p` | `f32` | Nucleus sampling (0.0-1.0) |
| `frequency_penalty` | `f32` | Reduce repetition (-2.0 to 2.0) |
| `presence_penalty` | `f32` | Encourage new topics (-2.0 to 2.0) |
| `stop` | `Vec<String>` | Stop sequences |

## Tips

1. **Start with `openai_simple.rs`** for basic understanding
2. **Use `gpt-3.5-turbo`** for faster/cheaper testing
3. **Set lower temperature** (0.1-0.3) for consistent responses
4. **Set higher temperature** (0.8-1.0) for creative tasks
5. **Use `max_tokens`** to control costs
6. **Test with `health_check()`** before making requests

## Troubleshooting

**"API key not found"**: Make sure `OPENAI_API_KEY` environment variable is set

**"Invalid API key"**: Ensure your API key starts with `sk-` and is valid

**"Rate limit"**: Add delays between requests or reduce concurrency

**"Model not found"**: Check that the model name is supported (see list above)

## Test All Providers (`test_all_providers.rs`)

Comprehensive test that validates all LLM providers and their `available_models` functionality:

```bash
# Set up your API keys
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GOOGLE_API_KEY="..."

# Run the comprehensive test
cargo run --example test_all_providers
```

**Features:**
- Tests OpenAI, Anthropic, and Google providers
- Calls `available_models()` for each provider
- Validates expected model patterns
- Performs health checks
- Provides detailed success/failure reporting
- Gracefully handles missing API keys

**Sample Output:**
```
🚀 Testing All LLM Providers and Their Available Models

🔍 Testing OpenAI Provider...
   Provider name: openai
   Health check: ✅ Passed
   Expected model 'gpt-4': ✅ Found
   Expected model 'gpt-3.5-turbo': ✅ Found
✅ OpenAI: Found 5 models

📊 SUMMARY:
┌─────────────┬────────┬─────────────┐
│ Provider    │ Status │ Models      │
├─────────────┼────────┼─────────────┤
│ OpenAI      │ ✅ Pass │ 5 models    │
│ Anthropic   │ ✅ Pass │ 5 models    │
│ Google      │ ✅ Pass │ 5 models    │
└─────────────┴────────┴─────────────┘

🎉 All providers are working correctly!
```

This example is perfect for:
- Verifying your API keys work
- Testing network connectivity
- Validating provider implementations
- CI/CD pipeline health checks 