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

# Interactive chat
rullm chat --model claude

# Chat history is saved between sessions.
# Use Alt+Enter for multiline input in chat mode.
# In chat, type /edit to open your $EDITOR and compose a message.
# Press Ctrl+O in chat to open a buffer for editing your prompt directly.

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

### Piping files and merging with queries

You can pipe files or stdin into `rullm` and optionally add a query string. The CLI will merge both, making it easy to work with code or text files:

```bash
# Just pipe a file (stdin only)
cat foo.py | rullm

# Pipe a file and add a query (stdin + arg)
cat foo.py | rullm "explain this code"
```

### System prompt on the fly

You can pass a system prompt directly to the model using the `--system` argument. This lets you customize the LLM's behavior for a single request:

```bash
rullm --system "You are a helpful assistant." "Summarize this text"
```
## 🔧 CLI Commands

### Model Management

```bash
# List available models (shows only chat models, with your aliases)
rullm models list

# Update model list for all providers with API keys
rullm models update

# Manage aliases
rullm alias list
rullm alias add my-fast "openai/gpt-3.5-turbo"
rullm alias show claude

# API key management
rullm keys set openai
rullm keys list
```

## 📝 Templates

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
# You can include multi-line prompts using TOML triple-quoted strings:
system_prompt = """
You are a senior Rust engineer.

Provide a thorough review with the following structure:
1. Summary
2. Strengths
3. Weaknesses
4. Suggestions
"""
user_prompt = "Please review this code: {{input}}"
```

### Template Placeholders

- `{{input}}` – Automatically filled with the user's query text.

### Model Aliases

Model aliases are now user-defined. Use `rullm alias add <alias> <provider/model>` to create your own shortcuts. Use `rullm alias list` to see your aliases.

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