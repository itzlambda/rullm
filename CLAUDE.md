# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`rullm` is a Rust library and CLI for interacting with multiple LLM providers (OpenAI, Anthropic, Google AI, Groq, OpenRouter). The project uses a workspace structure with two main crates:

- **rullm-core**: Core library implementing provider integrations, middleware (Tower-based), and streaming support
- **rullm-cli**: Command-line interface built on top of rullm-core

## Architecture

### Provider System

All LLM providers implement two core traits defined in `crates/rullm-core/src/types.rs`:
- `LlmProvider`: Base trait with provider metadata (name, aliases, env_key, default_base_url, available_models, health_check)
- `ChatCompletion`: Extends LlmProvider with chat completion methods (blocking and streaming)

Provider implementations are in `crates/rullm-core/src/providers/`:
- `openai.rs`: OpenAI GPT models
- `anthropic.rs`: Anthropic Claude models
- `google.rs`: Google Gemini models
- `openai_compatible.rs`: Generic provider for OpenAI-compatible APIs
- `groq.rs`: Groq provider (uses `openai_compatible`)
- `openrouter.rs`: OpenRouter provider (uses `openai_compatible`)

The `openai_compatible` provider is a generic implementation that other providers like Groq and OpenRouter extend. It uses a `ProviderIdentity` struct to define provider-specific metadata.

### Middleware Stack

The library uses Tower middleware for enterprise features (see `crates/rullm-core/src/middleware.rs`):
- Retry logic with exponential backoff
- Rate limiting
- Circuit breakers
- Timeouts
- Connection pooling

Configuration is done via `MiddlewareConfig` and `LlmServiceBuilder`.

### Simple API

`crates/rullm-core/src/simple.rs` provides a simplified string-based API (`SimpleLlmClient`, `SimpleLlmBuilder`) that wraps the advanced provider APIs for ease of use.

### CLI Architecture

The CLI entry point is `crates/rullm-cli/src/main.rs`, which:
1. Parses arguments using clap (see `args.rs`)
2. Loads configuration from `~/.config/rullm/` (see `config.rs`)
3. Dispatches to commands in `crates/rullm-cli/src/commands/`

Key CLI modules:
- `client.rs`: Creates provider clients from model strings (format: `provider:model`)
- `provider.rs`: Resolves provider names and aliases
- `config.rs`: Manages CLI configuration (models list, aliases, default model)
- `api_keys.rs`: Manages API key storage in system keychain
- `templates.rs`: TOML-based prompt templates with `{{input}}` placeholders
- `commands/chat/`: Interactive chat mode using reedline for advanced REPL features

### Model Format

Models are specified using the format `provider:model`:
- Example: `openai:gpt-4`, `anthropic:claude-3-opus-20240229`, `groq:llama-3-8b`
- The CLI resolves this via `client::from_model()` which creates the appropriate provider client

## Common Development Tasks

### Building and Running

```bash
# Build everything
cargo build --all

# Build release binary
cargo build --release

# Run the CLI (from workspace root)
cargo run -p rullm-cli -- "your query"

# Or after building
./target/debug/rullm "your query"
./target/release/rullm "your query"
```

### Testing

```bash
# Run all tests (note: some require API keys)
cargo test --all

# Run tests for specific crate
cargo test -p rullm-core
cargo test -p rullm-cli

# Run a specific test
cargo test test_name

# Check examples compile
cargo check --examples
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run clippy (linter)
cargo clippy --all-targets --all-features -- -D warnings

# Fix clippy suggestions automatically
cargo clippy --fix --all-targets --all-features
```

### Running Examples

```bash
# Run examples from rullm-core (requires API keys)
cargo run --example openai_simple
cargo run --example anthropic_simple
cargo run --example google_simple
cargo run --example openai_stream        # Streaming example
cargo run --example test_all_providers   # Test all providers at once
```

### Adding a New Provider

When adding a new provider:

1. **OpenAI-compatible providers**: Use `OpenAICompatibleProvider` with a `ProviderIdentity` in `providers/openai_compatible.rs`. See `groq.rs` or `openrouter.rs` for examples.

2. **Non-compatible providers**: Create a new file in `crates/rullm-core/src/providers/`:
   - Implement `LlmProvider` and `ChatCompletion` traits
   - Add provider config struct in `crates/rullm-core/src/config.rs`
   - Export from `providers/mod.rs` and `lib.rs`
   - Add client creation logic in `crates/rullm-cli/src/client.rs`
   - Update `crates/rullm-cli/src/provider.rs` for CLI support

3. Update `DEFAULT_MODELS` in `crates/rullm-core/src/simple.rs` if adding default model mappings

### Streaming Implementation

All providers should implement `chat_completion_stream()` returning `StreamResult<ChatStreamEvent>`. The stream emits:
- `ChatStreamEvent::Token(String)`: Each token/chunk
- `ChatStreamEvent::Done`: Completion marker
- `ChatStreamEvent::Error(String)`: Errors during streaming

See provider implementations for SSE parsing patterns using `utils::sse::sse_lines()`.

## Configuration Files

- **User config**: `~/.config/rullm/config.toml` (or system equivalent)
  - Stores: default model, model aliases, cached models list
- **Templates**: `~/.config/rullm/templates/*.toml`
- **API keys**: Stored in system keychain via `api_keys.rs`

## Important Notes

- The project uses Rust edition 2024 (rust-version 1.85+)
- Model separator changed from `/` to `:` (e.g., `openai:gpt-4` not `openai/gpt-4`)
- Chat history is persisted in `~/.config/rullm/chat_history/`
- The CLI uses `reedline` for advanced REPL features (syntax highlighting, history, multiline editing)
- In chat mode: Alt+Enter for multiline, Ctrl+O for buffer editing, `/edit` to open $EDITOR
