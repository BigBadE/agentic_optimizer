# Fixture Coverage Analysis & Action Plan

## Executive Summary

Current fixture coverage is **25.57% lines (4105/16052), 8.66% functions (513/5926)**. This analysis identifies why critical paths have low coverage and provides a detailed plan to reach >70% coverage for all SHOULD COVER files.

**Last Updated:** 2025-10-29 05:04

## Root Cause Analysis

### Why Self-Determination Has 0% Coverage

**Files affected:**
- `agent/self_assess.rs` (0.0%)
- `agent/executor/self_determining.rs` (14.2%)
- `agent/conversation.rs` (0.0%)
- `agent/executor/mod.rs` (38.4%) - missing decomposition path

**Root cause:** The self-determination system is **never triggered** by existing fixtures.

**How self-determination works:**
1. Task comes in → `execute_self_determining()` checks if it's "simple" via `QueryIntent::is_simple()`
2. Simple tasks (<=3 words, greetings, "hi", "hello") → skip assessment, execute directly
3. Complex tasks (code modifications, queries) → enter self-determination loop:
   - Call `SelfAssessor::assess_task()`
   - Get back JSON with action: "COMPLETE", "DECOMPOSE", or "GATHER"
   - Execute based on action

**Why it's not covered:**
- Existing fixtures use requests classified as `CodeQuery` or `CodeModification`, which SHOULD trigger assessment
- BUT: The mock provider setup may not be configured to return assessment-format JSON responses
- Fixtures never include LLM responses matching the assessment prompt pattern

**Example of what's needed:**
```json
{
  "type": "llm_response",
  "trigger": {
    "pattern": "task_assessment",  // Must match the prompt key
    "match_type": "contains"
  },
  "response": {
    "typescript": [
      "async function agent_code(): Promise<string> {",
      "  return JSON.stringify({",
      "    action: 'DECOMPOSE',",  // Uppercase required
      "    reasoning: 'Task requires multiple steps',",
      "    strategy: 'sequential',",
      "    subtasks: [",
      "      { description: 'Step 1', difficulty: 3 },",
      "      { description: 'Step 2', difficulty: 2 }",
      "    ]",
      "  });",
      "}"
    ]
  }
}
```

### Why Orchestrator Has Only 48.7% Coverage

**Uncovered sections:**
1. **Lines 47-71**: `RoutingOrchestrator::new()` - Production constructor
   - **Why**: Fixtures use `new_with_router()` for testing with mock providers
   - **Fix**: Not needed - this is the real constructor that creates real providers

2. **Lines 110-127**: Builder methods (`with_analyzer`, `with_router`, `with_validator`)
   - **Why**: Not used in test setup
   - **Fix**: Not critical - these are optional configuration methods

3. **Lines 332-362**: `execute_tasks()` - Parallel task execution with conflict detection
   - **Why**: Fixtures only test single-task streaming execution
   - **Fix**: Create fixtures that call this method (requires direct orchestrator API calls, not through TUI)

4. **Lines 366-370**: `process_request()` - High-level API that analyzes then executes
   - **Why**: Fixtures don't use this entry point
   - **Fix**: Create fixtures using this API

5. **Lines 145-160**: `analyze_request()` - Task analysis/decomposition
   - **Why**: Never called by fixtures
   - **Fix**: Test via `process_request()` or directly

### Why CLI Entry Points Have 0% Coverage

**Files affected:**
- `cli.rs` (0.0%)
- `interactive.rs` (0.0%)
- `handlers.rs` (0.0%)

**Why:** Fixtures create `TuiApp` directly, bypassing the CLI layer entirely.

**Execution path:**
```
Normal: CLI args → handlers → interactive mode → TuiApp
Fixture: → TuiApp (direct creation)
```

**Fix options:**
1. Create fixtures that invoke the actual CLI binary (integration test style)
2. Accept that CLI entry points are tested manually/separately
3. Create unit tests for CLI parsing logic

### Why TUI Components Have Low Coverage

**Files affected:**
- `ui/app/lifecycle.rs` (0.0%)
- `ui/app/thread_operations.rs` (0.0%)
- `ui/app/input_handler.rs` (12.0%)
- `ui/renderer/task_rendering.rs` (6.8%)

**Why:** Fixtures send minimal UI events and don't exercise all UI code paths.

**Missing coverage:**
- App lifecycle methods (startup, shutdown, error handling)
- Thread switching/creation/deletion
- Complex navigation scenarios
- Error rendering
- Task tree expansion/collapse
- Keyboard shortcuts

**Fix:** Create fixtures with:
- Multiple thread creation/switching
- Navigation events (arrow keys, tab, etc.)
- Error scenarios
- Complex task hierarchies

## Action Plan to Reach >70% Coverage

###  Priority 1: Enable Self-Determination Testing (CRITICAL)

**Target files:**
- `agent/self_assess.rs`: 0% → >70%
- `agent/executor/self_determining.rs`: 14.2% → >70%
- `agent/executor/mod.rs`: 38.4% → >70%

**Steps:**
1. Verify the mock provider can return assessment-format responses
2. Create fixtures with:
   - Assessment responses returning "COMPLETE" action
   - Assessment responses returning "DECOMPOSE" with subtasks
   - Assessment responses returning "GATHER" with context needs
3. Ensure complex task triggers (avoid conversational/simple patterns)

**Fixture examples needed:**
- `agent/self_determination_complete.json` - Task assessed and completed
- `agent/self_determination_decompose.json` - Task decomposed into subtasks
- `agent/self_determination_gather.json` - Task requests more context

### Priority 2: Conversation History Testing

**Target files:**
- `agent/conversation.rs`: 0% → >70%
- `thread_store.rs`: 18.2% → >70%

**Steps:**
1. Create fixtures with multiple turns in same thread
2. Test conversation history extraction
3. Test thread persistence and loading

**Fixture examples needed:**
- `threads/multi_turn_conversation.json` - Multiple messages in one thread
- `threads/thread_persistence.json` - Save and load thread state
- `threads/conversation_context.json` - Use history in subsequent requests

### Priority 3: Tooling Coverage

**Target files:**
- `file_ops.rs`: 49.3% → >70%
- `bash.rs`: 42.4% → >70%
- `edit_tool.rs`: 33.3% → >70%
- `context_request.rs`: 40.1% → >70%

**Steps:**
1. Create fixtures exercising all tool methods
2. Test error cases (file not found, permission denied, etc.)
3. Test edge cases (empty files, large files, special characters)

**Fixture examples needed:**
- `tools/file_operations_comprehensive.json` - All file ops with error cases
- `tools/bash_execution_scenarios.json` - Various bash commands
- `tools/edit_tool_edge_cases.json` - Replace, replace_all, errors
- `tools/context_request_patterns.json` - Different pattern types

### Priority 4: Validation Pipeline

**Target files:**
- `validator/pipeline.rs`: 25.0% → >70%
- `validator/stages/test.rs`: 31.0% → >70%
- `validator/stages/build.rs`: 33.3% → >70%
- `validator/stages/lint.rs`: 33.3% → >70%

**Steps:**
1. Create fixtures that produce code requiring validation
2. Test validation failures and successes
3. Test early exit behavior

**Fixture examples needed:**
- `validation/syntax_errors.json` - Code with syntax errors
- `validation/build_failures.json` - Code that fails to build
- `validation/lint_warnings.json` - Code with lint issues
- `validation/all_stages_pass.json` - Clean code through pipeline

### Priority 5: TUI Components

**Target files:**
- All `ui/app/*` files with <40% coverage

**Steps:**
1. Create fixtures with complex UI interactions
2. Test all navigation paths
3. Test error rendering

**Fixture examples needed:**
- `tui/navigation_comprehensive.json` - All navigation actions
- `tui/thread_management.json` - Create/switch/delete threads
- `tui/error_display.json` - Various error scenarios
- `tui/task_tree_interaction.json` - Expand/collapse, selection

### Priority 6: Orchestrator Completeness

**Target:**
- `orchestrator.rs`: 48.7% → >70%

**Steps:**
1. Test `process_request()` API
2. Test `analyze_request()` directly
3. Test parallel task execution (if possible via fixtures)

**Note:** Some uncovered code (`new()`, builder methods) may not need fixture coverage as they're tested via unit tests or used in production only.

## Implementation Strategy

### Phase 1: Foundation (Days 1-2)
1. Verify mock provider supports assessment responses
2. Create 3 self-determination fixtures (complete/decompose/gather)
3. Run coverage to verify self-assessment code is now hit

### Phase 2: Core Features (Days 3-4)
1. Create conversation/thread fixtures (3-5 fixtures)
2. Create comprehensive tool fixtures (4-6 fixtures)
3. Run coverage, aim for >50% overall

### Phase 3: Advanced Features (Days 5-6)
1. Create validation pipeline fixtures (4 fixtures)
2. Create TUI interaction fixtures (5-8 fixtures)
3. Run coverage, aim for >60% overall

### Phase 4: Polish (Day 7)
1. Identify remaining gaps
2. Create targeted fixtures for specific uncovered lines
3. Final coverage run, verify >70% for all SHOULD COVER files
4. Update FIXTURE_COVERAGE.md

## Expected Outcomes

**Before:**
- Overall: 25.57% lines (4105/16052)
- Functions: 8.66% (513/5926)
- Critical paths: 0-48% coverage
- Self-determination: Untested
- Conversation: Untested

**After (estimated):**
- Overall: >60% lines
- Functions: >40%
- Critical paths: >70% coverage
- Self-determination: Fully tested
- Conversation: Fully tested
- All user-facing features: Well covered

## Technical Challenges

### Challenge 1: Assessment Response Format
The mock provider returns TypeScript code that returns strings. Assessment needs to return JSON. Need to verify this works:

```typescript
async function agent_code(): Promise<string> {
  return JSON.stringify({ action: 'COMPLETE', ... });
}
```

### Challenge 2: Multi-Step Assessment
Self-determination loop may call provider multiple times (assess → gather → assess again). Fixtures need multiple LLM response blocks with different triggers.

### Challenge 3: CLI Layer Testing
CLI entry points may need a different testing approach (spawn actual binary) or accept they're manually tested.

### Challenge 4: Async/Parallel Execution
Some orchestrator methods (`execute_tasks`) are designed for parallel execution. Fixtures might not easily test this without direct API access.

## Files Not Requiring Fixture Coverage

These should stay in "SHOULDN'T COVER" section:

- **Test infrastructure**: `**/tests.rs`, `**/test_helpers.rs`
- **Provider implementations**: `openrouter.rs`, `groq.rs`, `mock.rs` (mocked in fixtures)
- **Internal routing**: `analyzer/*`, `router/*` (tested via unit tests)
- **Embeddings**: `embedding/*` (separate test suite)
- **Core types**: `error.rs`, `types.rs` (passive structures)

## Conclusion

Reaching >70% fixture coverage for SHOULD COVER files is achievable but requires:
1. **Fundamental fix**: Enable self-determination testing (currently at 0%)
2. **Systematic approach**: Create fixtures for each uncovered feature area
3. **20-30 new fixtures**: Covering self-determination, conversation, tools, validation, TUI
4. **1 week of focused effort**: Following the phased implementation plan

The current low coverage is not due to fixture system limitations, but rather that critical features (especially self-determination) have never been tested via fixtures.
