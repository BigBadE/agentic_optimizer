# merlin-providers

External LLM provider adapters (Claude Code, Groq, OpenRouter, Mock).

## Purpose

This crate provides integration with external LLM services. Each provider implements the `ModelProvider` trait from `merlin-core`, enabling unified access to different LLM backends.

## Module Structure

- `claude_code.rs` - Claude Code provider (Anthropic API)
- `groq.rs` - Groq provider (Llama models)
- `openrouter.rs` - OpenRouter provider (multi-model access)

## Public API

- `ClaudeCodeProvider` - Claude Code API integration
- `GroqProvider` - Groq API integration
- `OpenRouterProvider` - OpenRouter API integration

**Note**: `MockProvider` has been moved to `integration-tests` crate for better test isolation and performance.

## Providers

### ClaudeCodeProvider
Claude Code CLI integration using your Claude subscription.

**Features:**
- Uses Claude Code subscription (no API billing)
- Invokes `claude` CLI as subprocess
- High-quality responses
- Default model: `claude-sonnet-4-5-20250929`

**Setup:**
```bash
# Install Claude Code CLI and authenticate
# https://docs.claude.com/claude-code
claude setup-token

# No API key needed - uses your Claude subscription
```

### GroqProvider
Groq API integration with fast inference using Llama models.

**Features:**
- Free tier available
- Fast inference
- Default model: `llama-3.1-70b-versatile`

**Setup:**
```bash
export GROQ_API_KEY="gsk-..."
```

### OpenRouterProvider
Access to multiple models through OpenRouter API.

**Features:**
- Multiple model options
- Pay-per-use pricing
- Model selection flexibility

**Setup:**
```bash
export OPENROUTER_API_KEY="sk-or-..."
```

### MockProvider
Testing provider with configurable responses.

**Features:**
- Pattern-based response matching
- Configurable latency simulation
- Token usage simulation
- Error injection for testing

## Testing Status

**✅ Well-tested**

- **Unit tests**: All 4 provider files have tests
- **MockProvider**: Heavily used in fixture-based tests
- **Integration tests**: Extensive fixture coverage in integration-tests crate

## Code Quality

- ✅ **Documentation**: All public items documented
- ✅ **Error handling**: Proper `Result<T, E>` usage
- ✅ **No dead code**: All modules actively used
- ✅ **No TODOs**: Implementation complete

## Dependencies

- `merlin-core` - Core types and traits
- `reqwest` - HTTP client
- `serde` / `serde_json` - Serialization
- `tokio` - Async runtime

## Usage Example

```rust
use merlin_providers::ClaudeCodeProvider;
use merlin_core::{ModelProvider, Query, Context};

let provider = ClaudeCodeProvider::new()?;
let query = Query::new("Explain this code");
let context = Context::new("You are a coding assistant");

let response = provider.generate(&query, &context).await?;
println!("Response: {}", response.text);
```

## Issues and Recommendations

**None** - This crate is well-maintained with excellent test coverage through fixtures.
