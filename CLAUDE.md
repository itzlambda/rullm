# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`rullm` is a Rust library and CLI for interacting with Large Language Models (LLMs). It consists of two main crates:

- **rullm-core**: The core library providing a high-performance LLM client with Tower middleware for enterprise features
- **rullm-cli**: A CLI tool built on top of rullm-core for interactive LLM usage

## Architecture

### Core Library (rullm-core)
- **Providers**: Modular provider system supporting OpenAI, Groq, OpenRouter, Anthropic, and Google AI APIs
  - OpenAI-compatible providers (OpenAI, Groq, OpenRouter) share implementation via `OpenAICompatibleProvider`
  - Easily extensible to support any OpenAI-compatible API by adding a `ProviderIdentity`
- **Middleware**: Built on Tower for retry logic, rate limiting, circuit breakers, and timeouts
- **Dual APIs**: Simple string-based API and advanced API with full control over parameters
- **Streaming**: Real-time token-by-token streaming support via async streams
- **Types**: Comprehensive type system for chat messages, requests, responses, and configuration

### CLI Application (rullm-cli)
- **Commands**: Modular command structure for chat, models, aliases, keys, templates, etc.
- **Configuration**: TOML-based config management with user-defined aliases and templates
- **Interactive Chat**: Full-featured chat mode with history, slash commands, and editor integration
- **Templates**: TOML-based prompt templates with placeholder substitution

## Development Commands

### Building
```bash
# Build the entire workspace
cargo build

# Build with release optimizations
cargo build --release

# Build specific crate
cargo build -p rullm-core
cargo build -p rullm-cli
```

### Testing
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p rullm-core
cargo test -p rullm-cli

# Run integration tests (may require API keys)
cargo test --test integration
```

### Running Examples
```bash
# Core library examples (require API keys)
cargo run --example openai_simple
cargo run --example groq_simple
cargo run --example openrouter_simple
cargo run --example anthropic_stream
cargo run --example gemini_stream
cargo run --example test_all_providers

# CLI binary
cargo run -- "What is Rust?"
cargo run -- chat --model claude
cargo run -- chat --model groq/llama-3.3-70b-versatile
```

### Linting and Formatting
```bash
# Check code formatting
cargo fmt --check

# Format code
cargo fmt

# Run clippy lints
cargo clippy

# Run clippy with all targets
cargo clippy --all-targets --all-features
```

## Key Patterns and Conventions

### Provider Implementation
- All providers implement the `ChatProvider` trait with `chat_completion` and `chat_completion_stream` methods
- Configuration structs follow the pattern `{Provider}Config` (e.g., `OpenAIConfig`, `AnthropicConfig`)
  - OpenAI-compatible providers use `OpenAICompatibleConfig` with factory methods (`.groq()`, `.openrouter()`)
- Provider structs follow the pattern `{Provider}Provider` (e.g., `OpenAIProvider`, `GroqProvider`, `OpenRouterProvider`)
  - OpenAI-compatible providers wrap `OpenAICompatibleProvider` with different `ProviderIdentity` metadata

### Error Handling
- All public APIs return `Result<T, LlmError>` for comprehensive error handling
- LlmError enum covers authentication, rate limiting, network issues, and provider-specific errors
- Streaming APIs emit `ChatStreamEvent` enum variants: `Token`, `Done`, `Error`

### Configuration Management
- CLI config stored in `~/.config/rullm/` (or platform equivalent)
- Templates stored as TOML files in `templates/` subdirectory
- Model aliases defined in config.toml for user convenience

### Testing
- Unit tests co-located with implementation files
- Integration tests in `tests/` directories
- Examples serve as both documentation and integration tests
- Test helpers in `utils/test_helpers.rs` for common test patterns

## Important Files

- `crates/rullm-core/src/lib.rs` - Main library entry point and public API
- `crates/rullm-core/src/types.rs` - Core type definitions for requests/responses
- `crates/rullm-core/src/providers/` - LLM provider implementations
  - `openai_compatible.rs` - Shared implementation for OpenAI-compatible APIs (OpenAI, Groq, OpenRouter)
  - `openai.rs`, `groq.rs`, `openrouter.rs` - Provider wrappers with specific identities
  - `anthropic.rs`, `google.rs` - Provider-specific implementations
- `crates/rullm-cli/src/main.rs` - CLI entry point and argument parsing
- `crates/rullm-cli/src/commands/` - CLI command implementations
- `crates/rullm-cli/src/config.rs` - Configuration management

## Development Notes

- The project uses Rust 2024 edition with MSRV 1.85
- Tower middleware provides enterprise-grade reliability features
- Async/await throughout with tokio runtime
- Comprehensive error handling and observability via metrics and logging
- Shell completion support for bash, zsh, and fish