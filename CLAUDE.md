# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Merlin** is an intelligent AI coding assistant with multi-model routing, automatic task decomposition, and comprehensive validation. Named after the Merlin falcon for its speed, precision, and adaptability.

**Key Capabilities:**
- Multi-tier model routing (Local/Groq/Premium) with automatic escalation
- Self-determining task decomposition with dependency tracking
- Parallel task execution with conflict detection
- Multi-stage validation pipeline (syntax, build, test, lint)
- Terminal UI for real-time progress monitoring
- Interactive agent with conversation context

## Testing Guidelines

**IMPORTANT**: When writing or modifying tests, especially for UI components and input handling:
- **Never directly manipulate** `InputManager` or `TuiApp` internal state
- **Always use event sources** to inject test input
- See **[TESTS.md](./TESTS.md)** for detailed testing patterns and examples

## Essential Development Commands

### Building and Running

```bash
# Build the project (uses cranelift codegen for fast dev builds)
cargo build

# Release build (thin LTO, codegen-units=1)
cargo build --release

# CI-optimized build (used in GitHub Actions)
cargo build --profile ci-release

# Run the interactive agent
cargo run --bin merlin-cli

# Run with local models only
cargo run --bin merlin-cli -- --local

# Skip validation for faster iterations
cargo run --bin merlin-cli -- --no-validate
```

### Testing

```bash
# Run all tests across workspace
cargo test --workspace

# Run tests with coverage (requires cargo-llvm-cov)
cargo llvm-cov --lcov --output-path coverage.info --workspace --ignore-filename-regex "test_repositories|benchmarks"

# Run specific crate tests
cargo test -p merlin-routing
cargo test -p merlin-core

# Run integration tests with output
cargo test --test integration_tests -- --nocapture

# Run a single test
cargo test test_name -- --exact --nocapture
```

### Linting and Formatting

```bash
# Run clippy (very strict lints - see Cargo.toml workspace.lints)
cargo clippy --workspace --all-targets --all-features

# Format code
cargo fmt --all

# Check formatting without modifying
cargo fmt --all -- --check
```

### Benchmarking

```bash
# Run all benchmarks
cargo bench --workspace

# Run routing benchmarks only
cargo bench -p merlin-routing

# View results
open target/criterion/report/index.html
```

## Architecture

### Workspace Structure

This is a **Cargo workspace** with multiple crates organized by functionality:

**Core Crates:**
- `merlin-core` - Fundamental types, traits (`ModelProvider`), error handling
- `merlin-context` - Context management and file indexing
- `merlin-languages` - Language-specific backends (currently Rust via rust-analyzer)

**Model Integration:**
- `merlin-providers` - External API providers (Groq, OpenRouter, Anthropic)
- `merlin-local` - Local model integration via Ollama
- `merlin-agent` - Agent execution, self-assessment, step tracking

**Routing System (merlin-routing):**
- `analyzer/` - Task complexity analysis and decomposition
- `router/` - Model tier selection strategies
- `executor/` - Task execution with workspace isolation and conflict detection
- `validator/` - Multi-stage validation pipeline
- `orchestrator.rs` - High-level coordination of all routing components
- `user_interface/` - Terminal UI with ratatui (TUI) and crossterm

**CLI:**
- `merlin-cli` - Command-line interface and interactive agent
- `merlin-tools` - File operations and command execution tools

### Key Architectural Patterns

**Model Routing Flow:**
1. **Analyze** - `TaskAnalyzer` determines complexity, intent, scope
2. **Route** - `RoutingStrategy` selects model tier (Local/Groq/Premium)
3. **Execute** - `AgentExecutor` runs task with `ToolRegistry`
4. **Validate** - `ValidationPipeline` runs syntax/build/test/lint checks
5. **Escalate** - On failure, retry with higher tier (up to 3 retries)

**Task Decomposition:**
- Single tasks execute directly
- Complex tasks split into subtasks with dependencies
- Execution strategies: Sequential, Pipeline, Parallel, Hybrid
- Conflict detection prevents concurrent modifications to same files

**Workspace Isolation:**
- `TaskWorkspace` provides transactional file operations
- `WorkspaceSnapshot` enables rollback on validation failure
- `FileLockManager` prevents concurrent file modifications

**TUI Architecture:**
- `TuiApp` manages application state and event loop
- `TaskManager` displays task progress in tree structure
- `InputManager` handles user input with `tui-textarea`
- `UiChannel` receives streaming events from orchestrator

## Critical Constraints

### Strict Linting

This project has **extremely strict clippy lints** (see `Cargo.toml` lines 111-169):
- All clippy categories denied: `all`, `complexity`, `correctness`, `nursery`, `pedantic`, `perf`, `style`, `suspicious`
- Many restriction lints enabled: `expect_used`, `unwrap_used`, `todo`, `unimplemented`, `print_stdout`, `print_stderr`
- Missing documentation is denied (`missing_docs`)
- All code must use `Result<T, E>` with proper error handling

**When writing code:**
- Never use `.unwrap()` or `.expect()` - use proper error propagation
- Never use `todo!()`, `unimplemented!()`, or `unreachable!()`
- Never use `println!()` or `eprintln!()` - use `tracing` macros
- All public items must have documentation comments
- Avoid wildcards in use statements where possible
- Do NOT EVER, UNDER ANY CIRCUMSTANCES use allow

### Edition 2024 Features

The project uses **Rust Edition 2024** (Cargo.toml line 13):
- Use RPITIT (return position impl trait in traits) instead of `async-trait` where possible
- Leverage gen blocks and async generators
- Use new pattern matching features

### Toolchain Requirements

**Required:**
- Rust nightly (enforced by `rust-toolchain.toml`)
- Components: `rustfmt`, `clippy`, `rust-src`
- Cranelift codegen backend for dev profile (fast iteration)

**Optional Development:**
- `cargo-llvm-cov` for coverage
- `cargo-chef` for CI caching
- Ollama running locally for local tier testing

### Dependency Compatibility Notes

**TUI Stack (CRITICAL):**
```toml
ratatui = "0.29"        # NOT 0.30.x - incompatible with tui-textarea
crossterm = "0.28"       # NOT 0.29 - see above
tui-textarea = "0.7"     # Latest compatible version
```

Do not upgrade these dependencies independently - they must stay synchronized.

**Rust Analyzer:**
All `ra_ap_*` dependencies are pinned to `"0.0"` (latest) to track rust-analyzer releases.

## Build Optimizations

### Fast Config (.cargo/fast_config.toml)

The repository includes optimized linker configuration:
- **Linux**: Uses LLD linker with `-Zshare-generics=y -Zthreads=0`
- **macOS**: Uses default ld64 (faster than LLD on macOS)
- **Windows**: Uses `rust-lld.exe` with `-Zshare-generics=n`

**To enable locally:**
```bash
cp .cargo/fast_config.toml .cargo/config.toml
```

### Build Profiles

- `dev` - Uses cranelift backend for dependencies (fast iteration)
- `release` - Thin LTO, codegen-units=1, strip symbols (production)
- `ci-release` - Inherits release, codegen-units=16 for faster CI builds

## GitHub Actions

**Workflows:**
- `test.yml` - Multi-platform tests (Ubuntu/Windows/macOS) with coverage
- `style.yml` - Fast formatting and clippy checks
- `benchmark.yml` - Criterion benchmarks with historical tracking
- `quality_benchmarks.yml` - Runs test case benchmarks

**Custom Actions:**
- `.github/actions/setup-rust` - Installs toolchain, sets up fast config, uses cargo-chef for caching

**All workflows:**
- Use Rust nightly toolchain
- Copy `.cargo/fast_config.toml` to `.cargo/config.toml`
- Have concurrency groups to cancel redundant runs
- Set `CARGO_INCREMENTAL=0` and `CARGO_TERM_COLOR=always`

## Common Gotchas

### TUI Event Handling

When modifying `user_interface/app.rs`:
- Input must be converted to `tui_textarea::Input::from(crossterm::event::Event::Key(key))`
- Style types must come from `ratatui`, not re-exported from `tui-textarea`
- `TextArea` is rendered by reference: `frame.render_widget(&input_area, area)`

### File Operations

All file operations in task execution should use:
- `TaskWorkspace` for transactional operations
- `FileLockManager` to prevent conflicts
- `WriteFileTool`, `ReadFileTool`, `ListFilesTool` from `merlin-tools`

### Error Handling

Due to strict lints, errors must be handled explicitly:
```rust
// BAD - will not compile
let value = result.unwrap();

// GOOD
let value = result.map_err(|err| RoutingError::ExecutionFailed(err.to_string()))?;
```

### Async Context

Most agent and provider operations are async:
- Use `tokio` runtime (workspace default: full features)
- Provider trait methods are `async fn` (uses `async-trait` currently)
- Executor methods return `Future`s that must be `.await`ed

## Testing Strategy

**Unit Tests:** Inline in `src/` files with `#[cfg(test)]`
**Integration Tests:** In `tests/` directories
**Benchmarks:** In `benches/` directories (use `criterion` harness)

**TUI Testing:**
- Create mock `UiChannel` for event injection
- Use `TaskManager::new()` with test data
- Verify state transitions without rendering

**Provider Testing:**
- Mock HTTP responses for external APIs
- Use local Ollama for integration tests
- Test fallback and retry logic

## Documentation Standards

All public items require doc comments:
```rust
/// Brief one-line summary.
///
/// Detailed explanation with examples if complex.
///
/// # Errors
/// When applicable, describe error conditions.
///
/// # Examples
/// ```
/// # use merlin_routing::TaskAnalyzer;
/// let analyzer = TaskAnalyzer::new();
/// ```
pub fn function() -> Result<()> { ... }
```

## Critical Task Completion Requirements

**BEFORE marking any task as complete, you MUST:**

1. **Run clippy and fix ALL warnings/errors**:
   ```bash
   cargo clippy --workspace --all-targets --all-features
   ```
   - Fix every warning - this project has ZERO tolerance for clippy warnings
   - Never use `#[allow]` attributes outside of test modules
   - All code must pass the strict linting rules defined in Cargo.toml

2. **Run all tests and ensure they pass**:
   ```bash
   cargo test --workspace
   ```
   - All tests must pass
   - No ignored tests should be introduced without justification
   - Test coverage should increase, not decrease

3. **Verify the build succeeds**:
   ```bash
   cargo build --workspace
   ```
   - All crates must compile
   - No build warnings

**This is CRITICALLY important** - incomplete or untested code creates technical debt and breaks CI/CD.

## Verification Before Task Completion

**CRITICAL**: Before marking any task as complete, you MUST run the verification script:

```bash
./scripts/verify.sh
```

This script will:
1. Format all code with `cargo fmt`
2. Run `cargo clippy` with all warnings treated as errors
3. Run all tests across the workspace

**The verification script MUST pass with no errors.** If it fails:
- Fix all clippy warnings (never use `#[allow]` outside test modules)
- Fix all test failures
- Ensure code is properly formatted

This is the single source of truth for code quality - if `verify.sh` passes, the code is ready.
