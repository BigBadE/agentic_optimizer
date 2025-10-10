# Merlin 
[![DeepSource](https://app.deepsource.com/gh/BigBadE/agentic_optimizer.svg/?label=active+issues&show_trend=true&token=wViZ5mQV5zbj5EQV4856JR3g)](https://app.deepsource.com/gh/BigBadE/agentic_optimizer/)
[![CodeScene Average Code Health](https://codescene.io/projects/72070/status-badges/average-code-health)](https://codescene.io/projects/72070)
[![codecov](https://codecov.io/gh/BigBadE/agentic_optimizer/graph/badge.svg?token=rhOCXhgOUe)](https://codecov.io/gh/BigBadE/agentic_optimizer)

An intelligent AI coding assistant with multi-model routing, automatic task decomposition, and comprehensive validation. Named after the Merlin falcon, known for its speed, precision, and adaptability.

## Current Status: Production Ready

**Core Features:**
- **Interactive Agent** - Continuous conversation with context retention
- **Self-Determining Tasks** - Adaptive task decomposition at runtime
- **Multi-Model Routing** - Intelligent tier selection (Local/Groq/Premium)
- **Task Decomposition** - Automatic splitting of complex requests
- **Parallel Execution** - Concurrent task execution with dependency tracking
- **Validation Pipeline** - Syntax, build, test, and lint checking (enabled by default)
- **TUI Mode** - Real-time visual feedback with task progress
- **Cost Optimization** - Prefer free tiers when appropriate
- **Automatic Escalation** - Retry with higher tier on failure

**Model Tiers:**
- **Local** - Ollama (Qwen 2.5 Coder, DeepSeek Coder) - $0, ~100ms
- **Groq** - Llama 3.1 70B - $0 (free tier), ~500ms
- **Premium** - Claude 3.5 Sonnet, DeepSeek Coder - Paid, ~2000ms
{{ ... }}

**Test Coverage:** 93 tests passing (74 unit, 19 integration) with 26% code coverage

## Quick Start

### Prerequisites

**Required:**
- Rust 1.75+ (edition 2024)
- Ollama installed and running (for local tier)

**Optional (for other tiers):**
- Groq API key (free tier)
- OpenRouter API key (premium tier)
- Anthropic API key (premium tier)

### Installation

1. **Install Ollama** (for local models):
```bash
# macOS/Linux
curl https://ollama.ai/install.sh | sh

# Windows - Download from https://ollama.ai/download

# Start Ollama
ollama serve

# Pull a coding model
ollama pull qwen2.5-coder:7b
```

2. **Clone and build**:
```bash
git clone <repo-url>
cd merlin
cargo build --release
```

3. **Set up API keys** (optional):
```bash
# For Groq tier (free)
export GROQ_API_KEY="gsk-..."

# For premium tiers
export OPENROUTER_API_KEY="sk-or-..."
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Usage

**Interactive Agent (Default):**
```bash
# Start interactive session with multi-model routing
merlin

# With options
merlin --local --verbose  # Local only, show details
merlin --no-validate      # Skip validation for faster iterations
merlin -p /path/to/project  # Specify project directory

# Available flags:
#   --local             Use only local models (Ollama), disable remote tiers
#   --no-validate       Disable validation pipeline (enabled by default)
#   --verbose           Show detailed routing decisions and metrics
#   --no-tui            Disable TUI mode, use plain terminal output
#   -p, --project PATH  Project root directory (default: current directory)
```

**Interactive Session Example:**
```
$ merlin --local

=== Merlin - Interactive AI Coding Assistant ===
Project: .
Mode: Local Only

✓ Agent ready!

Type your request (or 'exit' to quit):

You:
> Add error handling to the parser

Merlin:
[Response with code changes...]

You:
> Now add tests for that error handling

Merlin:
[Response with test code...]

You:
> exit
Goodbye!
```

**Other commands:**
```bash
# Interactive chat session
merlin chat

# Direct query (legacy mode)
merlin query "Find the main function"

# Show configuration
merlin config

# Show metrics
merlin metrics --daily
```

## Configuration

### Environment Variables

**Required for Multi-Model Routing:**
- `GROQ_API_KEY` - Groq API key (free tier)
- `OPENROUTER_API_KEY` - OpenRouter API key (premium tier)
- `ANTHROPIC_API_KEY` - Anthropic API key (premium tier)

**Note:** Ollama must be installed and running for local tier.

### Routing Configuration

Default settings (can be customized in code):
```rust
RoutingConfig {
    tiers: TierConfig {
        local_enabled: true,
        local_model: "qwen2.5-coder:7b",
        groq_enabled: true,
        groq_model: "llama-3.1-70b-versatile",
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
    },
    execution: ExecutionConfig {
        max_concurrent_tasks: 4,
        enable_conflict_detection: true,
    },
}

## Performance

### Model Tier Comparison

| Tier | Provider | Model | Cost | Latency | Use Case |
|------|----------|-------|------|---------|----------|
| Local | Ollama | Qwen 2.5 Coder 7B | $0 | ~100ms | Simple tasks, quick iterations |
| Local | Ollama | DeepSeek Coder 6.7B | $0 | ~100ms | Code generation |
| Groq | Groq | Llama 3.1 70B | $0* | ~500ms | Medium complexity |
| Premium | OpenRouter | DeepSeek Coder | $0.0000002/token | ~2000ms | Complex tasks |
| Premium | Anthropic | Claude 3.5 Sonnet | $0.000003/token | ~2000ms | Critical quality |

*Free tier with rate limits

### Task Decomposition Examples

| Request | Tasks Generated | Strategy | Execution Time |
|---------|----------------|----------|----------------|
| "Add a comment" | 1 task | Sequential | ~100ms |
| "Refactor parser" | 3 tasks (Analyze → Refactor → Test) | Pipeline | ~2-3s |
| "Create auth module" | 3 tasks (Design → Implement → Test) | Pipeline | ~3-5s |
| "Fix multiple files" | N tasks | Parallel | ~500ms-2s |

## Implementation Status

### ✅ Completed (Production Ready)
- [x] **Multi-Model Routing** - All 3 tiers operational
- [x] **Task Decomposition** - Smart splitting with 4 strategies
- [x] **Parallel Execution** - Dependency-aware scheduling
- [x] **Validation Pipeline** - 4-stage validation
- [x] **TUI Mode** - Real-time progress display
- [x] **Provider Integration** - Local, Groq, OpenRouter, Anthropic
- [x] **Retry & Escalation** - Automatic tier upgrade on failure
- [x] **Comprehensive Testing** - 72 tests passing

### 🔄 Future Enhancements
- [ ] **Config Files** - TOML/JSON configuration support
- [ ] **Response Caching** - Cache responses for identical queries
- [ ] **Metrics Tracking** - Cost analysis and optimization suggestions
- [ ] **Streaming Responses** - Real-time token streaming
- [ ] **Multi-turn Conversations** - Maintain conversation context
- [ ] **Custom Strategies** - Plugin system for routing strategies

## Documentation

### User Guides
- **[USAGE.md](USAGE.md)** - Quick start and usage examples
- **[TESTING_GUIDE.md](docs/TESTING_GUIDE.md)** - Testing strategy and coverage

### Technical Documentation
- **[AGENTIC_SYSTEM_DESIGN.md](docs/AGENTIC_SYSTEM_DESIGN.md)** - Architecture and design
- **[SELF_DETERMINING_TASKS.md](docs/SELF_DETERMINING_TASKS.md)** - Adaptive task system
- **[PHASES.md](docs/PHASES.md)** - Implementation roadmap
- **[PLAN.md](docs/PLAN.md)** - Cost optimization strategy

### Module Documentation
- **[merlin-routing](crates/merlin-routing/README.md)** - Multi-model routing system
- **[merlin-local](crates/merlin-local/README.md)** - Local model integration (Ollama)
- **[merlin-providers](crates/merlin-providers/README.md)** - External providers (Groq, OpenRouter, Anthropic)
- **[merlin-core](crates/merlin-core/README.md)** - Core types and traits

## Testing & Benchmarking

### Running Tests

Run all tests across the workspace:
```bash
cargo test --workspace
```

Run tests for specific crates:
```bash
# Routing system tests (59 tests)
cargo test --manifest-path crates/merlin-routing/Cargo.toml

# Core tests
cargo test --manifest-path crates/merlin-core/Cargo.toml

# Provider tests
cargo test --manifest-path crates/merlin-providers/Cargo.toml

# Local model tests
cargo test --manifest-path crates/merlin-local/Cargo.toml
```

### End-to-End Tests

The routing system includes comprehensive integration test scenarios in `crates/merlin-routing/tests/integration_tests.rs`.

**Recommended Test Scenarios:**

1. **Complete Routing Flow**
   - Test: Analyze request → Route to tier → Execute → Validate
   - Verify correct tier selection based on complexity
   - Check escalation on failure

2. **Multi-Task Execution**
   - Test parallel execution of independent tasks
   - Test pipeline execution with dependencies
   - Verify conflict detection and resolution

3. **Validation Pipeline**
   - Test syntax validation (heuristics)
   - Test build validation (requires cargo project)
   - Test test execution
   - Test lint checking

4. **Workspace Isolation**
   - Test transactional workspaces
   - Test snapshot creation and rollback
   - Test file locking

5. **Provider Integration**
   - Test local model provider (requires Ollama)
   - Test Groq provider (requires API key)
   - Test fallback and escalation

6. **Error Handling**
   - Test timeout handling
   - Test rate limit handling
   - Test validation failures
   - Test conflict resolution

7. **UI Integration**
   - Test TUI event system
   - Test progress reporting
   - Test task status updates

**Running Integration Tests:**
```bash
# Run integration tests
cargo test --test integration_tests --manifest-path crates/merlin-routing/Cargo.toml

# Run with output
cargo test --test integration_tests -- --nocapture
```

**Test Coverage:**
- **Total**: 93 passing tests (74 unit + 19 integration)
- **Code Coverage**: 26.61% overall
  - Routing/Analyzer: 60-90% coverage ✅
  - TUI Components: Baseline tests added
  - Tools: 10-20% coverage
  - Providers: 20-40% coverage
- **Test Categories**:
  - Unit tests: 74 tests (inline in src/)
  - Integration tests: 19 tests (tests/ folders)
  - TUI tests: 12 tests (TaskManager)
  - CLI E2E tests: 7 tests

### Performance Benchmarking

Merlin includes comprehensive benchmarks with historical tracking:

```bash
# Run all benchmarks
cargo bench --workspace

# Compare against baseline
./scripts/benchmark_compare.sh --run --compare main

# View HTML reports
open target/criterion/report/index.html
```

**Features**:
- ✅ Criterion.rs-based benchmarks for accurate measurements
- ✅ Historical tracking and trend visualization
- ✅ Automatic regression detection (25% threshold)
- ✅ CI integration with GitHub Actions
- ✅ Local comparison scripts for development

See **[BENCHMARKING.md](docs/BENCHMARKING.md)** for detailed guide.

## Development

**Run tests:**
```bash
cargo test
```

**Run with logging:**
```bash
RUST_LOG=merlin=debug cargo run -- query "test"
```

**Check lints:**
```bash
cargo clippy
```

## Why Merlin?

Merlin is named after the **Merlin falcon** (Falco columbarius), a small but powerful bird of prey known for:

- **Speed and Agility** - Like our multi-model routing system
- **Precision** - Like our targeted code generation  
- **Adaptability** - Like our tier-based model selection
- **Efficiency** - Like our cost optimization strategies

The Merlin falcon is also known for its intelligence and ability to adapt to different environments, making it a perfect namesake for an AI coding assistant that intelligently routes between different model tiers.

## Contributing

Contributions are welcome! Please see the documentation in `docs/` for architecture details.

## License

MIT

