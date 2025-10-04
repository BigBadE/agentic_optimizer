# Agentic Providers

External LLM provider adapters for the Merlin.

## Overview

This crate provides integration with external LLM services including Groq (free tier), OpenRouter, and Anthropic. Each provider implements the `ModelProvider` trait from `merlin-core`.

## Features

- ✅ **Groq Integration** - Free tier API with fast inference
- ✅ **OpenRouter Integration** - Access to multiple models
- ✅ **Anthropic Integration** - Claude models
- ✅ **Unified Interface** - All providers implement `ModelProvider`
- ✅ **Cost Estimation** - Per-provider cost calculation

## Module Structure

```
merlin-providers/
├── src/
│   ├── groq.rs         # GroqProvider - Free tier API
│   ├── openrouter.rs   # OpenRouterProvider - Multi-model access
│   ├── anthropic.rs    # AnthropicProvider - Claude models
│   └── lib.rs          # Public API
└── Cargo.toml
```

## Providers

### GroqProvider

Free tier API with fast inference using Llama models.

**Features:**
- Free tier with rate limits
- Fast inference (~500ms)
- Default model: `llama-3.1-70b-versatile`
- Zero cost (free tier)

**Setup:**
```bash
export GROQ_API_KEY="gsk-..."
```

**Example:**
```rust
use agentic_providers::GroqProvider;
use agentic_core::{ModelProvider, Query, Context};

let provider = GroqProvider::new()?;

// Optional: Use different model
let provider = provider.with_model("llama-3.1-8b-instant".to_string());

let query = Query::new("Explain this code");
let context = Context::new("You are a coding assistant");

let response = provider.generate(&query, &context).await?;
println!("Response: {}", response.text);
```

**API Details:**
- **Endpoint:** `https://api.groq.com/openai/v1/chat/completions`
- **Format:** OpenAI-compatible chat completions
- **Authentication:** Bearer token
- **Rate Limits:** Free tier limits apply

### OpenRouterProvider

Access to multiple models through OpenRouter API.

**Features:**
- Multiple model options
- Pay-per-use pricing
- Model selection flexibility
- Fallback support

**Setup:**
```bash
export OPENROUTER_API_KEY="sk-or-..."
```

**Example:**
```rust
use agentic_providers::OpenRouterProvider;

let provider = OpenRouterProvider::new("sk-or-...".to_string())?
    .with_model("deepseek/deepseek-coder".to_string());

let response = provider.generate(&query, &context).await?;
```

**Popular Models:**
- `deepseek/deepseek-coder` - Specialized coding model
- `anthropic/claude-3.5-sonnet` - High quality
- `anthropic/claude-3-haiku` - Fast and cheap
- `meta-llama/llama-3.1-70b-instruct` - Open source

### AnthropicProvider

Direct integration with Anthropic's Claude models.

**Features:**
- Claude 3.5 Sonnet, Claude 3 Haiku
- High quality responses
- Prompt caching support
- Direct API access

**Setup:**
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

**Example:**
```rust
use agentic_providers::AnthropicProvider;

let provider = AnthropicProvider::new("sk-ant-...".to_string())?;

let response = provider.generate(&query, &context).await?;
```

**Note:** Currently uses a fixed model. Model selection will be added in future updates.

## Testing

### Unit Tests (3 passing)

Run tests:
```bash
cargo test
```

**Test Coverage:**
- `test_groq_provider_with_api_key` - Provider initialization
- `test_groq_availability` - Availability check
- `test_cost_estimation` - Cost calculation

**Note:** Tests use mock API keys and don't make real API calls.

## API Types

### Common Request Format

All providers use similar request structures:

```rust
struct Request {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: usize,
}

struct Message {
    role: String,      // "system", "user", "assistant"
    content: String,
}
```

### Common Response Format

```rust
struct Response {
    text: String,
    confidence: f64,
    tokens_used: TokenUsage,
    provider: String,
    latency_ms: u64,
}

struct TokenUsage {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write: u64,
}
```

## Cost Estimation

Each provider implements `estimate_cost()`:

```rust
let context = Context::new("system prompt");
let cost = provider.estimate_cost(&context);
println!("Estimated cost: ${:.4}", cost);
```

**Current Implementation:**
- **Groq:** $0 (free tier)
- **OpenRouter:** Varies by model
- **Anthropic:** Varies by model

## Error Handling

All providers return `agentic_core::Error`:

```rust
use agentic_core::Error;

match provider.generate(&query, &context).await {
    Ok(response) => println!("Success: {}", response.text),
    Err(Error::MissingApiKey(key)) => {
        eprintln!("Missing API key: {}", key);
    }
    Err(Error::Other(msg)) => {
        eprintln!("API error: {}", msg);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Performance Comparison

| Provider | Model | Cost | Latency | Use Case |
|----------|-------|------|---------|----------|
| Groq | llama-3.1-70b-versatile | $0 | ~500ms | Medium complexity, free tier |
| OpenRouter | deepseek-coder | $0.0000002/token | ~2000ms | Specialized coding |
| OpenRouter | claude-3-haiku | $0.00000025/token | ~1500ms | Fast, cheap |
| Anthropic | claude-3.5-sonnet | $0.000003/token | ~2000ms | High quality |

## Integration with Routing

These providers are used by `merlin-routing` for tier-based routing:

```rust
use agentic_routing::ModelTier;

// Groq tier
let tier = ModelTier::Groq {
    model_name: "llama-3.1-70b-versatile".to_string()
};

// Premium tier (OpenRouter)
let tier = ModelTier::Premium {
    provider: "openrouter".to_string(),
    model_name: "deepseek/deepseek-coder".to_string(),
};

// Premium tier (Anthropic)
let tier = ModelTier::Premium {
    provider: "anthropic".to_string(),
    model_name: "claude-3.5-sonnet".to_string(),
};
```

## Rate Limits

### Groq (Free Tier)
- Requests per minute: Varies
- Tokens per minute: Varies
- Automatic retry recommended

### OpenRouter
- Depends on model and plan
- Check OpenRouter dashboard

### Anthropic
- Depends on plan tier
- Check Anthropic console

## Best Practices

1. **API Key Security**
   - Use environment variables
   - Never hardcode keys
   - Rotate keys regularly

2. **Error Handling**
   - Implement retry logic
   - Handle rate limits gracefully
   - Log errors for debugging

3. **Cost Management**
   - Estimate costs before execution
   - Monitor usage
   - Use free tiers when appropriate

4. **Performance**
   - Cache responses when possible
   - Use streaming for long responses
   - Choose appropriate models

## Future Enhancements

- [ ] Streaming response support
- [ ] Response caching
- [ ] Automatic retry with backoff
- [ ] More provider integrations (Gemini, etc.)
- [ ] Model selection for AnthropicProvider
- [ ] Batch request support

## Dependencies

- `merlin-core` - Core types and traits
- `tokio` - Async runtime
- `reqwest` - HTTP client
- `serde` / `serde_json` - Serialization
- `async-trait` - Async trait support

## License

MIT

