# Comprehensive Testing Guide for Merlin

**Version**: 2.0
**Date**: 2025-10-07
**Status**: ✅ Implemented

---

## Implementation Status

### ✅ Completed
- Code coverage tooling set up (cargo-llvm-cov)
- Coverage profile configured in Cargo.toml
- Comprehensive TUI tests (TaskManager, Renderer)
- CLI E2E tests
- Common test utilities and helpers
- All inline tests passing (74 tests)
- New integration tests (19 tests)

### Current Coverage: **26.61%**

#### Coverage Breakdown
- **TUI Components** (app, renderer, task_manager, theme, etc.): 0% → **Needs tests**
- **Routing Logic** (analyzer, router, executor): 60-90% ✅
- **Tools** (bash, edit, show): 10-20% → **Needs tests**
- **Providers & Core**: 20-40% → **Needs tests**

---

## Table of Contents

1. [Testing Philosophy](#testing-philosophy)
2. [Test Tools & Setup](#test-tools--setup)
3. [Unit Testing](#unit-testing)
4. [Integration Testing](#integration-testing)
5. [E2E CLI Testing](#e2e-cli-testing)
6. [TUI Testing](#tui-testing)
7. [Agent Testing](#agent-testing)
8. [Test Organization](#test-organization)
9. [CI/CD Integration](#cicd-integration)

---

## Testing Philosophy

### Goals
- **Prevent Regressions** - Catch bugs before users do
- **Document Behavior** - Tests as living documentation
- **Enable Refactoring** - Confidently improve code
- **Fast Feedback** - Quick iteration cycles

### Testing Pyramid
```
        ┌─────────────┐
        │   E2E (5%)  │  ← Few, slow, high-value
        ├─────────────┤
        │  Integ (25%)│  ← Component interactions
        ├─────────────┤
        │  Unit (70%) │  ← Many, fast, focused
        └─────────────┘
```

---

## Test Tools & Setup

### Dependencies Already Added

Workspace-level (`Cargo.toml`):
```toml
assert_cmd = "2.0"      # CLI testing
predicates = "3.0"      # Output assertions
tempfile = "3.14"       # Temp directories
wiremock = "0.6"        # Mock HTTP servers
mockall = "0.13"        # Mock traits
proptest = "1.5"        # Property-based testing
serial_test = "3.1"     # Serialize shared-state tests
insta = "1.40"          # Snapshot testing
tokio-test = "0.4"      # Async test utilities
```

CLI crate (`merlin-cli/Cargo.toml`):
```toml
[dev-dependencies]
assert_cmd.workspace = true
predicates.workspace = true
tempfile.workspace = true
insta.workspace = true
serial_test.workspace = true
```

---

## Unit Testing

### 1. Task Manager Tests

**File**: `crates/merlin-routing/src/user_interface/task_manager.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_order_preserved_after_rebuild() {
        let mut manager = TaskManager::new();
        let now = Instant::now();
        
        // Add older task
        let task1 = create_test_task("First", now);
        let task1_id = TaskId::new();
        manager.add_task(task1_id, task1);
        
        // Add newer task
        let task2 = create_test_task("Second", now + Duration::from_secs(1));
        let task2_id = TaskId::new();
        manager.add_task(task2_id, task2);
        
        // Simulate reload
        manager.rebuild_order();
        
        // Verify order: older first, newer last
        assert_eq!(manager.task_order()[0], task1_id);
        assert_eq!(manager.task_order()[1], task2_id);
    }
    
    #[test]
    fn test_delete_removes_descendants() {
        let mut manager = TaskManager::new();
        
        let parent_id = TaskId::new();
        let child_id = TaskId::new();
        
        manager.add_task(parent_id, create_test_task("Parent", Instant::now()));
        manager.add_task(child_id, create_child_task("Child", parent_id));
        
        let deleted = manager.remove_task(parent_id);
        
        assert_eq!(deleted.len(), 2);
        assert!(manager.is_empty());
    }
    
    fn create_test_task(desc: &str, start: Instant) -> TaskDisplay {
        TaskDisplay {
            description: desc.to_string(),
            status: TaskStatus::Running,
            start_time: start,
            end_time: None,
            parent_id: None,
            progress: None,
            output_lines: vec![],
            output_tree: OutputTree::new(),
            steps: vec![],
        }
    }
}
```

### 2. Router Tests

**File**: `crates/merlin-routing/src/router/mod.rs`

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_complexity_routing() {
        let router = Router::new(RoutingConfig::default());
        
        // Simple -> Local
        let simple = AnalyzedRequest {
            complexity: ComplexityLevel::Simple,
            estimated_tokens: 100,
            ..Default::default()
        };
        assert_eq!(router.select_tier(&simple), ModelTier::Local);
        
        // Complex -> Premium
        let complex = AnalyzedRequest {
            complexity: ComplexityLevel::Complex,
            estimated_tokens: 5000,
            ..Default::default()
        };
        assert_eq!(router.select_tier(&complex), ModelTier::Premium);
    }
}
```

---

## Integration Testing

### Orchestrator Integration

**File**: `crates/merlin-routing/tests/orchestrator_integration.rs`

```rust
use merlin_routing::*;
use tempfile::TempDir;

#[tokio::test]
async fn test_end_to_end_request_flow() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    
    // Create test file
    std::fs::write("test.rs", "fn main() {}\n").unwrap();
    
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    
    let result = orchestrator.process_request("Add a comment").await;
    
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

#[tokio::test]
async fn test_task_decomposition() {
    let orchestrator = RoutingOrchestrator::new(RoutingConfig::default());
    
    let complex = "Create authentication module with tests";
    let analysis = orchestrator.analyze_request(complex).await.unwrap();
    
    assert_eq!(analysis.complexity, ComplexityLevel::Complex);
}
```

### Mock Provider Tests

```rust
use mockall::mock;

mock! {
    Provider {}
    
    #[async_trait]
    impl ModelProvider for Provider {
        async fn generate(&self, query: &Query, context: &Context) -> Result<Response>;
        fn name(&self) -> &'static str;
        async fn is_available(&self) -> bool;
        fn estimate_cost(&self, context: &Context) -> f64;
    }
}

#[tokio::test]
async fn test_with_mock_provider() {
    let mut mock = MockProvider::new();
    mock.expect_generate()
        .returning(|_, _| Ok(Response {
            text: "Mock response".to_string(),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "mock".to_string(),
            latency_ms: 0,
        }));
    
    let agent = Agent::new(Arc::new(mock));
    let result = agent.execute("test").await;
    
    assert!(result.is_ok());
}
```

---

## E2E CLI Testing

**File**: `crates/merlin-cli/tests/cli_e2e.rs`

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_cli_help() {
    Command::cargo_bin("merlin").unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Merlin"));
}

#[test]
fn test_cli_query() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("test.rs"), "fn main() {}").unwrap();
    
    Command::cargo_bin("merlin").unwrap()
        .current_dir(temp.path())
        .arg("query")
        .arg("Find main")
        .arg("--no-tui")
        .assert()
        .success();
}

#[test]
fn test_cli_validation() {
    let temp = TempDir::new().unwrap();
    
    // Create Cargo project
    std::fs::create_dir(temp.path().join("src")).unwrap();
    std::fs::write(temp.path().join("Cargo.toml"), 
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
    ).unwrap();
    std::fs::write(temp.path().join("src/main.rs"), 
        "fn main() {}"
    ).unwrap();
    
    Command::cargo_bin("merlin").unwrap()
        .current_dir(temp.path())
        .arg("query")
        .arg("Add comment")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}

#[test]
fn test_file_operations() {
    let temp = TempDir::new().unwrap();
    
    Command::cargo_bin("merlin").unwrap()
        .current_dir(temp.path())
        .arg("query")
        .arg("Create hello.txt")
        .arg("--no-tui")
        .assert()
        .success();
    
    assert!(temp.path().join("hello.txt").exists());
}
```

---

## TUI Testing

**File**: `crates/merlin-routing/tests/tui_tests.rs`

```rust
use merlin_routing::user_interface::*;

#[test]
fn test_task_selection() {
    let mut manager = TaskManager::new();
    let mut state = UiState::default();
    
    let task_id = TaskId::new();
    manager.add_task(task_id, create_test_task());
    
    state.selected_task_index = 0;
    state.active_task_id = Some(task_id);
    
    let visible = manager.get_visible_tasks();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0], task_id);
}

#[test]
fn test_collapse_expand() {
    let mut manager = TaskManager::new();
    
    let parent_id = TaskId::new();
    let child_id = TaskId::new();
    
    manager.add_task(parent_id, create_test_task());
    manager.add_task(child_id, create_child_task(parent_id));
    
    // Initially visible
    assert_eq!(manager.get_visible_tasks().len(), 2);
    
    // Collapse
    manager.collapse_task(parent_id);
    assert_eq!(manager.get_visible_tasks().len(), 1);
    
    // Expand
    manager.expand_task(parent_id);
    assert_eq!(manager.get_visible_tasks().len(), 2);
}

#[test]
fn test_rendering() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let manager = create_test_manager();
    let renderer = Renderer::new(Theme::default());
    
    terminal.draw(|frame| {
        let ctx = create_test_render_ctx(&manager);
        renderer.render(frame, &ctx);
    }).unwrap();
    
    let buffer = terminal.backend().buffer();
    let text: String = buffer.content().iter()
        .map(|c| c.symbol())
        .collect();
    
    assert!(text.contains("Task"));
}
```

---

## Agent Testing

**File**: `crates/merlin-agent/tests/agent_reasoning.rs`

```rust
#[tokio::test]
async fn test_tool_selection() {
    let provider = create_mock_with_response(
        r#"<tool_call name="edit">...</tool_call>"#
    );
    
    let agent = Agent::new(provider);
    let result = agent.execute("Fix bug").await.unwrap();
    
    assert!(result.steps.iter().any(|s| 
        matches!(s.step_type, StepType::ToolCall)
    ));
}

#[tokio::test]
async fn test_context_accumulation() {
    let responses = vec![
        "Let me check. <tool_call name=\"show\">...</tool_call>",
        "Now I'll edit. <tool_call name=\"edit\">...</tool_call>",
    ];
    
    let agent = Agent::new(create_mock_with_responses(responses));
    let result = agent.execute("Refactor").await.unwrap();
    
    assert!(result.steps.len() >= 2);
    assert!(!result.accumulated_context.is_empty());
}
```

---

## Test Organization

### Directory Structure

```
crates/merlin-cli/
  tests/
    cli_e2e.rs           # End-to-end CLI tests
    config_tests.rs      # Configuration testing

crates/merlin-routing/
  src/
    user_interface/
      task_manager.rs    # Unit tests inline
    router/
      mod.rs             # Unit tests inline
  tests/
    orchestrator_integration.rs
    tui_tests.rs
    mock_provider_tests.rs

crates/merlin-agent/
  tests/
    agent_reasoning.rs
    tool_execution.rs
```

### Test Helpers

Create `tests/common/mod.rs`:
```rust
pub fn create_test_task() -> TaskDisplay { ... }
pub fn create_temp_project() -> TempDir { ... }
pub fn create_mock_provider() -> MockProvider { ... }
```

---

## CI/CD Integration

### GitHub Actions Workflow

**File**: `.github/workflows/test.yml`

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rust-lang/setup-rust-toolchain@v1
    
    - name: Cache dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Run unit tests
      run: cargo test --workspace --lib
    
    - name: Run integration tests
      run: cargo test --workspace --test '*'
    
    - name: Run E2E tests
      run: cargo test --package merlin-cli --test cli_e2e
    
    - name: Check clippy
      run: cargo clippy --all-targets --all-features
```

### Pre-commit Hook

**File**: `.git/hooks/pre-commit`

```bash
#!/bin/bash
cargo test --workspace --lib --quiet
if [ $? -ne 0 ]; then
    echo "Unit tests failed"
    exit 1
fi
```

---

## Best Practices

### 1. Test Naming
```rust
#[test]
fn test_<component>_<scenario>_<expected_outcome>()
// Examples:
fn test_router_complex_request_selects_premium_tier()
fn test_task_manager_rebuild_preserves_order()
```

### 2. Arrange-Act-Assert Pattern
```rust
#[test]
fn test_example() {
    // Arrange
    let manager = TaskManager::new();
    
    // Act
    manager.add_task(id, task);
    
    // Assert
    assert_eq!(manager.task_count(), 1);
}
```

### 3. Use Test Fixtures
```rust
fn setup_test_environment() -> (TaskManager, UiState) {
    // Common setup
}
```

### 4. Isolate Tests
- Use `tempfile::TempDir` for file operations
- Use `serial_test` for shared state
- Mock external dependencies

### 5. Fast Tests
- Unit tests: <10ms each
- Integration tests: <100ms
- E2E tests: <1s
- Use `--test-threads=1` only when necessary

### 6. Test Coverage Goals
- Critical paths: 100%
- Core logic: 80%+
- UI code: 60%+
- Overall: 70%+

---

## Running Tests

```bash
# All tests
cargo test --workspace

# Unit tests only (inline in src/)
cargo test --workspace --lib

# Integration tests (tests/ folder)
cargo test --workspace --test '*'

# Specific crate
cargo test -p merlin-routing

# Specific test file
cargo test --test task_manager_tests

# Specific test
cargo test test_task_order

# With output
cargo test -- --nocapture

# Parallel execution (default)
cargo test

# Serial execution
cargo test -- --test-threads=1
```

## Running Code Coverage

```bash
# Generate HTML coverage report
cargo llvm-cov --workspace --html --ignore-filename-regex "test_repositories|benchmarks" --release

# Generate and open in browser
cargo llvm-cov --workspace --html --open --ignore-filename-regex "test_repositories|benchmarks" --release

# Summary only
cargo llvm-cov --workspace --summary-only --ignore-filename-regex "test_repositories|benchmarks" --release

# Coverage for specific package
cargo llvm-cov -p merlin-routing --html --release

# With test output
cargo llvm-cov --workspace --html --release --ignore-filename-regex "test_repositories|benchmarks" -- --nocapture
```

**Note**: Coverage uses the `--release` flag because dev profile uses Cranelift backend which doesn't support coverage instrumentation. The release profile uses LLVM which supports `-Cinstrument-coverage`.

---

## Current Test Suite

### Implemented Tests

#### TUI Tests (`crates/merlin-routing/tests/`)
- `task_manager_tests.rs` - 12 tests covering:
  - Task addition and removal
  - Parent-child relationships
  - Collapse/expand functionality
  - Task ordering and hierarchy
  - Visibility with nested collapse

- `tui_rendering_tests.rs` - 7 tests covering:
  - Renderer creation and theme cycling
  - Rendering empty state
  - Rendering with tasks (running, completed, failed)
  - All pane focus states
  - Theme compatibility

- `common/mod.rs` - Test helpers:
  - `create_test_task()`
  - `create_child_task()`
  - `create_completed_task()`
  - `create_failed_task()`

#### CLI E2E Tests (`crates/merlin-cli/tests/`)
- `cli_e2e.rs` - 7 tests covering:
  - Help command
  - Invalid command handling
  - Empty directory handling
  - Rust project detection
  - Cargo.toml parsing
  - Path handling edge cases

#### Inline Unit Tests (in `src/` files)
- 74 tests across routing, analyzer, executor, validator
- Covering complexity detection, routing strategies, task graphs, validation pipeline

### Test Organization

```
crates/
  merlin-routing/
    src/
      user_interface/
        task_manager.rs    # Now public for testing
        renderer.rs        # Now public for testing
        state.rs          # Now public for testing
        theme.rs          # Now public for testing
    tests/
      common/mod.rs           # ✅ Shared test helpers
      task_manager_tests.rs   # ✅ TUI task management
      tui_rendering_tests.rs  # ✅ TUI rendering
      integration_tests.rs    # ⚠️ Placeholder (1 test)

  merlin-cli/
    tests/
      cli_e2e.rs             # ✅ CLI end-to-end tests

  merlin-agent/
    tests/
      tool_integration.rs    # ✅ Tool registry tests

  merlin-context/
    tests/
      bm25_tokenization.rs   # ✅ Search tests
      chunking_validation.rs # ✅ Chunking tests
```

### Gaps & Next Steps

#### High Priority (0% coverage)
1. **TUI User Input** - No tests for input handling, wrapping, multi-line
2. **TUI Persistence** - No tests for task save/load
3. **Theme Persistence** - No tests for theme save/load
4. **Event Handler** - No tests for UI event processing

#### Medium Priority (10-20% coverage)
1. **Tool Execution** - bash, edit, show tools need more tests
2. **Providers** - Mock provider tests, fallback logic
3. **Agent Reasoning** - More self-assessment tests

#### Low Priority (60%+ coverage but could improve)
1. **Validator Stages** - Edge cases in build/lint/test validation
2. **Router Strategies** - More cost/quality strategy tests

---

## Summary

This testing strategy provides:
- **Multi-layered coverage** from unit to E2E
- **Fast feedback loops** with quick unit tests
- **Regression prevention** through comprehensive scenarios
- **CI/CD integration** for automated validation
- **Clear organization** with dedicated test files

### Current State
- ✅ **26.61% code coverage** baseline established
- ✅ **93 total tests** passing (74 inline + 19 integration)
- ✅ **Coverage tooling** configured and working
- ✅ **Test infrastructure** in place for expansion
- ✅ **Public APIs** exposed for testing

### Key Benefits Achieved
- Catch regressions in routing logic
- Test TUI components reliably
- Verify CLI behavior
- Foundation for expanding coverage to 70%+
