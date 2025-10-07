# Merlin Usage Guide

## Quick Start

The easiest way to use Merlin is the default interactive mode:

```bash
# Build first
cargo build --release

# Start interactive agent (default mode)
./target/release/merlin

# Or with options
./target/release/merlin --local          # Use only Ollama (free, offline)
./target/release/merlin --no-tui         # Plain text output
./target/release/merlin --validation disabled  # Skip validation for faster iterations
```

## Interactive Mode (Default)

When you run `merlin` without any subcommand, you get an interactive agent with:
- **Continuous conversation** - Build on previous responses
- **TUI interface** - Real-time task progress visualization
- **Multi-model routing** - Automatic tier selection based on complexity
- **Context awareness** - Remembers your codebase and conversation

### Example Session

```
$ ./target/release/merlin --local

=== Merlin - Interactive AI Coding Assistant ===
Project: /current_projects/agentic_optimizer
Mode: Local Only (Ollama)

✓ Agent ready!

Type your request (or 'exit' to quit):

You:
> Add error handling to the parse_input function

[TUI shows real-time progress with task decomposition...]

Merlin:
I've analyzed the parse_input function and added comprehensive error handling...
[Code changes displayed]

You:
> Now add tests for those error cases

[Agent builds on previous context...]

Merlin:
Here are the tests for the error handling we just added...
[Test code displayed]

You:
> exit
Goodbye!
```

## Chat Command (Alternative)

Start an interactive chat session with the AI agent:

```bash
# Using Anthropic (default)
cargo run -p merlin-cli -- chat

# Using OpenRouter with prompt caching
cargo run -p merlin-cli -- chat --openrouter

# Using OpenRouter with a specific model
cargo run -p merlin-cli -- chat --openrouter --model "anthropic/claude-sonnet-4-20250514"

# Specify a different project directory
cargo run -p merlin-cli -- chat --project /path/to/project
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
=== Merlin - Interactive Chat ===
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
cargo run -p merlin-cli -- query "Explain the agent architecture" --project .
```

## Context Preview

Preview what context would be gathered without sending to LLM:

```bash
cargo run -p merlin-cli -- prompt "How does caching work?" --project .
```

## Configuration

View current configuration:

```bash
cargo run -p merlin-cli -- config
cargo run -p merlin-cli -- config --full
```

