# Agentic Optimizer

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
cd agentic_optimizer
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

**Show configuration:**
```bash
cargo run -- config
```

**Show metrics:**
```bash
cargo run -- metrics --daily
```

## Configuration

Create a `config.toml` file (see `config.example.toml`):

```toml
[providers]
anthropic_api_key = "sk-ant-..."

[context]
max_files = 50
max_file_size = 100000
```

Or use environment variables:
- `ANTHROPIC_API_KEY` - Your Anthropic API key

## Architecture

```
agentic_optimizer/
├── crates/
│   └── agentic_optimizer/
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

See the `docs/` folder for detailed documentation:
- `PLAN.md` - Cost analysis and optimization strategies
- `DESIGN.md` - High-level architecture
- `ARCHITECTURE.md` - Module design and traits
- `PHASES.md` - Phase-by-phase implementation guide

## Development

**Run tests:**
```bash
cargo test
```

**Run with logging:**
```bash
RUST_LOG=agentic_optimizer=debug cargo run -- query "test"
```

**Check lints:**
```bash
cargo clippy
```

## License

MIT
