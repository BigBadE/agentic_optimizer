# integration-tests

Unified fixture-based integration testing framework for Merlin.

## Purpose

This crate provides comprehensive integration testing using JSON fixtures. All components of Merlin are tested through fixtures that define inputs, expected outputs, and verification criteria.

**Key principle: Tests run the actual CLI, not mock implementations.**

## Module Structure

- `lib.rs` - Main exports
- `fixture.rs` - `TestFixture` definition types
- `event_source.rs` - `FixtureEventSource` for injecting test events into TUI
- `runner.rs` - `UnifiedTestRunner`, `PatternMockProvider`
- `verifier.rs` - `UnifiedVerifier` for test verification (main orchestrator)
- `verification_result.rs` - `VerificationResult` type
- `execution_verifier.rs` - Execution and return value verification logic
- `file_verifier.rs` - File verification logic
- `ui_verifier.rs` - UI and state verification logic
- `tests/fixture_tests.rs` - Auto-discovery and execution

## Fixture Structure

Fixtures are JSON files with the following structure:

```json
{
  "name": "Test name",
  "description": "Test description",
  "input": {
    "query": "User request",
    "context": ["file1.rs", "file2.rs"]
  },
  "mock_responses": {
    "pattern": "Expected response"
  },
  "verify": {
    "success": true,
    "output_contains": ["expected text"],
    "files_modified": ["file.rs"],
    "ui": {
      "task_count": 1,
      "completed_tasks": 1
    }
  }
}
```

## Fixture Organization

**54 JSON fixtures** organized by component:

- `agent/` - Agent execution tests (4 fixtures)
- `basic/` - Simple response tests (1 fixture)
- `context/` - Context building (10 fixtures)
- `execution/` - File operations (4 fixtures)
- `executor/` - Task execution (3 fixtures)
- `orchestrator/` - Orchestration (6 fixtures)
- `tools/` - Tool tests (6 fixtures)
- `tui/` - TUI tests (4 fixtures)
- `typescript/` - TypeScript runtime (9 fixtures)
- `validation/` - Validation pipeline (3 fixtures)
- `workspace/` - Workspace isolation (2 fixtures)

## Architecture

Tests instantiate the actual `TuiApp` from `merlin-cli` with:
- `TestBackend` - ratatui test backend for headless TUI testing
- `FixtureEventSource` - Injects fixture events into the TUI event loop
- Read-only access to TUI state for verification (via `test-util` feature)

**No duplicate behavior**: The test runner does not re-implement any CLI logic.

## Public API

- `TestFixture` - Fixture structure
- `UnifiedTestRunner` - Test execution using actual CLI
- `UnifiedVerifier` - Verification logic
- `PatternMockProvider` - Mock LLM responses based on patterns
- `FixtureEventSource` - Event source for fixture-based testing

## Features

### Auto-Discovery
Fixtures are automatically discovered from the `tests/fixtures/` directory:

```rust
fn discover_fixtures() -> Vec<PathBuf> {
    // Recursively finds all .json files
}
```

### Actual CLI Testing
Tests run the real CLI with fixture-based event injection:

```rust
// Create fixture event source
let event_source = Box::new(FixtureEventSource::new(&fixture));

// Create TUI app with test backend
let backend = TestBackend::new(80, 24);
let (tui_app, _) = TuiApp::new_for_test(backend, event_source, workspace_dir)?;

// Verify by reading TUI state (read-only)
let state = tui_app.test_state();
```

### Pattern-Based Mocking
`PatternMockProvider` returns responses based on query patterns:

```rust
let provider = PatternMockProvider::new();
provider.add_pattern("error handling", "Added error handling...");
```

### Comprehensive Verification
`UnifiedVerifier` checks:
- TypeScript execution results
- File modifications
- TUI state (via read-only accessors)

## Testing Status

**✅ Comprehensive**

- **All crates tested**: Via 54 fixtures
- **Auto-discovery**: Single test runner
- **Coverage verified**: Via `scripts/commit.sh`

## Code Quality

- ✅ **Documentation**: All public items documented
- ✅ **Error handling**: Proper `Result<T, E>` usage
- ✅ **No dead code**: All modules used
- ✅ **No TODOs**: Implementation complete

## Usage

### Running Tests
```bash
# Run all fixture tests
cargo nextest run -p integration-tests

# Run specific fixture category
cargo nextest run -p integration-tests orchestrator

# With coverage
./scripts/commit.sh
```

### Adding New Fixtures
1. Create JSON file in `tests/fixtures/<category>/`
2. Define input, mock responses, and verification criteria
3. Tests auto-discovered and run

### Example Fixture
```json
{
  "name": "Add error handling",
  "description": "Test adding error handling to a function",
  "input": {
    "query": "Add error handling to parse_config",
    "context": ["src/config.rs"]
  },
  "mock_responses": {
    "Add error handling": "Modified parse_config to return Result<Config, Error>"
  },
  "verify": {
    "success": true,
    "output_contains": ["Result<Config, Error>"],
    "files_modified": ["src/config.rs"]
  }
}
```

## Fixture Categories

### agent/ - Agent Execution
Tests agent task execution, tool usage, and result handling.

### context/ - Context Management
Tests context building, file inclusion, and semantic search.

### execution/ - File Operations
Tests file reading, writing, editing, and deletion.

### orchestrator/ - Orchestration
Tests task decomposition, dependency tracking, and parallel execution.

### tools/ - Tool System
Tests individual tools (edit, delete, list, show, etc.).

### tui/ - Terminal UI
Tests TUI navigation, task display, and event handling using `InputEventSource`.

### typescript/ - TypeScript Runtime
Tests TypeScript code execution with tool integration.

### validation/ - Validation Pipeline
Tests syntax, build, test, and lint validation stages.

### workspace/ - Workspace Isolation
Tests transactional workspaces and conflict detection.

## Dependencies

- `merlin-cli` - CLI and TUI (with `test-util` feature for test accessors)
- `merlin-core` - Core types
- `merlin-agent` - Agent execution
- `merlin-routing` - Routing logic
- `merlin-tooling` - Tool system
- `ratatui` - TUI framework (with `TestBackend`)
- `serde` / `serde_json` - Fixture parsing
- `tokio` - Async runtime

## Current Status

**Fully Implemented** - Integration tests now:
- ✅ Use actual `TuiApp` from `merlin-cli`
- ✅ Inject events via `FixtureEventSource`
- ✅ Read state via test-feature-gated accessors
- ✅ No duplicate CLI implementation
- ✅ Event-driven task completion using `tokio::select!`
- ✅ Comprehensive UI verification (all fields implemented)
- ✅ TaskStatus includes Pending variant for dependency tracking
- ✅ All verifier structs use `deny_unknown_fields` for type safety
