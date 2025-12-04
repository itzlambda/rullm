#  rullm

## Build Commands

```bash
# Build all crates
cargo build --all

# Lint (format + clippy)
just lint

# Format only
just fmt

# Check examples compile
cargo check --examples

# Run tests
cargo test
```

## Project Structure

This is a Rust workspace with two crates:

- **rullm-core** (`crates/rullm-core/`) - Core library for LLM provider interactions
- **rullm-cli** (`crates/rullm-cli/`) - CLI binary for querying LLMs

### Core Library Architecture

The core library uses a trait-based provider system with two API levels:

1. **Simple API** - String-based, minimal configuration
2. **Advanced API** - Full control with `ChatRequestBuilder`

Key modules:
- `providers/` - Provider implementations (OpenAI, Anthropic, Google, OpenAI-compatible)
- `compat_types.rs` - OpenAI-compatible message/response types used across providers
- `config.rs` - Provider configuration traits and builders
- `error.rs` - `LlmError` enum with comprehensive error variants
- `utils/sse.rs` - Server-sent event parsing for streaming

### CLI Architecture

The CLI is organized by commands in `commands/`:
- `auth.rs` - OAuth and API key management
- `chat.rs` - Interactive chat mode with reedline
- `models.rs` - Model listing and updates
- `alias.rs` - User-defined model aliases
- `templates.rs` - TOML template management

OAuth implementation in `oauth/`:
- `openai.rs`, `anthropic.rs` - Provider-specific OAuth flows
- `server.rs` - Local callback server for OAuth redirects
- `pkce.rs` - PKCE challenge generation
