# Agentic Core

Core types and traits for the Merlin.

## Overview

This crate provides the foundational types and traits used across all Merlin crates. It defines the `ModelProvider` trait that all LLM providers must implement, along with common data structures for queries, responses, and contexts.

## Features

- ✅ **ModelProvider Trait** - Unified interface for all LLM providers
- ✅ **Type Safety** - Strong typing for queries and responses
- ✅ **Token Tracking** - Comprehensive token usage tracking
- ✅ **Context Management** - File-based context handling
- ✅ **Serialization** - Full serde support

## Module Structure

```
merlin-core/
├── src/
│   ├── types.rs        # Core data structures
│   ├── provider.rs     # ModelProvider trait
│   ├── error.rs        # Error types
│   └── lib.rs          # Public API
└── Cargo.toml
```

## Core Types

### Query

Represents a user query or request to an LLM.

**Fields:**
- `text` - The query text
- `conversation_id` - Optional conversation ID for multi-turn
- `files_context` - List of file paths for context

**Example:**
```rust
use agentic_core::Query;
use std::path::PathBuf;

let query = Query::new("Explain this code")
    .with_files(vec![
        PathBuf::from("src/main.rs"),
        PathBuf::from("src/lib.rs"),
    ]);
```

### Response

Represents an LLM response.

**Fields:**
- `text` - Generated response text
- `confidence` - Confidence score (0.0-1.0)
- `tokens_used` - Token usage breakdown
- `provider` - Provider name/model used
- `latency_ms` - Response time in milliseconds

**Example:**
```rust
use agentic_core::Response;

println!("Response: {}", response.text);
println!("Confidence: {:.2}", response.confidence);
println!("Tokens: {}", response.tokens_used.total());
println!("Latency: {}ms", response.latency_ms);
```

### TokenUsage

Tracks token consumption with cache support.

**Fields:**
- `input` - Input tokens
- `output` - Output tokens
- `cache_read` - Tokens read from cache
- `cache_write` - Tokens written to cache

**Methods:**
- `total()` - Sum of all token types

**Example:**
```rust
let usage = response.tokens_used;
println!("Input: {}", usage.input);
println!("Output: {}", usage.output);
println!("Cache read: {}", usage.cache_read);
println!("Total: {}", usage.total());

// Calculate cache hit rate
if usage.total() > 0 {
    let cache_rate = (usage.cache_read as f64 / usage.total() as f64) * 100.0;
    println!("Cache hit rate: {:.1}%", cache_rate);
}
```

### Context

Represents the context provided to an LLM.

**Fields:**
- `files` - List of file contexts
- `system_prompt` - System-level instructions

**Methods:**
- `new(system_prompt)` - Create with system prompt
- `with_files(files)` - Add file contexts
- `files_to_string()` - Format files as string
- `token_estimate()` - Estimate token count

**Example:**
```rust
use agentic_core::{Context, FileContext};
use std::path::PathBuf;

let context = Context::new("You are a coding assistant")
    .with_files(vec![
        FileContext::new(
            PathBuf::from("main.rs"),
            "fn main() { }".to_string()
        )
    ]);

println!("Estimated tokens: {}", context.token_estimate());
```

### FileContext

Represents a single file in the context.

**Fields:**
- `path` - File path
- `content` - File content

**Methods:**
- `new(path, content)` - Create from path and content
- `from_path(path)` - Load from filesystem

**Example:**
```rust
use agentic_core::FileContext;
use std::path::PathBuf;

// From path (reads file)
let file_ctx = FileContext::from_path(&PathBuf::from("main.rs"))?;

// From content
let file_ctx = FileContext::new(
    PathBuf::from("main.rs"),
    "fn main() { }".to_string()
);
```

## ModelProvider Trait

The core trait that all LLM providers must implement.

**Required Methods:**
- `name()` - Provider name
- `is_available()` - Check if provider is accessible
- `generate(query, context)` - Generate response
- `estimate_cost(context)` - Estimate cost for context

**Example Implementation:**
```rust
use agentic_core::{ModelProvider, Query, Context, Response, Result};
use async_trait::async_trait;

struct MyProvider;

#[async_trait]
impl ModelProvider for MyProvider {
    fn name(&self) -> &'static str {
        "MyProvider"
    }
    
    async fn is_available(&self) -> bool {
        // Check if API key is set, service is reachable, etc.
        true
    }
    
    async fn generate(&self, query: &Query, context: &Context) -> Result<Response> {
        // Call LLM API and return response
        Ok(Response {
            text: "Generated response".to_string(),
            confidence: 0.9,
            tokens_used: TokenUsage::default(),
            provider: self.name().to_string(),
            latency_ms: 100,
        })
    }
    
    fn estimate_cost(&self, context: &Context) -> f64 {
        // Calculate estimated cost based on context size
        let tokens = context.token_estimate();
        tokens as f64 * 0.000001  // $0.000001 per token
    }
}
```

**Usage:**
```rust
let provider = MyProvider;

if provider.is_available().await {
    let query = Query::new("Hello");
    let context = Context::new("You are helpful");
    
    let response = provider.generate(&query, &context).await?;
    println!("Response: {}", response.text);
}
```

## Error Types

### Error

Main error enum for the crate.

**Variants:**
- `FileNotFound(String)` - File not found
- `MissingApiKey(String)` - API key not set
- `Other(String)` - Generic error

**Example:**
```rust
use agentic_core::{Error, Result};

fn load_file(path: &str) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|_| Error::FileNotFound(path.to_string()))
}

match load_file("config.toml") {
    Ok(content) => println!("Loaded: {}", content),
    Err(Error::FileNotFound(path)) => {
        eprintln!("File not found: {}", path);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Testing

### Unit Tests

Run tests:
```bash
cargo test
```

**Test Coverage:**
- Query creation and builder methods
- Context creation and file handling
- TokenUsage calculations
- FileContext loading
- Error handling

## Token Estimation

The `Context::token_estimate()` method provides a rough estimate:

```rust
// Estimation formula: (system_prompt.len() + files_content.len()) / 4
let tokens = context.token_estimate();
```

**Note:** This is a rough approximation. Actual token counts may vary by model and tokenizer.

## Serialization

All types support serde serialization:

```rust
use agentic_core::Query;

let query = Query::new("Hello");

// Serialize to JSON
let json = serde_json::to_string(&query)?;

// Deserialize from JSON
let query: Query = serde_json::from_str(&json)?;
```

## Best Practices

1. **Context Size**
   - Keep contexts reasonably sized
   - Use `token_estimate()` to check size
   - Consider model context limits

2. **Error Handling**
   - Always handle `Result` types
   - Provide meaningful error messages
   - Log errors for debugging

3. **Token Tracking**
   - Track all token usage
   - Monitor cache hit rates
   - Use for cost calculation

4. **Provider Implementation**
   - Implement all trait methods
   - Handle errors gracefully
   - Return accurate token counts

## Integration

This crate is used by:
- `merlin-providers` - External LLM providers
- `merlin-local` - Local model integration
- `merlin-routing` - Multi-model routing
- `merlin-agent` - Agent implementation

**Example:**
```rust
use agentic_core::{ModelProvider, Query, Context};
use agentic_providers::GroqProvider;

let provider = GroqProvider::new()?;
let query = Query::new("Explain this code");
let context = Context::new("You are a coding assistant");

let response = provider.generate(&query, &context).await?;
```

## Dependencies

- `serde` / `serde_json` - Serialization
- `thiserror` - Error handling

## License

MIT

