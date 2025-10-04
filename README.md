# Merlin

A cost-optimized AI coding agent that reduces API costs by up to 96% while maintaining high quality through intelligent routing, local models, and minimal context strategies.

## Current Status: Phase 0 (MVP)

**Features:**
- ✅ CLI interface for queries
- ✅ Claude Sonnet 4.5 integration
- ✅ Basic context building
- ✅ Cost tracking per request
- ✅ File-based context selection

**Cost:** ~$15/day (baseline, no optimizations yet)

## Quick Start

### Prerequisites

- Rust 1.75+ (edition 2024)
- Anthropic API key

### Installation

1. Clone the repository:
```bash
git clone <repo-url>
cd merlin
```

2. Set up your API key:
```bash
# Windows (PowerShell)
$env:ANTHROPIC_API_KEY="sk-ant-..."

# Linux/Mac
export ANTHROPIC_API_KEY="sk-ant-..."
```

3. Build the project:
```bash
cargo build --release
```

### Usage

**Multi-Model Routing (NEW!):**
```bash
# Simple request with automatic tier selection (TUI mode by default)
cargo run -- route "Add error handling to the parser"

# Complex refactor with validation
cargo run -- route "Refactor the parser module" --validate

# Plain terminal output (disable TUI)
cargo run -- route "Add logging to main.rs" --no-tui --verbose

# Available flags:
#   --validate          Enable validation pipeline (syntax, build, test, lint)
#   --verbose           Show detailed routing decisions (non-TUI mode only)
#   --no-tui            Disable TUI mode, use plain terminal output
#   -p, --project PATH  Project root directory (default: current directory)
```

**Ask a question:**
```bash
cargo run -- query "Find the main function"
```

**Query with specific files:**
```bash
cargo run -- query "Refactor this function" --files src/main.rs src/lib.rs
```

**Query with project path:**
```bash
cargo run -- query "Explain the architecture" --project ./my-project
```

**Interactive chat:**
```bash
cargo run -- chat
```

**Show configuration:**
```bash
cargo run -- config
```

**Show metrics:**
```bash
cargo run -- metrics --daily
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
        enabled: false,  // Use --validate flag
        early_exit: true,
        syntax_check: true,
        build_check: true,
        test_check: true,
        lint_check: true,
    },
    execution: ExecutionConfig {
        max_concurrent_tasks: 4,
        enable_parallel: true,
        enable_conflict_detection: true,
    },
}
```

## Architecture

```
merlin/
├── crates/
│   └── merlin/
│       └── src/
│           ├── core/          # Core types and traits
│           ├── providers/     # Model provider implementations
│           ├── context/       # Context building
│           ├── cli/           # CLI interface
│           └── config/        # Configuration
├── docs/                      # Documentation
│   ├── PLAN.md               # Cost optimization plan
│   ├── DESIGN.md             # Architecture overview
│   ├── ARCHITECTURE.md       # Detailed module design
│   └── PHASES.md             # Implementation phases
└── config.example.toml       # Example configuration
```

## Context Fetching Performance

**Current Metrics** (Phase 5 - Graph Ranking):
- **Precision@3**: 30.0% (Target: 60%)
- **Recall@10**: 49.4% (Target: 70%)
- **MRR**: 0.440 (Target: 0.700)
- **NDCG@10**: 0.437 (Target: 0.750)
- **Critical in Top-3**: 25.0% (Target: 65%)

*Targets set to industry tool levels (Cursor/GitHub Copilot estimated performance). Achieving these requires solving documentation dominance fundamentally.*

See `benchmarks/README.md` for improvement roadmap.

## Roadmap

- [x] **Phase 0 (MVP)** - Basic Sonnet-only agent
- [ ] **Phase 1** - Context optimization (60% cost reduction)
- [ ] **Phase 2** - Output optimization (35% token reduction)
- [ ] **Phase 3** - Multi-model routing (Groq/Gemini)
- [ ] **Phase 4** - Local model integration (70% local)
- [ ] **Phase 5** - Advanced optimizations

**Target:** $0.50/day (96% cost reduction)

## Documentation

### Main Documentation
- **[FINAL_SUMMARY.md](docs/FINAL_SUMMARY.md)** - Complete overview and quick start
- **[PRODUCTION_READY.md](docs/PRODUCTION_READY.md)** - Production readiness guide
- **[ROUTING_ARCHITECTURE.md](docs/ROUTING_ARCHITECTURE.md)** - Complete architecture (11 phases)
- **[CLI_ROUTING.md](docs/CLI_ROUTING.md)** - CLI usage and examples

### Module Documentation
- **[merlin-routing](crates/merlin-routing/README.md)** - Multi-model routing system
- **[merlin-local](crates/merlin-local/README.md)** - Local model integration (Ollama)
- **[merlin-providers](crates/merlin-providers/README.md)** - External providers (Groq, OpenRouter, Anthropic)
- **[merlin-core](crates/merlin-core/README.md)** - Core types and traits

### Legacy Documentation
- `PLAN.md` - Cost analysis and optimization strategies
- `DESIGN.md` - High-level architecture
- `ARCHITECTURE.md` - Module design and traits
- `PHASES.md` - Phase-by-phase implementation guide

## Testing

### Unit Tests

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
- **Total**: 59 passing tests
- **Analyzer**: 18 tests (intent, complexity, decomposition)
- **Router**: 13 tests (strategies, tier selection)
- **Executor**: 12 tests (graph, pool, isolation)
- **Validator**: 11 tests (pipeline, stages)
- **Config**: 2 tests (serialization, defaults)
- **Orchestrator**: 3 tests (analysis, execution)

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

## License

MIT

