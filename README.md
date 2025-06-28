# rullm

Yet another Rust library and CLI for interacting with LLMs.

## 🚀 Quick Start

### CLI Usage

```bash
# Basic query
rullm "What is the capital of France?"

# Use different models with aliases
rullm --model gpt4 "Explain quantum computing"
rullm --model claude "Write a poem about the ocean"
rullm --model gemini "What's the weather like?"

# Interactive chat
rullm chat --model claude

rullm "Tell me a story"
rullm --model gpt4 "Explain quantum computing in detail"
rullm chat --model claude

# Disable streaming for buffered output
rullm --no-streaming "Write a poem about the ocean"
rullm chat --no-streaming --model gemini

# Set up your API keys
rullm keys set openai
export OPENAI_API_KEY="your-key-here"
```
## 🔧 CLI Commands

### Model Management

```bash
# List available models
rullm models list

# Manage aliases
rullm alias list
rullm alias add my-fast "openai/gpt-3.5-turbo"
rullm alias show claude

# API key management
rullm keys set openai
rullm keys list
```

### Built-in Model Aliases

| Alias | Full Model |
|-------|------------|
| `gpt4` | `openai/gpt-4` |
| `gpt4o` | `openai/gpt-4o` |
| `turbo` | `openai/gpt-3.5-turbo` |
| `claude` | `anthropic/claude-3-5-sonnet-20241022` |
| `sonnet` | `anthropic/claude-3-5-sonnet-20241022` |
| `opus` | `anthropic/claude-3-opus-20240229` |
| `gemini` | `google/gemini-1.5-pro` |
| `gemini-flash` | `google/gemini-1.5-flash` |

## Shell Completion

To enable shell completion, generate the completion script for your shell:

```shell
# fish
source (COMPLETE=fish ./target/debug/rullm | psub)

# bash
source <(COMPLETE=bash ./target/debug/rullm)

# zsh
source <(COMPLETE=zsh ./target/debug/rullm)
```