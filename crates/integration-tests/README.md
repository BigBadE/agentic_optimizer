# integration-tests

Unified fixture-based integration testing framework for Merlin.

## Purpose

This crate provides comprehensive integration testing using JSON fixtures. All components of Merlin are tested through fixtures that define inputs, expected outputs, and verification criteria.

## Module Structure

- `lib.rs` - Main exports
- `fixture.rs` - `TestFixture` definition types
- `runner.rs` - `UnifiedTestRunner`, `PatternMockProvider`
- `verifier.rs` - `UnifiedVerifier` for test verification
- `tests/unified_tests.rs` - Auto-discovery and execution

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

## Public API

- `TestFixture` - Fixture structure
- `UnifiedTestRunner` - Test execution
- `UnifiedVerifier` - Verification logic
- `PatternMockProvider` - Mock LLM responses based on patterns

## Features

### Auto-Discovery
Fixtures are automatically discovered from the `tests/fixtures/` directory:

```rust
fn discover_fixtures() -> Vec<PathBuf> {
    // Recursively finds all .json files
}
```

### Pattern-Based Mocking
`PatternMockProvider` returns responses based on query patterns:

```rust
let provider = PatternMockProvider::new();
provider.add_pattern("error handling", "Added error handling...");
```

### Comprehensive Verification
`UnifiedVerifier` checks:
- Success/failure status
- Output content
- Modified files
- UI state (task counts, completion)
- Custom verification logic

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

- `merlin-core` - Core types
- `merlin-agent` - Agent execution
- `merlin-routing` - Routing logic
- `merlin-tooling` - Tool system
- `serde` / `serde_json` - Fixture parsing
- `tokio` - Async runtime

## Issues and Recommendations

**None** - This crate provides comprehensive fixture-based testing coverage for the entire Merlin system.
