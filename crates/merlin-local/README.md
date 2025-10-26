# merlin-local

Local model integration via Ollama.

## Purpose

This crate provides integration with Ollama for running local language models, enabling zero-cost model execution with locally-hosted models like Qwen Coder, DeepSeek Coder, and CodeLlama.

## Module Structure

- `manager.rs` - `OllamaManager` for model management
- `models.rs` - Model metadata and API types
- `inference.rs` - `LocalModelProvider` implementation
- `error.rs` - `LocalError` type

## Public API

- `LocalModelProvider` - Local inference via Ollama
- `OllamaManager` - Ollama service management
- `LocalError`, `Result` - Error handling
- Data types: `OllamaModel`, `ModelInfo`, `OllamaGenerateRequest`, `OllamaGenerateResponse`

## Features

### OllamaManager
Manages Ollama installation and models:
- Check if Ollama is running
- List installed models
- Pull models from registry
- Auto-install missing models

### LocalModelProvider
Implements `ModelProvider` trait for local inference:
- Zero-cost execution
- Token usage tracking
- Integration with Merlin routing system

## Testing Status

**✅ Good coverage**

- **Unit tests**: 2 tests in manager.rs
- **Integration tests**: `tests/ollama_integration_tests.rs` with 17 tests
  - Manager creation and configuration
  - Model metadata and recommended models
  - Provider creation and cost estimation
  - Availability checks and error handling
  - Concurrent operation testing
  - Query/context setup validation

## Code Quality

- ✅ **Documentation**: All public items documented
- ✅ **Error handling**: Proper `Result<T, E>` usage
- ✅ **No dead code**: All modules used
- ✅ **No TODOs**: Implementation complete

## Dependencies

- `merlin-core` - Core types and traits
- `reqwest` - HTTP client for Ollama API
- `serde` / `serde_json` - Serialization
- `tokio` - Async runtime
- `thiserror` / `anyhow` - Error handling

## Usage Example

```rust
use merlin_local::{OllamaManager, LocalModelProvider};
use merlin_core::{ModelProvider, Query, Context};

// Check Ollama availability
let manager = OllamaManager::default();
if manager.is_available().await {
    manager.ensure_model("qwen2.5-coder:7b").await?;
}

// Use local model
let provider = LocalModelProvider::new("qwen2.5-coder:7b".to_string());
let query = Query::new("Explain this code");
let context = Context::new("You are a coding assistant");

let response = provider.generate(&query, &context).await?;
```

## Issues and Recommendations

### Future Enhancements
1. Add mock Ollama API server for more comprehensive testing
2. Add fixture coverage for local model execution scenarios
3. Add performance benchmarks for inference latency
4. Test integration with routing system via fixtures
