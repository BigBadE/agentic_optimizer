# Merlin Test & Benchmark Infrastructure

**Test Coverage**: ~27% (baseline established)

---

## Overview

Merlin has three distinct testing/benchmarking systems:

### 1. **Automated Tests** (~165 tests)
Standard Rust test suite covering unit, integration, and E2E scenarios.

**Distribution**:
- **Unit Tests** (~76): Inline `#[cfg(test)]` modules in `src/` files
- **Integration Tests** (~86): `tests/` directories testing component interactions
- **E2E Tests** (~7): CLI behavior validation in `merlin-cli/tests/`

**Coverage by Area**:
- ✅ **Routing & Executor** (60-90%): Well-tested core logic
- ⚠️ **Validation Pipeline** (~40%): Moderate coverage
- ⚠️ **Context System** (~30%): Needs expansion
- ❌ **TUI Input/Persistence** (0%): Critical gap
- ❌ **Event Handling** (0%): Critical gap
- ❌ **Tool Execution** (10-20%): Minimal coverage

### 2. **Performance Benchmarks** (Criterion.rs)
**Location**: `crates/merlin-routing/benches/routing_benchmarks.rs`

Measures execution speed using industry-standard Criterion.rs:
- Request analysis (simple/medium/complex)
- Task decomposition
- Complexity analysis
- Task graph construction

**CI Integration**: ✅ Automated via GitHub Actions, tracks regressions >15%

### 3. **Context Quality Benchmarks** (Custom)
**Location**: `benchmarks/`

Measures search accuracy and relevance (not speed):
- 20 test cases on real codebase (Valor browser)
- Metrics: Precision@3/10, Recall@10, MRR, NDCG@10
- Current: 30% P@3, 49% R@10 (target: 60% P@3, 70% R@10)

**CI Integration**: ❌ Manual only (needs automation)

---

## Test Organization

```
crates/
├── merlin-routing/
│   ├── benches/
│   │   └── routing_benchmarks.rs       # Performance benchmarks (Criterion)
│   ├── src/                            # Unit tests inline
│   │   ├── analyzer/                   # ~18 tests (complexity, intent, decompose)
│   │   ├── executor/                   # ~12 tests (graph, locks, transactions)
│   │   ├── router/                     # ~16 tests (strategies, tiers)
│   │   ├── validator/                  # ~11 tests (pipeline, stages)
│   │   ├── tools/                      # ~7 tests (command, file ops)
│   │   └── user_interface/             # ~7 tests (text width)
│   └── tests/                          # Integration tests
│       ├── executor_tests.rs           # 22 tests (graph, locks, workspace)
│       ├── validator_tests.rs          # 19 tests (pipeline, stages)
│       ├── output_tree_tests.rs        # 24 tests (tree structure, navigation)
│       ├── task_manager_tests.rs       # 12 tests (task operations, hierarchy)
│       ├── tui_rendering_tests.rs      # 8 tests (renderer, themes)
│       └── integration_tests.rs        # 1 test (orchestrator)
│
├── merlin-cli/
│   └── tests/
│       └── cli_e2e.rs                  # 7 E2E tests
│
├── merlin-context/
│   ├── src/query/analyzer.rs           # 5 inline tests
│   └── tests/
│       ├── bm25_tokenization.rs        # 4 tests
│       └── chunking_validation.rs      # 4 tests
│
├── merlin-agent/
│   └── tests/
│       └── tool_integration.rs         # 1 test
│
└── merlin-{providers,local}/
    └── src/                            # ~5 inline tests

benchmarks/                              # Context quality benchmarks (separate system)
├── test_cases/valor/                   # 20 test case definitions (TOML)
└── test_repositories/valor/            # Real codebase for testing
```

---

## Test Organization

```
crates/
├── merlin-agent/
│   └── tests/
│       └── tool_integration.rs          (1 test)
│
├── merlin-cli/
│   └── tests/
│       └── cli_e2e.rs                   (7 tests)
│
├── merlin-context/
│   ├── src/
│   │   └── query/analyzer.rs           (5 inline tests)
│   └── tests/
│       ├── bm25_tokenization.rs        (4 tests)
│       └── chunking_validation.rs      (4 tests)
│
├── merlin-local/
│   └── src/
│       ├── inference.rs                (2 inline tests)
│       └── manager.rs                  (1 inline test)
│
├── merlin-providers/
│   └── src/
│       └── groq.rs                     (2 inline tests)
│
└── merlin-routing/
    ├── benches/
    │   └── routing_benchmarks.rs       (4 benchmarks)
    ├── src/
    │   ├── agent/
    │   │   ├── executor.rs             (1 inline test)
    │   │   └── self_assess.rs          (1 inline test)
    │   ├── analyzer/
    │   │   ├── complexity.rs           (4 inline tests)
    │   │   ├── decompose.rs            (4 inline tests)
    │   │   ├── intent.rs               (6 inline tests)
    │   │   └── local.rs                (4 inline tests)
    │   ├── config.rs                   (2 inline tests)
    │   ├── executor/
    │   │   ├── graph.rs                (2 inline tests)
    │   │   ├── isolation.rs            (3 inline tests)
    │   │   ├── pool.rs                 (1 inline test)
    │   │   ├── scheduler.rs            (2 inline tests)
    │   │   ├── state.rs                (2 inline tests)
    │   │   └── transaction.rs          (2 inline tests)
    │   ├── orchestrator.rs             (3 inline tests)
    │   ├── router/
    │   │   ├── strategies/
    │   │   │   ├── complexity.rs       (4 inline tests)
    │   │   │   ├── context.rs          (3 inline tests)
    │   │   │   ├── cost.rs             (2 inline tests)
    │   │   │   └── quality.rs          (3 inline tests)
    │   │   └── tiers.rs                (4 inline tests)
    │   ├── tools/
    │   │   ├── command.rs              (3 inline tests)
    │   │   └── file_ops.rs             (4 inline tests)
    │   ├── user_interface/
    │   │   └── text_width.rs           (7 inline tests)
    │   └── validator/
    │       ├── pipeline.rs             (2 inline tests)
    │       └── stages/
    │           ├── build.rs            (2 inline tests)
    │           ├── lint.rs             (2 inline tests)
    │           ├── syntax.rs           (3 inline tests)
    │           └── test.rs             (2 inline tests)
    └── tests/
        ├── common/mod.rs               (test helpers)
        ├── executor_tests.rs           (22 tests)
        ├── integration_tests.rs        (1 test)
        ├── output_tree_tests.rs        (24 tests)
        ├── task_manager_tests.rs       (12 tests)
        ├── tui_rendering_tests.rs      (8 tests)
        └── validator_tests.rs          (19 tests)
```

---

## Running Tests

### All Tests
```bash
cargo test --workspace
```

### Unit Tests Only
```bash
cargo test --workspace --lib
```

### Integration Tests Only
```bash
cargo test --workspace --test '*'
```

### Specific Crate
```bash
cargo test -p merlin-routing
cargo test -p merlin-cli
cargo test -p merlin-context
```

### Specific Test File
```bash
cargo test --test executor_tests
cargo test --test validator_tests
cargo test --test cli_e2e
```

### Specific Test
```bash
cargo test test_task_graph_creation
cargo test test_complexity_routing
```

### With Output
```bash
cargo test -- --nocapture
```

### Benchmarks
```bash
# Run all benchmarks
cargo bench --workspace

# Specific benchmark
cargo bench --bench routing_benchmarks

# With profiling
cargo bench --profile release
```

---

## Coverage Analysis

### Generate Coverage Report

```bash
# HTML report
cargo llvm-cov --workspace --html --ignore-filename-regex "test_repositories|benchmarks" --release

# Open in browser
cargo llvm-cov --workspace --html --open --ignore-filename-regex "test_repositories|benchmarks" --release

# Summary only
cargo llvm-cov --workspace --summary-only --ignore-filename-regex "test_repositories|benchmarks" --release

# Specific package
cargo llvm-cov -p merlin-routing --html --release

# With test output
cargo llvm-cov --workspace --html --release --ignore-filename-regex "test_repositories|benchmarks" -- --nocapture
```

**Note**: Coverage uses `--release` profile because dev profile uses Cranelift backend which doesn't support coverage instrumentation.

### Current Coverage: 26.61%

#### Coverage by Component
- **Routing Logic** (analyzer, router, executor): 60-90% ✅
- **Validation Pipeline**: ~40% ⚠️
- **Tools** (bash, edit, show): 10-20% ❌
- **TUI Components** (app, renderer, task_manager): Varies ⚠️
- **Providers & Core**: 20-40% ⚠️
- **Context System**: ~30% ⚠️

---

## Gaps & Improvements Needed

### High Priority - Missing Test Coverage

#### 1. TUI User Input (0% coverage)
**What's Missing**:
- Input handling and validation
- Text wrapping in input fields
- Multi-line input support
- Cursor movement and editing
- Input history navigation

**Why Critical**: User input is a core interaction point; bugs here directly impact UX.

**Recommended Tests**:
```rust
// crates/merlin-routing/tests/tui_input_tests.rs
#[test]
fn test_input_wrapping()
#[test]
fn test_multiline_input()
#[test]
fn test_cursor_movement()
#[test]
fn test_input_history()
```

#### 2. TUI Persistence (0% coverage)
**What's Missing**:
- Task save/load functionality
- State persistence across restarts
- Corrupted state handling
- Migration between versions

**Why Critical**: Data loss would severely impact user trust.

**Recommended Tests**:
```rust
// crates/merlin-routing/tests/tui_persistence_tests.rs
#[test]
fn test_save_task_state()
#[test]
fn test_load_task_state()
#[test]
fn test_corrupted_state_recovery()
#[test]
fn test_state_migration()
```

#### 3. Theme Persistence (0% coverage)
**What's Missing**:
- Theme save/load
- Theme configuration validation
- Custom theme support
- Theme migration

**Why Critical**: User preferences should persist reliably.

**Recommended Tests**:
```rust
// crates/merlin-routing/tests/theme_persistence_tests.rs
#[test]
fn test_save_theme_preference()
#[test]
fn test_load_theme_preference()
#[test]
fn test_invalid_theme_fallback()
```

#### 4. Event Handler (0% coverage)
**What's Missing**:
- UI event processing
- Keyboard shortcut handling
- Mouse event handling
- Event queue management

**Why Critical**: Event handling bugs can freeze or crash the UI.

**Recommended Tests**:
```rust
// crates/merlin-routing/tests/event_handler_tests.rs
#[test]
fn test_keyboard_shortcuts()
#[test]
fn test_mouse_events()
#[test]
fn test_event_queue()
#[test]
fn test_concurrent_events()
```

### Medium Priority - Low Coverage

#### 5. Tool Execution (10-20% coverage)
**Current State**: Basic tests exist but edge cases missing.

**What's Missing**:
- Tool error handling
- Tool timeout behavior
- Tool output parsing
- Tool chaining
- Tool parameter validation

**Recommended Tests**:
```rust
// crates/merlin-agent/tests/tool_execution_tests.rs
#[test]
fn test_tool_timeout()
#[test]
fn test_tool_error_recovery()
#[test]
fn test_tool_output_parsing()
#[test]
fn test_tool_chaining()
#[test]
fn test_invalid_parameters()
```

#### 6. Provider System (20-40% coverage)
**Current State**: Basic provider tests, missing fallback logic.

**What's Missing**:
- Provider fallback chains
- Rate limiting behavior
- Provider health checks
- Cost tracking
- Token usage tracking
- Provider-specific error handling

**Recommended Tests**:
```rust
// crates/merlin-providers/tests/provider_integration_tests.rs
#[test]
fn test_provider_fallback()
#[test]
fn test_rate_limiting()
#[test]
fn test_health_check()
#[test]
fn test_cost_tracking()
#[test]
fn test_provider_errors()
```

#### 7. Agent Reasoning (minimal coverage)
**Current State**: 1 test for self-assessment.

**What's Missing**:
- Multi-step reasoning
- Context accumulation
- Tool selection logic
- Error recovery strategies
- Reasoning chain validation

**Recommended Tests**:
```rust
// crates/merlin-agent/tests/agent_reasoning_tests.rs
#[test]
fn test_multi_step_reasoning()
#[test]
fn test_context_accumulation()
#[test]
fn test_tool_selection()
#[test]
fn test_error_recovery()
#[test]
fn test_reasoning_chain()
```

#### 8. Context System (30% coverage)
**Current State**: BM25 and chunking tests exist.

**What's Missing**:
- Context window management
- Context prioritization
- Context compression
- Semantic search accuracy
- Context relevance scoring

**Recommended Tests**:
```rust
// crates/merlin-context/tests/context_management_tests.rs
#[test]
fn test_context_window_limits()
#[test]
fn test_context_prioritization()
#[test]
fn test_context_compression()
#[test]
fn test_semantic_search()
#[test]
fn test_relevance_scoring()
```

### Low Priority - Good Coverage but Could Improve

#### 9. Validator Stages (60%+ coverage)
**Current State**: Good pipeline tests, some edge cases missing.

**What Could Improve**:
- More edge cases in build validation
- Lint rule customization tests
- Test framework integration tests

#### 10. Router Strategies (60%+ coverage)
**Current State**: Good strategy tests.

**What Could Improve**:
- More cost/quality trade-off scenarios
- Strategy conflict resolution
- Dynamic strategy adjustment

---

## Testing Best Practices

### 1. Test Naming Convention
```rust
#[test]
fn test_<component>_<scenario>_<expected_outcome>()

// Examples:
fn test_router_complex_request_selects_premium_tier()
fn test_task_manager_rebuild_preserves_order()
fn test_validator_syntax_error_fails_validation()
```

### 2. Arrange-Act-Assert Pattern
```rust
#[test]
fn test_example() {
    // Arrange: Set up test data
    let manager = TaskManager::new();
    let task = create_test_task();
    
    // Act: Perform the action
    manager.add_task(task_id, task);
    
    // Assert: Verify the outcome
    assert_eq!(manager.task_count(), 1);
}
```

### 3. Use Test Fixtures
```rust
// tests/common/mod.rs
pub fn create_test_task() -> TaskDisplay { ... }
pub fn create_temp_project() -> TempDir { ... }
pub fn create_mock_provider() -> MockProvider { ... }
```

### 4. Isolate Tests
- Use `tempfile::TempDir` for file operations
- Use `serial_test` for shared state tests
- Mock external dependencies
- Avoid test interdependencies

### 5. Performance Targets
- Unit tests: <10ms each
- Integration tests: <100ms each
- E2E tests: <1s each
- Use `--test-threads=1` only when necessary

### 6. Coverage Goals
- **Critical paths**: 100%
- **Core logic**: 80%+
- **UI code**: 60%+
- **Overall**: 70%+ (current: 26.61%)

---

## CI/CD Integration

### GitHub Actions Workflows

#### Test Workflow (`.github/workflows/test.yml`)
- Runs on: Ubuntu, Windows, macOS
- Triggers: Push to master, PRs, daily schedule
- Coverage: Generates and uploads to Codecov
- Profile: `ci-release` for faster builds

#### Benchmark Workflow (`.github/workflows/benchmark.yml`)
- Runs on: Push to master, PRs
- Stores results to gh-pages branch
- Alert threshold: 15% regression
- Retention: 90 days

#### Style Workflow (`.github/workflows/style.yml`)
- Runs clippy on all targets
- Enforces strict lints (see `Cargo.toml`)

---

## Test Dependencies

### Workspace-level Dependencies
```toml
criterion = "0.7"          # Benchmarking
assert_cmd = "2.0"         # CLI testing
predicates = "3.1"         # Output assertions
tempfile = "3.23"          # Temporary directories
serial_test = "3.2"        # Serialize tests
insta = "1.43"             # Snapshot testing
tokio-test = "0.4"         # Async test utilities
```

---

## Summary

### Current State ✅
- **164 tests** covering core functionality
- **4 benchmarks** for performance tracking
- **26.61% code coverage** baseline established
- **Multi-platform CI** (Ubuntu, Windows, macOS)
- **Automated coverage reporting** to Codecov

### Strengths 💪
- Excellent routing and executor test coverage (60-90%)
- Comprehensive TUI component tests (task manager, renderer, output tree)
- Good validation pipeline tests
- Strong CLI E2E test suite
- Well-organized test structure with shared helpers

### Critical Gaps 🚨
1. **TUI User Input** - No tests (0% coverage)
2. **TUI/Theme Persistence** - No tests (0% coverage)
3. **Event Handler** - No tests (0% coverage)
4. **Tool Execution** - Minimal tests (10-20% coverage)
5. **Provider System** - Low coverage (20-40%)
6. **Agent Reasoning** - Minimal tests
7. **Context System** - Moderate coverage (30%)

### Recommended Next Steps 📋
1. **Immediate**: Add TUI input and persistence tests (high user impact)
2. **Short-term**: Expand tool execution and provider tests (stability)
3. **Medium-term**: Improve agent reasoning and context tests (quality)
4. **Long-term**: Increase overall coverage to 70%+ (confidence)

### Coverage Target Roadmap 🎯
- **Phase 1** (Current → 40%): TUI input, persistence, event handling
- **Phase 2** (40% → 55%): Tool execution, provider fallbacks
- **Phase 3** (55% → 70%): Agent reasoning, context management
- **Phase 4** (70%+): Edge cases, stress tests, property-based tests
