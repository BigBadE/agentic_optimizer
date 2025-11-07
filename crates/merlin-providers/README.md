# merlin-providers

External LLM provider adapters (Groq, OpenRouter, Mock).

## Purpose

This crate provides integration with external LLM services. Each provider implements the `ModelProvider` trait from `merlin-core`, enabling unified access to different LLM backends.

## Module Structure

- `groq.rs` - Groq provider (Llama models)
- `openrouter.rs` - OpenRouter provider (multi-model access)

## Public API

- `GroqProvider` - Groq API integration
- `OpenRouterProvider` - OpenRouter API integration

**Note**: `MockProvider` has been moved to `integration-tests` crate for better test isolation and performance.

## Providers

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

- **Unit tests**: All 3 provider files have tests
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
use merlin_providers::GroqProvider;
use merlin_core::{ModelProvider, Query, Context};

let provider = GroqProvider::default()?;
let query = Query::new("Explain this code");
let context = Context::new("You are a coding assistant");

let response = provider.generate(&query, &context).await?;
println!("Response: {}", response.text);
```

## Issues and Recommendations

**None** - This crate is well-maintained with excellent test coverage through fixtures.
