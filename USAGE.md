# Agentic Optimizer Usage Guide

## Interactive Chat

Start an interactive chat session with the AI agent:

```bash
# Using Anthropic (default)
cargo run -p agentic-cli -- chat

# Using OpenRouter with prompt caching
cargo run -p agentic-cli -- chat --openrouter

# Using OpenRouter with a specific model
cargo run -p agentic-cli -- chat --openrouter --model "anthropic/claude-sonnet-4-20250514"

# Specify a different project directory
cargo run -p agentic-cli -- chat --project /path/to/project
```

### Environment Variables

**For Anthropic:**
```bash
export ANTHROPIC_API_KEY="your-api-key"
```

**For OpenRouter:**
```bash
export OPENROUTER_API_KEY="your-api-key"
```

**For Ollama (required for context fetching):**
Ensure Ollama is running locally with the `qwen2.5-coder:1.5b` model installed.

### Features

- **Intelligent Context Fetching**: Automatically finds relevant code files using hybrid BM25 + vector search
- **Prompt Caching**: OpenRouter provider supports prompt caching to reduce costs on repeated context
- **Rust Semantic Analysis**: Uses rust-analyzer for deep code understanding
- **Interactive Loop**: Continuous conversation with context awareness
- **Cost Tracking**: Shows token usage and cache hit rates

### Chat Commands

- Type your question or request
- Type `exit` or `quit` to end the session
- Empty input is ignored

### Example Session

```
=== Agentic Optimizer - Interactive Chat ===
Project: .
Provider: OpenRouter

Initializing agent...
✓ Agent ready!

Type your message (or 'exit' to quit):

You:
> What does the ContextBuilder do?

Agent:
The ContextBuilder is responsible for gathering relevant code files and building
context for the AI agent. It uses hybrid search (BM25 + vector embeddings) to
find the most relevant files based on your query...

---
Provider: openrouter | Latency: 2341ms | Tokens: 15234 tokens
Cache: 12000 tokens (78.8% cache hit)
---

You:
> exit
Goodbye!
```

## Single Query Mode

For one-off queries without interactive mode:

```bash
cargo run -p agentic-cli -- query "Explain the agent architecture" --project .
```

## Context Preview

Preview what context would be gathered without sending to LLM:

```bash
cargo run -p agentic-cli -- prompt "How does caching work?" --project .
```

## Configuration

View current configuration:

```bash
cargo run -p agentic-cli -- config
cargo run -p agentic-cli -- config --full
```
