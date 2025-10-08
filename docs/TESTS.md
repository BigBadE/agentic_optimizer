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

## Critical Gaps & Priorities

### High Priority (0% Coverage) 🚨
1. **TUI User Input**: Input handling, wrapping, cursor movement, history
2. **TUI/Theme Persistence**: Save/load, corruption recovery, migration
3. **Event Handling**: Keyboard/mouse events, queue management

### Medium Priority (10-40% Coverage) ⚠️
4. **Tool Execution**: Error handling, timeouts, chaining, parameter validation
5. **Provider System**: Fallback chains, rate limiting, health checks, cost tracking
6. **Agent Reasoning**: Multi-step reasoning, context accumulation, tool selection
7. **Context System**: Window management, prioritization, compression, relevance scoring

### Low Priority (60%+ Coverage) ✅
8. **Validator Stages**: Edge cases, lint customization, test framework integration
9. **Router Strategies**: Cost/quality trade-offs, conflict resolution

---

## Benchmark System Improvements

### Implemented ✅
- **Performance Benchmarks** (Criterion.rs): 8 benchmarks tracking execution speed
  - Request analysis, task decomposition, complexity analysis, task graph
  - Validation pipeline, task scheduling, file locking, tier selection
  - CI integration with GitHub Actions
  - Automatic regression detection (>15% threshold)
  - Historical tracking on gh-pages

- **Quality Benchmarks** (Custom): Automated CI tracking
  - Binary: `cargo run --bin quality-bench`
  - Metrics: Precision@3/10, Recall@10, MRR, NDCG@10, Critical@3
  - GitHub Actions workflow for automated runs
  - Results stored in gh-pages alongside performance benchmarks
  - PR comments with quality metrics

### In Progress 🔄

#### 1. Context Quality Benchmarks - Integration
**Status**: ✅ Binary created, ⚠️ Needs integration with actual context system  
**Goal**: Connect benchmark binary to real context retrieval

**Tasks**:
- [x] Create `benchmarks/src/main.rs` binary for running quality benchmarks
- [x] Add `[[bin]]` section to workspace `Cargo.toml`
- [x] Create GitHub Actions workflow for quality benchmarks
- [x] Implement metrics calculation (P@3, R@10, MRR, NDCG)
- [ ] Integrate with merlin-context search system
- [ ] Generate JSON output for benchmark-action
- [ ] Test on actual Valor repository

#### 2. Expand Performance Benchmarks
**Status**: ✅ Expanded to 8 benchmarks (target: 15-20)  
**Goal**: Add remaining component benchmarks

**Completed**:
- [x] Add validation pipeline benchmarks
- [x] Add task scheduling benchmarks
- [x] Add file locking benchmarks
- [x] Add tier selection benchmarks

**Remaining Tasks**:
- [ ] Add tool execution benchmarks (bash, edit, show)
- [ ] Add TUI rendering benchmarks
- [ ] Add context retrieval benchmarks
- [ ] Add end-to-end request benchmarks

**Progress**: 8/20 benchmarks (40%)

#### 3. Integration Benchmarks (New Category)
**Status**: ❌ Missing  
**Goal**: Track end-to-end performance and resource usage

**Tasks**:
- [ ] Create `crates/merlin-routing/benches/integration_benchmarks.rs`
- [ ] Add full request → response time benchmarks
- [ ] Add memory usage tracking benchmarks
- [ ] Add concurrency benchmarks (multi-task execution)
- [ ] Add stress test benchmarks (high load scenarios)

#### 4. Quality Benchmark Expansion
**Status**: ⚠️ Only Valor browser tested  
**Goal**: Test on diverse codebases

**Tasks**:
- [ ] Add test repository: Rust compiler subset
- [ ] Add test repository: Web framework (e.g., Axum)
- [ ] Add test repository: CLI tool (e.g., ripgrep)
- [ ] Create 20 test cases per repository
- [ ] Track metrics across all repositories

**Target**: 60+ test cases across 3+ diverse codebases

---

## Implementation Tracking

### Phase 1: Benchmark Automation (Week 1-2) ✅ COMPLETED
- [x] Create benchmark binary in `benchmarks/`
- [x] Add GitHub Actions workflow for quality benchmarks
- [x] Integrate with gh-pages for historical tracking
- [x] Add automated comparison reports
- ⚠️ **Note**: Binary created but needs integration with actual context system

### Phase 2: Expand Performance Benchmarks (Week 3-4) ✅ PARTIALLY COMPLETED
- [x] Add validation pipeline benchmarks
- [x] Add task scheduling benchmarks
- [x] Add file locking benchmarks
- [x] Add tier selection benchmarks
- [ ] Add tool execution benchmarks (remaining)
- [ ] Add TUI rendering benchmarks (remaining)
- [ ] Add context retrieval benchmarks (remaining)

### Phase 3: Integration Benchmarks (Week 5-6)
- [ ] Create integration benchmark suite
- [ ] Add memory usage tracking
- [ ] Add concurrency benchmarks
- [ ] Add stress test benchmarks

### Phase 4: Quality Benchmark Expansion (Week 7-8)
- [ ] Add 2 new test repositories
- [ ] Create 40 new test cases
- [ ] Validate metrics across diverse codebases
- [ ] Document findings and improvements

---

## CI/CD Integration

### Existing Workflows ✅
- **Test Workflow**: Multi-platform (Ubuntu, Windows, macOS), coverage to Codecov
- **Performance Benchmarks**: Automated, gh-pages tracking, 15% regression alerts
- **Style Workflow**: Clippy enforcement

### New Workflows (Planned) 🔄
- **Quality Benchmarks**: Track search accuracy over time
- **Integration Benchmarks**: Track end-to-end performance and memory usage

---

## Summary

### Strengths ✅
- Solid routing/executor test coverage (60-90%)
- Well-organized test structure with shared helpers
- Automated performance benchmarking with CI integration
- Comprehensive quality benchmark system (manual)

### Critical Actions Required 🚨
1. **Automate quality benchmarks** - Move from manual to CI-integrated
2. **Add TUI tests** - 0% coverage on critical user-facing components
3. **Expand performance benchmarks** - Cover more components beyond routing
4. **Create integration benchmarks** - Track end-to-end performance and resources

### Success Metrics 🎯
- **Test Coverage**: 27% → 70%+ (18-24 months)
- **Performance Benchmarks**: 4 → 8 → 20+ benchmarks (40% complete)
- **Quality Benchmarks**: Manual → ✅ Automated CI tracking (needs integration)
- **Integration Benchmarks**: 0 → 10+ end-to-end scenarios (planned)
