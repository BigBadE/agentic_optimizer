# Agentic Local

Local model integration for the Merlin using Ollama.

## Overview

This crate provides integration with Ollama for running local language models. It enables zero-cost model execution by leveraging locally-hosted models like Qwen Coder, DeepSeek Coder, and CodeLlama.

## Features

- ✅ **Ollama Integration** - Seamless connection to Ollama API
- ✅ **Model Management** - List, check, and pull models
- ✅ **Zero Cost** - No API fees for local execution
- ✅ **Fast Inference** - ~100ms latency for simple tasks
- ✅ **Multiple Models** - Support for various coding models

## Module Structure

```
merlin-local/
├── src/
│   ├── manager.rs      # OllamaManager - Model management
│   ├── models.rs       # Model metadata and API types
│   ├── inference.rs    # LocalModelProvider - Inference execution
│   ├── error.rs        # Error types
│   └── lib.rs          # Public API
└── Cargo.toml
```

## Components

### OllamaManager

Manages Ollama installation and models.

**Key Methods:**
- `is_available()` - Check if Ollama is running
- `list_models()` - List installed models
- `has_model(name)` - Check if specific model is installed
- `pull_model(name)` - Download model from registry
- `ensure_model(name)` - Auto-pull if missing
- `recommended_models()` - Get list of recommended coding models

**Example:**
```rust
use agentic_local::OllamaManager;

let manager = OllamaManager::new();

// Check availability
if manager.is_available().await {
    // Ensure model is installed
    manager.ensure_model("qwen2.5-coder:7b").await?;
    
    // List all models
    let models = manager.list_models().await?;
}
```

### LocalModelProvider

Implements the `ModelProvider` trait for local inference.

**Features:**
- Integrates with Ollama API
- Handles context files in prompts
- Zero cost execution
- Token usage tracking

**Example:**
```rust
use agentic_local::LocalModelProvider;
use agentic_core::{ModelProvider, Query, Context};

let provider = LocalModelProvider::new("qwen2.5-coder:7b".to_string());

let query = Query::new("Explain this code");
let context = Context::new("You are a coding assistant");

let response = provider.generate(&query, &context).await?;
println!("Response: {}", response.text);
```

### Model Metadata

Pre-configured model information for recommended coding models.

**Recommended Models:**
- **Qwen 2.5 Coder 7B** - 4.4GB, Q4_0 quantization, excellent for code
- **DeepSeek Coder 6.7B** - 3.8GB, Q4_0 quantization, specialized for coding
- **CodeLlama 7B** - 3.8GB, Q4_0 quantization, Meta's coding model

**Example:**
```rust
use agentic_local::ModelInfo;

let qwen = ModelInfo::qwen_coder_7b();
println!("Model: {}", qwen.name);
println!("Size: {} bytes", qwen.size_bytes);
println!("Parameters: {}", qwen.parameter_count);
```

## API Types

### OllamaGenerateRequest
Request structure for Ollama generation API.

**Fields:**
- `model` - Model name
- `prompt` - Input prompt
- `system` - Optional system prompt
- `temperature` - Optional temperature (0.0-1.0)
- `max_tokens` - Optional max tokens
- `stream` - Enable streaming (default: false)

### OllamaGenerateResponse
Response structure from Ollama generation API.

**Fields:**
- `model` - Model used
- `response` - Generated text
- `done` - Completion status
- `total_duration` - Total time in nanoseconds
- `prompt_eval_count` - Input tokens
- `eval_count` - Output tokens

## Testing

### Unit Tests (5 passing)

Run tests:
```bash
cargo test
```

**Test Coverage:**
- `test_ollama_manager_creation` - Manager initialization
- `test_custom_url` - Custom Ollama URL
- `test_recommended_models` - Model metadata
- `test_local_provider_creation` - Provider initialization
- `test_cost_estimation` - Cost calculation (always $0)

## Setup

### Prerequisites

1. **Install Ollama:**
   ```bash
   # macOS/Linux
   curl https://ollama.ai/install.sh | sh
   
   # Windows
   # Download from https://ollama.ai/download
   ```

2. **Start Ollama:**
   ```bash
   ollama serve
   ```

3. **Pull a model:**
   ```bash
   ollama pull qwen2.5-coder:7b
   ```

### Configuration

**Default Ollama URL:** `http://localhost:11434`

**Custom URL:**
```rust
let manager = OllamaManager::new()
    .with_url("http://custom:8080".to_string());
```

## Performance

| Model | Size | Latency | Use Case |
|-------|------|---------|----------|
| Qwen 2.5 Coder 7B | 4.4GB | ~100ms | General coding tasks |
| DeepSeek Coder 6.7B | 3.8GB | ~100ms | Code generation |
| CodeLlama 7B | 3.8GB | ~100ms | Code completion |

**Note:** Latency depends on hardware. GPU acceleration recommended for best performance.

## Error Handling

### Error Types

- `OllamaUnavailable` - Ollama not running or not accessible
- `ModelNotFound` - Requested model not installed
- `ModelPullFailed` - Failed to download model
- `InferenceFailed` - Generation failed
- `Http` - Network/HTTP errors
- `Json` - JSON parsing errors

**Example:**
```rust
use agentic_local::{OllamaManager, LocalError};

match manager.list_models().await {
    Ok(models) => println!("Found {} models", models.len()),
    Err(LocalError::OllamaUnavailable(msg)) => {
        eprintln!("Ollama not available: {}", msg);
        eprintln!("Please start Ollama with: ollama serve");
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Integration

This crate is used by `merlin-routing` for the Local tier in multi-model routing:

```rust
use agentic_routing::{ModelTier, RoutingOrchestrator};

// Local tier automatically uses LocalModelProvider
let tier = ModelTier::Local {
    model_name: "qwen2.5-coder:7b".to_string()
};
```

## Dependencies

- `merlin-core` - Core types and traits
- `tokio` - Async runtime
- `reqwest` - HTTP client for Ollama API
- `serde` / `serde_json` - Serialization
- `async-trait` - Async trait support
- `thiserror` / `anyhow` - Error handling

## License

MIT

