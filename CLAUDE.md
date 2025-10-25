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
- `analyzer/` - Task complexity analysis and decomposition (`local.rs`, `decompose.rs`, `intent.rs`)
- `router/` - Model tier selection strategies, provider/model registries (`tiers.rs`, `provider_registry.rs`, `model_registry.rs`)
- `cache/` - Response caching
- `metrics/` - Performance metrics collection and reporting

**Agent System (`merlin-agent`):**
- `agent/` - Agent execution (`executor.rs`), conversation tracking (`conversation.rs`), task coordination (`task_coordinator.rs`), self-assessment (`self_assess.rs`), command runner (`command_runner.rs`)
- `executor/` - Task execution with workspace isolation (`isolation.rs`), conflict detection (`graph.rs`), parallel execution pool (`pool.rs`), transaction state (`transaction.rs`)
- `validator/` - Multi-stage validation pipeline (`pipeline.rs`) with stages: syntax, build, test, lint
- `orchestrator.rs` - High-level coordination of all components

**CLI & Tools:**
- `merlin-cli` - Command-line interface and TUI (`ui/` with ratatui and crossterm)
- `merlin-tooling` - TypeScript runtime, file operations tools (`ReadFileTool`, `WriteFileTool`, `EditTool`, `DeleteTool`, `ListFilesTool`), bash execution, context requests

**Testing:**
- `integration-tests` - Unified fixture-based integration testing for all components

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
- `WorkspaceState` - Manages workspace-level state and transaction history
- `TransactionState` - Transactional file operations with rollback support
- `BuildIsolation` - Isolated build environments for validation
- `ConflictAwareTaskGraph` - Prevents concurrent file modifications through dependency tracking

**TUI Architecture:**
- `TuiApp` - Application state and event loop (`merlin-cli/src/ui/app/`)
- `TaskManager` - Task progress tree
- `InputManager` - User input with `tui-textarea`
- `UiChannel` - Streaming events from orchestrator
- `InputEventSource` - Trait for event injection (enables testing without rendering)

**TypeScript Execution:**
- `TypeScriptRuntime` - JavaScript runtime using Boa engine with tool integration
- SWC transpiler strips TypeScript types before execution
- Sandboxed execution with 30-second timeout
- Tools registered as native JavaScript functions
- Returns structured `ToolOutput` with execution results

## Critical Constraints

### Strict Linting

Extremely strict clippy lints (Cargo.toml lines 112-187):
- All clippy categories denied: `all`, `complexity`, `correctness`, `nursery`, `pedantic`, `perf`, `style`, `suspicious`
- Restriction lints: `expect_used`, `unwrap_used`, `todo`, `unimplemented`, `print_stdout`, `print_stderr`, and many more
- `missing_docs` denied
- All code uses `Result<T, E>` with proper error handling

**Requirements:**
- Never use `.unwrap()`, `.expect()`, `todo!()`, `unimplemented!()`, `unreachable!()`
- Never use `println!()`/`eprintln!()` - use `tracing` macros
- All public items need doc comments
- Never use `#[allow]` outside test modules (test modules use `#[cfg_attr(test, allow(...))]`)
- Do NOT use `cargo clean`, ever. On ICE (Internal Compiler Error), delete only incremental compilation caches: `target/debug/incremental` or `target/release/incremental`, NOT the entire profile folder.

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

**SWC TypeScript Transpiler:**
All SWC dependencies must be updated together for compatibility:
```toml
swc_common = "15.0"
swc_ecma_ast = "16.0"
swc_ecma_codegen = "18.0"
swc_ecma_parser = "25.0"
swc_ecma_transforms_base = "28.0"
swc_ecma_transforms_typescript = "31.0"
swc_ecma_visit = "16.0"
```

**Rust Analyzer:**
All `ra_ap_*` dependencies pinned to `"0.0"` (latest).

### Build Configuration

**Profiles:**
- `dev` - Optimized dependencies (opt-level=3), incremental compilation, minimal debug info
- `release` - Thin LTO, codegen-units=1, panic=abort, full optimization
- `bench` - Debug symbols enabled for profiling
- `ci` - Optimized for size (opt-level=s), minimal disk usage for CI/CD

**Fast Config (`.cargo/fast_config.toml`):**
- Linux: Clang + LLD linker (optional: mold), nightly flags commented out
- macOS: Default ld64 (fastest, LLD option commented out)
- Windows: `rust-lld.exe` linker, nightly flags commented out
- `checksum-freshness = true` prevents unnecessary rebuilds on timestamp changes

**Toolchain:**
- Nightly required (`rust-toolchain.toml`)
- Components: `rustfmt`, `clippy`, `rust-src`

## Common Gotchas

### TUI Event Handling

When modifying TUI code (`merlin-cli/src/ui/`):
- Input converts to `tui_textarea::Input::from(crossterm::event::Event::Key(key))`
- Style types from `ratatui`, not `tui-textarea`
- `TextArea` rendered by reference: `frame.render_widget(&input_area, area)`
- Use `InputEventSource` trait for all input to enable fixture-based testing
- Never manipulate `InputManager` or `TuiApp` internal state directly - use fixtures

### File Operations

Available tools from `merlin-tooling`:
- `ReadFileTool` - Read file contents
- `WriteFileTool` - Create or overwrite files
- `EditTool` - Make targeted edits to existing files
- `DeleteTool` - Delete files or directories
- `ListFilesTool` - List directory contents
- `BashTool` - Execute shell commands
- `ContextRequestTool` - Request additional context from the user

All file operations in task execution use:
- `TransactionState` for atomic operations with rollback
- `ConflictAwareTaskGraph` to prevent concurrent file modifications

### Error Handling

```rust
// BAD - will not compile
let value = result.unwrap();

// GOOD
let value = result.map_err(|err| RoutingError::ExecutionFailed(err.to_string()))?;
```

## Testing

**CRITICAL TESTING PHILOSOPHY:**

**ALWAYS prefer JSON fixtures over manual tests.** Fixtures provide comprehensive, reproducible testing across all system components with minimal code duplication.

### Unified Fixture System

All tests use JSON fixtures in `crates/integration-tests/tests/fixtures/`:

**Fixture Categories:**
- `agent/` - Agent executor, conversation context, tool usage, task execution
- `basic/` - Simple response handling
- `cli/` - Command-line interface operations
- `context/` - Context fetching, conversation building, file references
- `execution/` - File operations, error handling, state transitions
- `executor/` - Task decomposition, self-determining execution
- `orchestrator/` - Task graphs, conflict detection, dependencies
- `task_lists/` - Task list execution patterns
- `tools/` - Individual tool operations (show, edit, delete, list)
- `tui/` - Terminal UI navigation, rendering, event handling
- `typescript/` - TypeScript execution, type stripping, control flow
- `validation/` - Validation pipeline stages
- `workspace/` - Workspace isolation, transactions, file locking

**Fixture Format:** (`crates/integration-tests/src/fixture.rs`)
```json
{
  "name": "Test Name",
  "description": "What this test verifies",
  "tags": ["category", "feature"],
  "setup": {
    "files": {"path/to/file.txt": "content"},
    "env_vars": {"VAR": "value"},
    "terminal_size": [80, 24]
  },
  "events": [
    {
      "type": "user_input",
      "data": {"text": "Do something", "submit": true},
      "verify": {
        "execution": {"typescript_executed": true},
        "files": [{"path": "output.txt", "contains": ["expected"]}],
        "ui": {"input_cleared": true}
      }
    },
    {
      "type": "llm_response",
      "trigger": {"pattern": "Do something", "match_type": "contains"},
      "response": {"typescript": ["function agent_code() { return 'done'; }"]},
      "verify": {"execution": {"return_value_matches": "done"}}
    }
  ],
  "final_verify": {
    "execution": {"validation_passed": true}
  }
}
```

**Test Runner:** `crates/integration-tests/tests/unified_tests.rs`
- Auto-discovers all fixtures
- Runs them with `UnifiedTestRunner`
- Verifies execution, file state, UI state, and final outcomes

### When to Add Tests

**DO add a test if:**
- The behavior is NOT covered by existing fixtures
- You're adding a new tool or capability
- You're testing edge cases or error conditions not yet covered
- You're adding a new validation stage or execution pattern

**DO NOT add a test if:**
- It duplicates functionality already tested in fixtures
- Example: Don't add a test for "submitting a message" - fixtures already cover this
- Example: Don't add a test for "basic file read" - `tools/show_tool.json` covers this
- Example: Don't test UI event handling directly - use fixtures instead

**Before adding a test:**
1. Search existing fixtures: `rg "pattern" crates/integration-tests/tests/fixtures/`
2. Check if a similar test exists in the relevant category
3. If similar tests exist, extend an existing fixture or create a new one only if testing a distinct scenario

### Manual Testing (Limited Use)

**Unit tests** (inline with `#[cfg(test)]`):
- Only for testing internal logic of specific functions
- Must not duplicate fixture coverage
- Use when testing private implementation details

**Examples of acceptable unit tests:**
- Testing a pure calculation function
- Testing error parsing logic
- Testing data structure transformations

**Benchmarks** (`benchmarks/crates/`):
- Performance regression testing with criterion
- Quality benchmarks for LLM routing decisions

### TUI Testing

**NEVER manipulate TUI state directly.** Always use fixtures with event injection:
- `InputEventSource` trait enables fixture-based event injection
- Fixtures define `user_input` and `key_press` events
- `PatternMockProvider` simulates LLM responses based on patterns
- Verify UI state through fixture `verify.ui` blocks

**Example TUI fixture:** `crates/integration-tests/tests/fixtures/tui/basic_navigation.json`

## Verification Before Completion

**Fast verification (recommended for development):**

```bash
./scripts/verify.sh
```

This script:
1. Formats code (`cargo fmt`)
2. Runs clippy with warnings as errors
3. Runs all tests with `cargo nextest run --run-ignored all`

**Full verification with coverage (run before commits):**

```bash
./scripts/commit.sh
```

This script runs everything from `verify.sh` plus:
1. Coverage instrumentation with `cargo llvm-cov` on `--lib --tests` (excludes benchmark crates)
2. Shows profraw file count and size (typically 1000+ files, 10-20GB)
3. Manually merges profraw files into single profdata using `llvm-profdata merge -sparse`
4. Deletes profraw files to save disk space (~18GB freed)
5. Generates lcov report using `--instr-profile` (fast, no re-merge)
6. Generates HTML report using `--instr-profile` (fast, no re-merge)
7. Deletes profdata file after reports
8. Stages coverage report for commit
9. Runs `cargo sweep` to clean old build artifacts

**Performance optimization:**
- Merges profraw files once manually (~40s)
- Both reports use the merged profdata via `--instr-profile` (no re-merge, ~5s each)
- Without manual merge: 80s (40s per report × 2)
- With manual merge: 50s (40s merge + 5s + 5s reports)
- Profraw files: ~1000 files, 10-20GB → merged profdata: ~25MB
- After cleanup: Only instrumented binaries remain (~8GB) for faster incremental runs

**Optional flags (both scripts):**
- `--no-cloud` - Disable cloud provider tests (unsets API keys)
- `--ollama` - Run Ollama-specific tests (requires local Ollama server)
- `--html` - Generate HTML coverage report (commit.sh only)

**Must pass with zero errors.** Never use `#[allow]` to silence warnings.

## Running Tests

**IMPORTANT: Never use `cargo test` directly.** Always use one of the following:

**For all tests (recommended):**
```bash
./scripts/verify.sh
```

**For specific tests (use nextest):**
```bash
cargo nextest run -p <package> <test_name>
```

Examples:
```bash
# Run specific test in a package
cargo nextest run -p merlin-cli test_prompt_command_shows_context

# Run all tests in a package
cargo nextest run -p merlin-agent

# Run with timeout
cargo nextest run -p merlin-cli --test-threads=4
```

**Why nextest over cargo test:**
- Respects build cache and profiles correctly
- Parallel execution with better isolation
- Cleaner output and better timeout handling
- Avoids unnecessary rebuilds of dependency tree

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
