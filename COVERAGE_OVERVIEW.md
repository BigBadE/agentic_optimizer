# Coverage Overview

**Last Updated:** 2025-10-31
**Coverage Type:** Fixture tests only
**Command:** `./scripts/verify.sh --fixture --cov`
**Report:** `benchmarks/data/coverage/html/index.html`

## Summary Metrics

**Current Coverage:**
- **Lines:** 46.46% (5638 / 12136)
- **Functions:** 16.05% (634 / 3950)
- **Total Fixtures:** 107

**Recent Change:**
- **Lines:** 42.23% → 46.46% (+4.23pp, +685 lines)
- **Functions:** 22.26% → 16.05% (denominator increased from 2493 → 3950)

**Key Insight:** Fixture tests provide significantly higher quality coverage than unit tests because they exercise real end-to-end user paths.

## Coverage by Module

### Excellent Coverage (>70%)

| Module | Lines | Functions | Notes |
|--------|-------|-----------|-------|
| `merlin-tooling/src/runtime` | 86.98% (294/338) | 56.45% (35/62) | TypeScript agent execution |
| `merlin-cli/src/ui/renderer` | 76.14% (549/721) | 77.08% (37/48) | UI rendering pipeline |
| `merlin-tooling/src` | 73.13% (351/480) | 48.65% (54/111) | File operations, tools |

### Good Coverage (50-70%)

| Module | Lines | Functions | Notes |
|--------|-------|-----------|-------|
| `merlin-context/src/embedding/chunking/config.rs` | 68.57% | Unknown | Code chunking config (was 0%) |
| `merlin-agent/src/agent/executor` | 69.42% (495/713) | 42.86% (48/112) | Agent execution |
| `merlin-cli/src/ui/app` | 63.49% (772/1216) | 34.27% (73/213) | TUI application |
| `merlin-cli/src/ui` | 61.8% (343/555) | 61.64% (45/73) | UI components |
| `integration-tests/src` | 60.95% (857/1406) | 13.76% (89/647) | Test infrastructure |
| `merlin-context/src/embedding` | 60.88% (179/294) | 50% (25/50) | Embedding system |
| `merlin-context/src` | 57.85% (129/223) | 51.35% (19/37) | Context management |
| `integration-tests/src/ui_verifier` | 56.06% (361/644) | 15.42% (37/240) | UI verification |
| `merlin-agent/src/agent/executor` | 54.21% (483/891) | 25.51% (50/196) | Executor internals |

### Moderate Coverage (30-50%)

| Module | Lines | Functions | Notes |
|--------|-------|-----------|-------|
| `merlin-agent/src` | 48.12% (205/426) | 32.1% (26/81) | Agent core |
| `merlin-core/src/ui` | 47.83% (33/69) | 60% (6/10) | Core UI types |
| `merlin-agent/src/validator/stages` | 45.19% (61/135) | 31.03% (9/29) | Validation stages |
| `merlin-core/src` | 42.25% (79/187) | 27.5% (11/40) | Core types |
| `merlin-agent/src` | 39.96% (195/488) | 16.88% (27/160) | Agent modules |
| `merlin-core/src/conversation` | 38.5% (87/226) | 33.33% (15/45) | Conversation system |
| `merlin-agent/src/agent` | 36.84% (7/19) | 40% (2/5) | Agent interface |
| `merlin-core/src/task` | 36.73% (18/49) | 30% (3/10) | Task types |
| `merlin-cli/src/config` | 32.89% (25/76) | 30% (6/20) | Configuration |
| `merlin-context/src/builder` | 32.07% (135/421) | 36.73% (18/49) | Context builder |

### Low Coverage - Intentionally Mocked (<10%)

These modules are intentionally bypassed in fixtures for deterministic testing:

| Module | Lines | Functions | Reason |
|--------|-------|-----------|--------|
| `merlin-routing/src/router` | 7% (21/300) | 8% (4/50) | MockRouter used |
| `merlin-routing/src/analyzer` | 6.25% (11/176) | 10.53% (2/19) | Mocked for determinism |
| `merlin-cli/src` | 0% (0/12) | 0% (0/6) | CLI entry bypassed |
| `merlin-local/src` | 0% (0/100) | 0% (0/17) | Ollama not used |
| `merlin-providers/src` | 0% (0/125) | 0% (0/33) | External APIs mocked |
| `merlin-routing/src/cache` | 0% (0/98) | 0% (0/16) | Not critical for e2e |
| `merlin-routing/src/metrics` | 0% (0/12) | 0% (0/3) | Not critical for e2e |

### Low Coverage - ACTIONABLE GAPS (<30%)

Critical modules that need more fixture coverage:

| Module | Lines | Functions | Priority | Gap Analysis |
|--------|-------|-----------|----------|--------------|
| **merlin-agent/src/executor** | 5.44% (18/331) | 2.27% (3/132) | 🔴 HIGH | Task decomposition, parallel execution, scheduler |
| **merlin-agent/src/validator** | 10.39% (16/154) | 7.5% (3/40) | 🔴 HIGH | Validation pipeline, citation checks, syntax validation |
| **merlin-context/src/embedding/vector_search** | 26.84% (179/667) | 24.24% (24/99) | 🟡 MEDIUM | Search logic, query processing |
| **merlin-context/src/embedding/chunking** | Unknown | Unknown | 🟢 IMPROVED | Was 0%, now 68.57% (config.rs) |
| **merlin-context/src/embedding/vector_search/scoring** | 0% (0/421) | 0% (0/42) | 🟡 MEDIUM | Relevance scoring, RRF fusion, BM25 |

## Gap Analysis

### Why Scoring Still at 0%

**Chunking Fixed** (0% → 68.57%): Embedding generation now runs synchronously during fixture setup.

**Scoring Still 0%**: Scoring happens during semantic search queries, not embedding generation.

**Code Path**: `ContextBuilder::build()` → `vector_manager.search()` → `score_results()`

**Missing**: Fixtures that make context requests triggering semantic search with actual query matching.

### Why Executor Still at 5.44%

**Scheduler/Decomposition Not Triggered**: Fixtures return simple TypeScript string responses, not TaskList.

**Code Path**: `ExecutorPool` → `schedule_steps()` → parallel execution → conflict detection

**Missing**: Fixtures with LLM responses returning TaskList with:
- Multiple sequential steps
- Parallel steps (no dependencies)
- Exit requirements (file_exists, file_contains, pattern matching)
- Retry logic (soft/hard errors)
- Nested decomposition (TaskList returning TaskList)

### Why Validator Still at 10.39%

**Validation Stages Not Triggered**: Fixtures use mock responses that succeed; validation never fails.

**Code Path**: `ValidationPipeline` → `run_stages()` → citation/syntax/build validators

**Missing**: Fixtures that:
- Return code with missing file references (citation validator)
- Return code with syntax errors (syntax validator)
- Trigger multi-stage validation cascade
- Test early exit on critical failures

## Coverage by Component Category

| Category | Average Coverage | Assessment |
|----------|------------------|------------|
| **User-Facing** (TUI, rendering, input) | 65-75% | ✅ Excellent |
| **Tools & Runtime** (file ops, bash, TS) | 70-85% | ✅ Excellent |
| **Agent Execution** (executor, validation) | 35-45% | ⚠️ Needs improvement |
| **Context/Embedding** (chunking, search) | 40-50% | ⚠️ Mixed (chunking fixed, scoring gap) |
| **Context/Scoring** (semantic search) | 0-10% | ❌ Major gap |
| **Routing/Providers** (mocked) | 0-7% | ✅ Intentional |
| **Test Infrastructure** | 55-65% | ✅ Good |

## Realistic Coverage Goals

### Current State
- **Fixture coverage:** 46.46% lines, 16.05% functions
- **Untestable code:** ~12-17% (mocked routing, CLI, OS errors)
- **Actionable gaps:** ~10-15% (executor, validation, scoring)

### Short-Term Goal (50-55%)
**Target Additions:**
- TaskList decomposition fixtures (+300-400 lines executor coverage)
- Validation failure fixtures (+100-150 lines validator coverage)
- **Expected Total:** 50-52% lines

### Medium-Term Goal (55-60%)
**Target Additions:**
- Semantic search query fixtures (+300-400 lines scoring coverage)
- Context builder fixtures (+100-150 lines context coverage)
- **Expected Total:** 55-58% lines

### Long-Term Maximum (60-65%)
**Realistic Ceiling:**
- 12-17% of code is untestable by fixtures (mocked/CLI/OS-level/platform-specific)
- ~800-1000 lines routing/providers/metrics (intentionally mocked)
- ~200-300 lines CLI entry points (fixtures bypass CLI)
- ~300-500 lines OS-level error paths
- ~100-200 lines platform-specific code

**Note:** >65% coverage unrealistic given architectural constraints.

## What NOT to Cover with Fixtures

❌ **Skip these (not reachable by fixtures):**
- CLI entry points (`cli.rs`, `handlers.rs`)
- Production constructors needing real API keys
- OS-level error paths (permission denied, disk full)
- Platform-specific branches (Windows vs Unix)
- Routing internals (intentionally mocked)
- External provider implementations (intentionally mocked)
- Metrics/cache internals (not critical for e2e)

## What TO Cover with Fixtures

✅ **Focus on:**
- Agent executor task decomposition (TaskList responses)
- Validation pipeline stages (failures, warnings, errors)
- Context selection and semantic search (queries)
- User-facing thread operations
- Error display in TUI
- Task tree interactions
- File operation workflows
- Multi-turn conversations

## Key Findings

1. **Fixtures Exercise Real Paths**: 46.46% fixture coverage >> 27.34% all-tests coverage
2. **User-Facing Code Well-Tested**: TUI rendering/runtime 76-87%, application logic 61-63%
3. **Intentional Mocking Effective**: Routing/providers 0-7% by design (tested separately)
4. **Major Gaps Identified**: Executor (5.44%), validator (10.39%), scoring (0%)
5. **Chunking Fixed**: Was 0%, now 68.57% after embedding generation improvement
6. **Scoring Needs Queries**: Embeddings generated but no semantic search triggered
7. **Executor Needs TaskLists**: No decomposition fixtures exist yet
8. **Validator Needs Failures**: All fixtures succeed, no validation triggered

## Recent Improvements

**2025-10-31**: Embedding generation in fixture setup
- **Change**: Modified test runner to generate embeddings synchronously during setup
- **Impact**: +4.23pp coverage (42.23% → 46.46%), +685 lines
- **Modules Fixed**: Chunking 0% → 68.57%

## Conclusion

**46.46% fixture coverage is excellent** given:
1. ✅ 12-17% of code intentionally untestable by fixtures
2. ✅ User-facing code has 65-85% coverage
3. ✅ Fixtures focus on real behavior, not mocked internals

**Key gaps are fixable:**
1. ⚠️ Agent task decomposition (5.44%) - needs TaskList fixtures
2. ⚠️ Validation pipeline (10.39%) - needs failure fixtures
3. ⚠️ Semantic search scoring (0%) - needs query fixtures

**Next target:** 50-55% coverage with TaskList and validation fixtures.
