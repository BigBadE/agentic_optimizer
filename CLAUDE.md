# CLAUDE.md

This file provides guidance when working with code in this repository.

## Project Overview

**Merlin** is an intelligent AI coding assistant with multi-model routing, automatic task decomposition, and comprehensive validation. Named after the Merlin falcon for its speed, precision, and adaptability.

**Key Capabilities:**
- Multi-tier model routing (Local/Groq/Premium) with automatic escalation
- Self-determining task decomposition with dependency tracking
- Parallel task execution with conflict detection
- Multi-stage validation pipeline (syntax, build, test, lint)
- Terminal UI for real-time progress monitoring
- Interactive agent with conversation context

## Architecture

### Workspace Structure

Cargo workspace with multiple crates:

**Core:**
- `merlin-core` - Fundamental types, traits (`ModelProvider`), error handling
- `merlin-context` - Context management, file indexing
- `merlin-languages` - Language-specific backends (Rust via rust-analyzer)

**Model Integration:**
- `merlin-providers` - External API providers (Groq, OpenRouter, Anthropic)
- `merlin-local` - Local model integration via Ollama
- `merlin-agent` - Agent execution, self-assessment, step tracking

**Routing System (`merlin-routing`):**
- `agent/` - Agent execution (`executor.rs`), context management (`context_manager.rs`, `context_fetcher.rs`), conversation tracking (`conversation.rs`), task coordination (`task_coordinator.rs`), self-assessment (`self_assess.rs`)
- `analyzer/` - Task complexity analysis and decomposition
- `router/` - Model tier selection strategies
- `executor/` - Task execution with workspace isolation and conflict detection
- `validator/` - Multi-stage validation pipeline
- `orchestrator.rs` - High-level coordination of all components
- `user_interface/` - Terminal UI (TUI) with ratatui and crossterm

**CLI:**
- `merlin-cli` - Command-line interface and interactive agent
- `merlin-tools` - File operations and command execution tools

### Key Patterns

**Model Routing:**
1. Analyze - `TaskAnalyzer` determines complexity, intent, scope
2. Route - `RoutingStrategy` selects model tier
3. Execute - `AgentExecutor` runs task with `ToolRegistry`
4. Validate - `ValidationPipeline` runs checks
5. Escalate - Retry with higher tier on failure (up to 3 retries)

**Task Decomposition:**
- Complex tasks split into subtasks with dependencies
- Execution strategies: Sequential, Pipeline, Parallel, Hybrid
- Conflict detection prevents concurrent file modifications

**Workspace Isolation:**
- `TaskWorkspace` - Transactional file operations
- `WorkspaceSnapshot` - Rollback on validation failure
- `FileLockManager` - Prevents concurrent modifications

**TUI Architecture:**
- `TuiApp` - Application state and event loop
- `TaskManager` - Task progress tree
- `InputManager` - User input with `tui-textarea`
- `UiChannel` - Streaming events from orchestrator
- All input sourced through `InputEventSource` trait for test injection

## Critical Constraints

### Strict Linting

Extremely strict clippy lints (Cargo.toml lines 111-172):
- All clippy categories denied: `all`, `complexity`, `correctness`, `nursery`, `pedantic`, `perf`, `style`, `suspicious`
- Restriction lints: `expect_used`, `unwrap_used`, `todo`, `unimplemented`, `print_stdout`, `print_stderr`, and many more
- `missing_docs` denied
- All code uses `Result<T, E>` with proper error handling

**Requirements:**
- Never use `.unwrap()`, `.expect()`, `todo!()`, `unimplemented!()`, `unreachable!()`
- Never use `println!()`/`eprintln!()` - use `tracing` macros
- All public items need doc comments
- Never use `#[allow]` outside test modules (test modules use `#[cfg_attr(test, allow(...))]`)
- Do NOT use cargo clean, ever. Instead, delete specific profile folders, like target/debug, on ICE.

### Rust Edition 2024

Uses Rust Edition 2024 (Cargo.toml line 11):
- Prefer RPITIT over `async-trait` where possible
- Leverage gen blocks and async generators

### Dependencies

**TUI Stack (CRITICAL - DO NOT UPGRADE):**
```toml
ratatui = "0.29"        # NOT 0.30+ (incompatible with tui-textarea)
crossterm = "0.28"      # NOT 0.29+ (incompatible with tui-textarea)
tui-textarea = "0.7"
```

**Rust Analyzer:**
All `ra_ap_*` dependencies pinned to `"0.0"` (latest).

### Build Configuration

**Profiles:**
- `dev` - Cranelift backend for dependencies (fast iteration)
- `release` - Thin LTO, codegen-units=1, strip symbols
- `bench` - Debug symbols enabled

**Fast Config (`.cargo/fast_config.toml`):**
- Linux: LLD linker with `-Zshare-generics=y -Zthreads=0`
- macOS: Default ld64
- Windows: `rust-lld.exe` with `-Zshare-generics=n`

**Toolchain:**
- Nightly required (`rust-toolchain.toml`)
- Components: `rustfmt`, `clippy`, `rust-src`

## Common Gotchas

### TUI Testing

**CRITICAL - Never manipulate internal state directly in tests:**
- Use `InputEventSource` trait to inject events
- Create test event sources implementing the trait
- Example in `tests/scenario_runner.rs:TestEventSource`
- All UI tests use event injection, not direct state manipulation

### TUI Event Handling

When modifying `user_interface/app.rs`:
- Input converts to `tui_textarea::Input::from(crossterm::event::Event::Key(key))`
- Style types from `ratatui`, not `tui-textarea`
- `TextArea` rendered by reference: `frame.render_widget(&input_area, area)`

### File Operations

Use in task execution:
- `TaskWorkspace` for transactional operations
- `FileLockManager` to prevent conflicts
- `WriteFileTool`, `ReadFileTool`, `ListFilesTool` from `merlin-tools`

### Error Handling

```rust
// BAD - will not compile
let value = result.unwrap();

// GOOD
let value = result.map_err(|err| RoutingError::ExecutionFailed(err.to_string()))?;
```

## Testing

**Structure:**
- Unit tests: Inline in `src/` with `#[cfg(test)]`
- Integration tests: `tests/integration/`
- E2E tests: `tests/e2e/`
- Scenario-based tests: JSON fixtures in `tests/fixtures/scenarios/`
- Benchmarks: `benches/` (criterion)

**Scenario Testing:**
- JSON-based test scenarios in `tests/fixtures/scenarios/`
- Snapshots in `tests/fixtures/snapshots/`
- Runner in `tests/scenario_runner.rs`
- Supports UI state verification, event injection, task spawning

**TUI Testing:**
- Mock `UiChannel` for event injection
- Test event sources implementing `InputEventSource`
- Verify state without rendering
- Never manipulate `InputManager` or `TuiApp` internal state

## Verification Before Completion

**CRITICAL - Run before marking any task complete:**

```bash
./scripts/verify.sh
```

This script:
1. Formats code (`cargo fmt`)
2. Runs clippy with warnings as errors
3. Runs all tests (workspace, lib, bins, tests)

**Optional flags:**
- `--no-cloud` - Disable cloud provider tests
- `--ollama` - Run Ollama-specific tests

**Must pass with zero errors.** Never use `#[allow]` to silence warnings.

## Documentation Standards

All public items need doc comments:
```rust
/// Brief one-line summary.
///
/// Detailed explanation with examples if complex.
///
/// # Errors
/// Describe error conditions when returning `Result`.
///
/// # Examples
/// ```
/// # use merlin_routing::TaskAnalyzer;
/// let analyzer = TaskAnalyzer::new();
/// ```
pub fn function() -> Result<()> { ... }
```
