# Fixture Coverage Report

**Overall Fixture Coverage: 25.57% lines (4105/16052), 8.66% functions (513/5926)**

**Last Updated:** 2025-10-29 05:04

This document tracks which files are covered by the fixture-based integration tests. The fixture system tests end-to-end user interactions by simulating complete CLI sessions.

**📊 See [COVERAGE_ANALYSIS.md](COVERAGE_ANALYSIS.md) for detailed analysis of why coverage is low and how to fix it.**

## Key Findings

- **Self-determination system (0% coverage)**: Never tested via fixtures. Needs assessment response fixtures.
- **CLI entry points (0% coverage)**: Fixtures bypass CLI layer by creating TuiApp directly.
- **Conversation history (0% coverage)**: No multi-turn fixtures exist.
- **Orchestrator (48.7%)**: Only streaming execution path tested, not batch/analysis paths.

## Files with Unexpectedly Low Coverage

### SHOULD COVER:
These files should be hit by fixtures but have low coverage. Priority items at top.

**Integration test infrastructure:**
- `crates/integration-tests/src/` (53.74%, 819/1524 lines)
- `crates/integration-tests/src/ui_verifier/` (58.59%, 365/623 lines)

**merlin-agent - Core execution:**
- `crates/merlin-agent/src/` (25.51%, 151/592 lines)
- `crates/merlin-agent/src/agent/` (2.39%, 7/293 lines)
- `crates/merlin-agent/src/agent/executor/` (29.62%, 322/1087 lines)
- `crates/merlin-agent/src/agent/task_coordinator/` (0%, 0/537 lines)
- `crates/merlin-agent/src/executor/` (1.94%, 18/927 lines)
- `crates/merlin-agent/src/validator/` (6.41%, 20/312 lines)
- `crates/merlin-agent/src/validator/stages/` (45.19%, 61/135 lines)

**merlin-cli - TUI and CLI:**
- `crates/merlin-cli/src/` (0%, 0/286 lines)
- `crates/merlin-cli/src/config/` (0%, 0/22 lines)
- `crates/merlin-cli/src/ui/` (45.77%, 314/686 lines)
- `crates/merlin-cli/src/ui/app/` (38.03%, 432/1136 lines)
- `crates/merlin-cli/src/ui/renderer/` (38.25%, 293/766 lines)

**merlin-context - Context management:**
- `crates/merlin-context/src/` (36.54%, 129/353 lines)
- `crates/merlin-context/src/builder/` (30.88%, 130/421 lines)
- `crates/merlin-context/src/embedding/` (5.68%, 18/317 lines)
- `crates/merlin-context/src/embedding/chunking/` (0%, 0/480 lines)
- `crates/merlin-context/src/embedding/chunking/rust/` (0%, 0/329 lines)
- `crates/merlin-context/src/embedding/vector_search/` (10.34%, 69/667 lines)
- `crates/merlin-context/src/embedding/vector_search/scoring/` (0%, 0/421 lines)
- `crates/merlin-context/src/query/` (62.69%, 84/134 lines)

**merlin-core - Core types:**
- `crates/merlin-core/src/` (20.47%, 79/386 lines)
- `crates/merlin-core/src/conversation/` (19.31%, 62/321 lines)
- `crates/merlin-core/src/prompts/` (23.81%, 15/63 lines)
- `crates/merlin-core/src/streaming/` (0%, 0/24 lines)
- `crates/merlin-core/src/task/` (36.73%, 18/49 lines)
- `crates/merlin-core/src/ui/` (43.10%, 25/58 lines)

**merlin-tooling - Tool system:**
- `crates/merlin-tooling/src/` (40.96%, 351/857 lines)
- `crates/merlin-tooling/src/runtime/` (75.98%, 291/383 lines)

**merlin-providers - Provider abstractions (mocked):**
- `crates/merlin-providers/src/` (0%, 0/337 lines)

**merlin-local - Local models:**
- `crates/merlin-local/src/` (0%, 0/119 lines)

**merlin-languages - Language servers:**
- `crates/merlin-languages/src/` (0%, 0/64 lines)

**merlin-routing - Routing & analysis:**
- `crates/merlin-routing/src/analyzer/` (3.57%, 11/308 lines)
- `crates/merlin-routing/src/cache/` (0%, 0/260 lines)
- `crates/merlin-routing/src/metrics/` (0%, 0/288 lines)
- `crates/merlin-routing/src/router/` (4.31%, 21/487 lines)

### SHOULDN'T COVER:
These are not user-facing and should remain in SHOULDN'T COVER:

**Test-only files:**
- Test helpers, mock implementations, test utilities

**Third-party integrations not yet used:**
- Language server backends (not connected)
- Local model providers (optional feature)

---

## Coverage Summary by Crate

For detailed per-file coverage, see the HTML report at `benchmarks/data/coverage/html/index.html`.

**Top Coverage (>50%):**
- `integration-tests` - 53.74% lines, 16.23% functions
- `ui_verifier` - 58.59% lines, 15.68% functions
- `merlin-tooling/runtime` - 75.98% lines, 16.67% functions

**Moderate Coverage (25-50%):**
- `merlin-agent/src` - 25.51% lines, 8.33% functions
- `merlin-agent/executor` - 29.62% lines, 10.48% functions
- `merlin-context/src` - 36.54% lines, 12.42% functions
- `merlin-cli/ui/app` - 38.03% lines, 11.63% functions
- `merlin-cli/ui/renderer` - 38.25% lines, 15.86% functions
- `merlin-tooling/src` - 40.96% lines, 15.17% functions

**Low Coverage (<25%):**
- `merlin-agent/agent` - 2.39% lines, 1.53% functions
- `merlin-agent/task_coordinator` - 0% lines, 0% functions
- `merlin-agent/executor (parallel)` - 1.94% lines, 0.96% functions
- `merlin-agent/validator` - 6.41% lines, 6.35% functions
- `merlin-cli/src` - 0% lines, 0% functions
- `merlin-core/src` - 20.47% lines, 6.11% functions

**Expected Low Coverage (internal systems):**
- `merlin-providers` - 0% (mocked in tests)
- `merlin-local` - 0% (not used in fixtures)
- `merlin-routing` - 3.57% avg (internal routing logic)
- `merlin-languages` - 0% (not yet integrated)

