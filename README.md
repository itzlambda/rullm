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

# Use templates for structured queries ({{input}} placeholder is automatically filled)
rullm -t code-review "Review this function"
rullm -t greeting --param name=Alice "Welcome message"

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

## 📝 Templates

Templates provide a way to create reusable prompt structures with placeholders for dynamic content. They're stored as TOML files in your system's configuration directory.

### Template Usage

```bash
# Use a template ({{input}} is replaced by your query)
rullm -t my-template "input text"
```

### Template Format

Templates are stored as TOML files in `~/.config/rullm/templates/` (or your system's config directory):

```toml
name = "code-review"
description = "Template for code review requests"
system_prompt = "You are an expert {{language}} developer. Review the code with {{style}} analysis."
user_prompt = "Please review this {{language}} code: {{input}}"

[defaults]
language = "generic"
style = "standard"
```

### Template Placeholders

- `{{input}}` – Automatically filled with the user's query text.
- Any additional placeholders **must** have a default value specified in the
  `[defaults]` table of the template file, since the CLI no longer supports
  runtime `--param` overrides.

If a placeholder other than `input` lacks a default, rendering will fail.

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