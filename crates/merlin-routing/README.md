# merlin-routing

Task analysis, complexity assessment, model selection, and routing strategies.

## Purpose

This crate provides intelligent routing between different LLM tiers based on task complexity, cost, and quality requirements. It includes task analysis, decomposition, model selection, response caching, and metrics collection.

## Module Structure

### Analyzer (`analyzer/`)
- `decompose.rs` - `TaskDecomposer` for breaking down complex tasks
- `intent.rs` - `IntentExtractor` for extracting intent from requests
- `local.rs` - `LocalTaskAnalyzer` for local task analysis (no LLM)

### Router (`router/`)
- `mod.rs` - Main router logic
- `model_registry.rs` - Model registration and management
- `models.rs` - Model definitions
- `provider_registry.rs` - Provider registration
- `tiers.rs` - Tier selection logic

### Cache (`cache/`)
- `mod.rs` - Response caching interface
- `storage.rs` - Cache storage implementation

### Metrics (`metrics/`)
- `mod.rs` - Metrics collection interface
- `collector.rs` - `MetricsCollector` implementation
- `reporter.rs` - `MetricsReport` generation

### UI (`user_interface/`)
- `mod.rs` - UI event re-exports

## Public API

- `TaskAnalyzer`, `LocalTaskAnalyzer` - Analyze task complexity
- `TaskDecomposer` - Break down complex tasks
- `IntentExtractor` - Extract intent from requests
- `ModelRouter`, `StrategyRouter` - Route to appropriate models
- `ResponseCache` - Semantic caching
- `MetricsCollector`, `MetricsReport` - Performance metrics
- `ModelRegistry`, `ProviderRegistry` - Model management
  - `ProviderRegistry` owns its configuration (RoutingConfig)
  - Internally uses Arc for providers (HashMap<Model, Arc<dyn ModelProvider>>)

## Features

### Task Analysis
- Complexity estimation
- Intent extraction
- Dependency detection
- Context requirement analysis

### Multi-Tier Routing
- Local tier (Ollama) - Free, fast
- Groq tier - Free with rate limits
- Premium tier (OpenRouter, Anthropic) - Paid, high quality
- **Difficulty-based model selection**: Tasks with difficulty 1-10 automatically route to appropriate tier
- **Automatic escalation**: Failed tasks can be retried at higher tiers by increasing difficulty (managed by merlin-agent orchestrator)
- **Provider overrides**: Configure `provider_low` (1-3), `provider_mid` (4-6), `provider_high` (7-10) in config to override tier-based routing
  - When all difficulty levels are covered by overrides, tier-based providers (Local/Groq/Premium) are not initialized
  - Useful for using a single provider (e.g., ClaudeCode) for all tasks without needing API keys for other providers

### Caching
- Semantic caching for repeated queries
- Configurable TTL
- Cache hit/miss tracking

### Metrics
- Cost tracking
- Latency measurement
- Success rate monitoring
- Token usage analytics

## Testing Status

**✅ Well-tested**

- **Unit tests**: 2 files with tests
  - `analyzer/local.rs` - Local analyzer tests
  - `cache/storage.rs` - Cache storage tests
- **Fixture coverage**: 10+ fixtures
  - `orchestrator/` - Orchestration tests
  - `analysis_decomposition.json` - Task decomposition
  - `task_graph_operations.json` - Task graph tests

## Code Quality

- ✅ **Documentation**: All public items documented
- ✅ **Error handling**: Proper `Result<T, E>` usage
- ✅ **No dead code**: All modules actively used
- ✅ **No TODOs**: Implementation complete

## Dependencies

- `merlin-core` - Core types
- `merlin-providers` - External providers
- `merlin-local` - Local models
- `serde` - Serialization
- `tokio` - Async runtime

## Usage Example

```rust
use merlin_routing::{TaskAnalyzer, ModelRouter};
use merlin_core::Query;

// Analyze task
let analyzer = TaskAnalyzer::new();
let analysis = analyzer.analyze("Add error handling").await?;

// Route to appropriate tier
let router = ModelRouter::new();
let provider = router.select_provider(&analysis)?;

// Execute
let response = provider.generate(&query, &context).await?;
```

## Issues and Recommendations

**None** - This crate is well-maintained with excellent fixture-based test coverage.
