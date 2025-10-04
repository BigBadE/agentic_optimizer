# Agentic Routing

Multi-model routing architecture with intelligent task decomposition, parallel execution, and comprehensive validation.

## Features

### 🎯 Intelligent Routing
- **Multi-tier model selection**: Local (Ollama) → Groq (free) → Premium (paid)
- **Strategy-based routing**: Quality, cost, complexity, and context-aware
- **Automatic escalation**: Fallback to higher tiers on failure
- **Cost optimization**: Prefer free tiers when appropriate

### 🔄 Task Management
- **Smart decomposition**: Break complex requests into subtasks
- **Dependency tracking**: Automatic dependency graph construction
- **Parallel execution**: Run independent tasks concurrently
- **Conflict detection**: Prevent concurrent file modifications

### ✅ Validation Pipeline
- **Multi-stage validation**: Syntax → Build → Test → Lint
- **Early exit**: Stop on first failure for fast feedback
- **Isolated environments**: Test changes without affecting workspace
- **Comprehensive reporting**: Detailed validation results

### 🔒 Workspace Safety
- **File-level locking**: RAII guards prevent conflicts
- **Transactional workspaces**: Snapshot-based isolation
- **Conflict detection**: Check for concurrent modifications
- **Atomic commits**: All-or-nothing change application

### 🎨 User Interface
- **TUI with ratatui**: Real-time progress display
- **Type-enforced feedback**: All tasks must report progress
- **Scrollable output**: Review complete execution history
- **Interactive input**: Modal editing for user commands

## Quick Start

```rust
use agentic_routing::{RoutingConfig, RoutingOrchestrator};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create configuration
    let config = RoutingConfig::default();
    
    // Create orchestrator
    let orchestrator = RoutingOrchestrator::new(config);
    
    // Process request
    let results = orchestrator.process_request("Add error handling").await?;
    
    for result in results {
        println!("Task completed: {}", result.success);
    }
    
    Ok(())
}
```

## Architecture

### Core Components

1. **Analyzer** (`analyzer/`)
   - Intent extraction from natural language
   - Complexity estimation
   - Task decomposition
   - Context requirement analysis

2. **Router** (`router/`)
   - Strategy-based tier selection
   - Cost and latency estimation
   - Availability checking
   - Model escalation

3. **Executor** (`executor/`)
   - Task graph construction
   - Parallel execution
   - Workspace state management
   - Conflict-aware scheduling

4. **Validator** (`validator/`)
   - Multi-stage validation pipeline
   - Syntax, build, test, and lint checks
   - Isolated build environments
   - Comprehensive scoring

5. **Orchestrator** (`orchestrator.rs`)
   - High-level coordinator
   - End-to-end workflow management
   - Configuration management

## Configuration

```rust
use agentic_routing::*;

let config = RoutingConfig {
    tiers: TierConfig {
        local_enabled: true,
        local_model: "qwen2.5-coder:7b".to_string(),
        groq_enabled: true,
        groq_model: "llama-3.1-70b-versatile".to_string(),
        premium_enabled: true,
        max_retries: 3,
        timeout_seconds: 300,
    },
    validation: ValidationConfig {
        enabled: true,
        early_exit: true,
        syntax_check: true,
        build_check: true,
        test_check: true,
        lint_check: true,
        build_timeout_seconds: 60,
        test_timeout_seconds: 300,
    },
    execution: ExecutionConfig {
        max_concurrent_tasks: 4,
        enable_parallel: true,
        enable_conflict_detection: true,
        enable_file_locking: true,
    },
    workspace: WorkspaceConfig {
        root_path: PathBuf::from("."),
        enable_snapshots: true,
        enable_transactions: true,
    },
};
```

## Model Tiers

### Local Tier (Free)
- **Provider**: Ollama
- **Models**: Qwen 2.5 Coder 7B, DeepSeek Coder 6.7B, CodeLlama 7B
- **Cost**: $0
- **Latency**: ~100ms
- **Use case**: Simple tasks, quick iterations

### Groq Tier (Free)
- **Provider**: Groq
- **Models**: Llama 3.1 70B Versatile
- **Cost**: $0 (with rate limits)
- **Latency**: ~500ms
- **Use case**: Medium complexity, faster than local

### Premium Tier (Paid)
- **Providers**: OpenRouter, Anthropic
- **Models**: Claude 3.5 Sonnet, Claude 3 Haiku, DeepSeek Coder
- **Cost**: $0.0000002 - $0.000015 per token
- **Latency**: ~2000ms
- **Use case**: Complex tasks, critical quality

## Routing Strategies

### 1. Quality Critical Strategy (Priority: 100)
- Applies to: Critical and High priority tasks
- Routes to: Premium models (Claude 3.5 Sonnet)

### 2. Long Context Strategy (Priority: 90)
- Applies to: Tasks with >16k tokens or full context required
- Routes to: Models with large context windows

### 3. Cost Optimization Strategy (Priority: 70)
- Applies to: Non-critical tasks
- Routes to: Free tiers (Local or Groq)

### 4. Complexity Based Strategy (Priority: 50)
- Applies to: All tasks (fallback)
- Routes based on: Task complexity level

## Task Decomposition

### Simple Tasks
```
Input: "Add a comment to main.rs"
Output: 1 task (Sequential)
```

### Refactoring
```
Input: "Refactor the parser module"
Output: 3 tasks (Pipeline)
  1. Analyze current structure
  2. Refactor (depends on #1)
  3. Test refactored code (depends on #2)
```

### Complex Creation
```
Input: "Create a new authentication module"
Output: 3 tasks (Pipeline)
  1. Design structure
  2. Implement (depends on #1)
  3. Add tests (depends on #2)
```

## Validation Pipeline

### Stage 1: Syntax (0ms)
- Heuristic-based checks
- Balanced braces, parentheses, brackets
- Explicit error detection

### Stage 2: Build (5-30s)
- Isolated `cargo check`
- Temporary workspace
- Full error reporting

### Stage 3: Test (10-300s)
- Isolated `cargo test`
- Configurable timeout
- Pass rate calculation

### Stage 4: Lint (5-30s)
- Clippy with `-D warnings`
- Configurable max warnings
- Graduated scoring

## Examples

Run the basic example:
```bash
cargo run --example basic_routing
```

## Testing

Run unit tests:
```bash
cargo test --lib
```

Run integration tests (TODO):
```bash
cargo test --test integration_tests
```

## Environment Setup

### Required
- Rust 1.70+
- Ollama installed and running (for local models)

### Optional
- `GROQ_API_KEY` - For Groq tier
- `OPENROUTER_API_KEY` - For OpenRouter premium tier
- `ANTHROPIC_API_KEY` - For Claude premium tier

## Next Steps

### Immediate
1. Implement provider integration in orchestrator
2. Add real model execution (currently mocked)
3. Implement TUI integration
4. Add configuration file support (TOML/JSON)

### Integration Tests (TODO)
See `tests/integration_tests.rs` for recommended test scenarios:
- Complete routing flow
- Multi-task execution
- Validation pipeline
- Workspace isolation
- Provider integration
- Error handling
- UI integration

### Future Enhancements
1. **Caching**: Cache model responses for identical queries
2. **Metrics**: Track costs, latency, success rates
3. **Learning**: Adjust routing based on historical performance
4. **Streaming**: Support streaming responses
5. **Conversation**: Multi-turn conversation support
6. **Plugins**: Extensible validation and routing strategies

## License

MIT
