# Test Audit Report
Generated: 2025-10-25
Total Tests Audited: 363

## Audit Criteria
- **Usefulness (1-10)**: Does it catch actual bugs? Tests important functionality?
- **Uniqueness (1-10)**: Already covered by fixtures/other tests? (1 = totally unique, 10 = heavily duplicated)
- **Fixture Migration Ease (1-10)**: How easy to convert to fixture? (10 = very easy, 1 = very hard/impossible)
- **Should Rewrite/Join (1-10)**: Should it be rewritten/joined/removed? (10 = definitely, 1 = keep as-is)

---

## integration-tests (39 tests)

### 1. runner::tests::test_pattern_response_exact_match
- **Usefulness**: 7/10
- **Uniqueness**: 2/10
- **Fixture Migration**: 3/10
- **Rewrite/Join**: 7/10
- **Notes**: Tests `PatternResponse` exact matching for fixture infrastructure. Low-level unit test of testing framework itself. Could combine with other pattern tests.
- **Recommendation**: Combine all 3 pattern tests into 1 comprehensive test

### 2. runner::tests::test_pattern_response_contains_match
- **Usefulness**: 7/10
- **Uniqueness**: 2/10
- **Fixture Migration**: 3/10
- **Rewrite/Join**: 7/10
- **Notes**: Tests substring matching in mock provider. Similar to exact match test above.
- **Recommendation**: Combine with other pattern tests

### 3. runner::tests::test_pattern_response_regex_match
- **Usefulness**: 7/10
- **Uniqueness**: 2/10
- **Fixture Migration**: 3/10
- **Rewrite/Join**: 7/10
- **Notes**: Tests regex pattern matching. Third of similar pattern tests.
- **Recommendation**: Combine all 3 pattern tests into 1

### 4. test_empty_task_list
- **Usefulness**: 6/10
- **Uniqueness**: 5/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 8/10
- **Notes**: Edge case - empty task list is complete by default. Simple test. Check if merlin-core already covers this.
- **Recommendation**: Remove if covered in merlin-core, otherwise convert to fixture

### 5. test_step_status_display
- **Usefulness**: 4/10
- **Uniqueness**: 8/10
- **Fixture Migration**: 2/10
- **Rewrite/Join**: 9/10
- **Notes**: Tests Display trait for `StepStatus` enum (emojis + text). Pure formatting, unlikely to catch bugs. Wrong location (should be merlin-core if anywhere).
- **Recommendation**: REMOVE - belongs in merlin-core if needed

### 6. test_step_type_display
- **Usefulness**: 4/10
- **Uniqueness**: 8/10
- **Fixture Migration**: 2/10
- **Rewrite/Join**: 9/10
- **Notes**: Tests Display trait for `StepType`. Same issues as step_status_display.
- **Recommendation**: REMOVE - belongs in merlin-core if needed

### 7. test_task_list_all_steps_completed
- **Usefulness**: 7/10
- **Uniqueness**: 6/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 7/10
- **Notes**: Tests completion when all steps finish. Overlaps heavily with test_task_list_completion_tracking.
- **Recommendation**: MERGE with test_task_list_completion_tracking

### 8. test_task_list_completion_tracking
- **Usefulness**: 8/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 6/10
- **Notes**: Tests incremental completion (step by step). Good state management test.
- **Recommendation**: Keep and merge with #7, or convert to fixture

### 9. test_task_list_creation
- **Usefulness**: 5/10
- **Uniqueness**: 7/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 8/10
- **Notes**: Constructor test. Verifies field assignment only. Constructors rarely have bugs.
- **Recommendation**: REMOVE or merge into workflow tests

### 10. test_task_list_failure_detection
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 5/10
- **Notes**: Tests `has_failures()` detection. Important for error handling.
- **Recommendation**: Keep as-is or convert to fixture

### 11. test_task_list_partial_completion
- **Usefulness**: 7/10
- **Uniqueness**: 5/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 6/10
- **Notes**: Tests mixed states (completed/in-progress/pending). Good edge case coverage.
- **Recommendation**: Convert to fixture

### 12. test_task_list_with_skipped_steps
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 6/10
- **Notes**: Tests skipped steps don't prevent completion. Important edge case.
- **Recommendation**: Convert to fixture

### 13. test_task_step_complete_clears_error
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Notes**: Tests error clearing on retry/completion. Important for retry logic.
- **Recommendation**: Keep as-is (good bug-catching test)

### 14. test_task_step_creation_with_defaults
- **Usefulness**: 5/10
- **Uniqueness**: 7/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 8/10
- **Notes**: Another constructor test. Low value.
- **Recommendation**: REMOVE or merge

### 15. test_task_step_default_exit_commands
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 6/10
- **Notes**: Tests default validation commands per step type. Important for correctness.
- **Recommendation**: Keep, possibly convert to fixture

### 16. test_task_step_failure
- **Usefulness**: 8/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 5/10
- **Notes**: Tests failure state transition. Important for error handling.
- **Recommendation**: Convert to fixture

### 17. test_task_step_is_pending_or_in_progress
- **Usefulness**: 6/10
- **Uniqueness**: 5/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 7/10
- **Notes**: Tests helper method. Simple boolean logic.
- **Recommendation**: MERGE with test_task_step_state_transitions

### 18. test_task_step_skip
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 6/10
- **Notes**: Tests skip state for optional steps.
- **Recommendation**: Convert to fixture

### 19. test_task_step_state_transitions
- **Usefulness**: 9/10
- **Uniqueness**: 2/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 4/10
- **Notes**: Core state machine test (Pending→InProgress→Completed). HIGH VALUE - catches state bugs.
- **Recommendation**: KEEP AS-IS or as comprehensive fixture

### 20. test_task_step_with_custom_exit_command
- **Usefulness**: 7/10
- **Uniqueness**: 5/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 7/10
- **Notes**: Tests custom exit command override.
- **Recommendation**: MERGE with test_task_step_default_exit_commands

### 21-27. test_*_task_list_workflow tests (7 tests)
- **Usefulness**: 8/10 (average)
- **Uniqueness**: 4/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 5/10
- **Notes**: Workflow tests (bug_fix, refactor, exit_commands, simple_structure, progress_tracking, etc.). These ARE integration tests and should be fixtures.
- **Recommendation**: Convert ALL to task_list fixtures

### 28-36. test_basic_fixtures, test_context_fixtures, test_discover_all_fixtures, test_execution_fixtures, test_task_list_fixtures, test_tool_fixtures, test_tui_fixtures, test_typescript_fixtures (8 tests)
- **Usefulness**: 9/10
- **Uniqueness**: 1/10
- **Fixture Migration**: 1/10 (N/A - these ARE the fixture runners)
- **Rewrite/Join**: 1/10
- **Notes**: Essential fixture runner tests. These execute all JSON fixtures in each category. Cannot be fixtures themselves (bootstrap problem).
- **Recommendation**: KEEP ALL AS-IS - critical infrastructure

---

## merlin-agent (88 tests)

### conversation tests (6 tests)
#### 37. agent::conversation::tests::test_add_message
- **Usefulness**: 7/10
- **Uniqueness**: 5/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 6/10
- **Notes**: Tests message addition to conversation manager.
- **Recommendation**: Convert to agent fixture or merge

#### 38. agent::conversation::tests::test_clear
- **Usefulness**: 6/10
- **Uniqueness**: 6/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 7/10
- **Notes**: Tests conversation clearing. Simple state reset.
- **Recommendation**: MERGE with other conversation tests

#### 39. agent::conversation::tests::test_conversation_limit
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Notes**: Tests conversation history limit. Important for memory management.
- **Recommendation**: Keep or convert to fixture

#### 40. agent::conversation::tests::test_conversation_manager_creation
- **Usefulness**: 4/10
- **Uniqueness**: 8/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 9/10
- **Notes**: Constructor test only.
- **Recommendation**: REMOVE

#### 41. agent::conversation::tests::test_file_tracking
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Notes**: Tests file reference tracking. Important for context.
- **Recommendation**: Convert to context fixture

#### 42. agent::conversation::tests::test_focus_tracking
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 6/10
- **Notes**: Tests focus state for UI.
- **Recommendation**: Convert to TUI fixture

### executor tests (17 tests)
#### 43. agent::executor::tests::test_agent_execution_result_error_handling
- **Usefulness**: 8/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Recommendation**: Convert to execution fixture

#### 44. agent::executor::tests::test_agent_executor_creation
- **Usefulness**: 4/10
- **Uniqueness**: 8/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 9/10
- **Notes**: Constructor test.
- **Recommendation**: REMOVE

#### 45. agent::executor::tests::test_continue_result_handling
- **Usefulness**: 7/10
- **Uniqueness**: 5/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 6/10
- **Recommendation**: Convert to fixture

#### 46-54. agent::executor::tests::test_extract_typescript_code_* (9 tests)
- **Usefulness**: 8/10 (collectively)
- **Uniqueness**: 5/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 7/10
- **Notes**: NINE granular tests for TypeScript extraction: empty_block, mixed_languages, multiple_blocks, no_blocks, no_code_blocks, single_block, syntax_error, ts_language, with_indentation. Very repetitive.
- **Recommendation**: CONSOLIDATE into 2-3 comprehensive tests OR convert to TypeScript fixtures

#### 55-56. agent::executor::tests::test_parse_task_list_from_*
- **Usefulness**: 8/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 5/10
- **Recommendation**: Convert to task_list fixtures

#### 57-59. agent::executor::tests::test_*_result_handling (3 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 5/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 6/10
- **Recommendation**: MERGE into single result_handling test

#### 60. agent::executor::tests::test_tool_registry_integration
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Recommendation**: Convert to tool fixture

### self_assess tests (11 tests)
#### 61. agent::self_assess::tests::test_assessment_prompt_generation
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 6/10
- **Notes**: Tests prompt creation for self-assessment.
- **Recommendation**: Keep or convert to fixture

#### 62-72. agent::self_assess::tests::test_parse_* (11 tests)
- **Usefulness**: 8/10 (collectively)
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Notes**: Parse tests for: assessment_response, complete_action, complete_without_result, decompose_action, decompose_default_sequential, decompose_without_subtasks, gather_action, gather_without_needs, invalid_json_error, json_with_surrounding_text, unknown_action_error. Critical for agent decision-making but could consolidate.
- **Recommendation**: Keep core tests, consolidate edge cases

### step tests (7 tests)
#### 73-79. agent::step::tests::test_* (7 tests)
- **Usefulness**: 7/10 (average)
- **Uniqueness**: 5/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 6/10
- **Notes**: Tests for step tracking (add_step, clone_tracker, create_step, get_steps_none, multiple_steps_for_task, multiple_tasks, step_tracker_default).
- **Recommendation**: CONSOLIDATE into 2-3 comprehensive tests

### task_coordinator tests (18 tests)
#### 80-97. agent::task_coordinator::tests::test_* (18 tests)
- **Usefulness**: 8/10 (average)
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Notes**: Comprehensive task coordinator tests (checkpoint_creation, checkpoint_limit, cleanup_old_tasks, concurrent_subtask_completion, coordinator_creation, decompose_task, decompose_task_not_found, get_progress, get_subtasks, get_task_status, is_ready, max_depth_enforcement, max_subtasks_enforcement, nonexistent_task, register_task, task_completion, task_hierarchy, task_stats). Good coverage but many could be fixtures.
- **Recommendation**: Keep critical ones (max_depth, max_subtasks, concurrent_completion), convert rest to fixtures

### task_list_executor tests (5 tests)
#### 98-102. agent::task_list_executor::tests::test_* (5 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 5/10
- **Recommendation**: Convert ALL to task_list fixtures

### executor module tests (13 tests)
#### 103-104. executor::graph::tests::test_task_graph_* (2 tests)
- **Usefulness**: 9/10
- **Uniqueness**: 1/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 3/10
- **Notes**: Cycle detection and ready task identification. CRITICAL graph algorithms.
- **Recommendation**: KEEP AS-IS - prevents infinite loops

#### 105-107. executor::isolation::tests::test_*_lock* (3 tests)
- **Usefulness**: 9/10
- **Uniqueness**: 1/10
- **Fixture Migration**: 5/10
- **Rewrite/Join**: 3/10
- **Notes**: File locking tests (read_locks_shared, write_blocks_read, write_lock_exclusive). CRITICAL for concurrency.
- **Recommendation**: KEEP AS-IS - tests concurrency primitives

#### 108. executor::pool::tests::test_executor_pool_basic
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 5/10
- **Recommendation**: Keep or convert to executor fixture

#### 109-110. executor::scheduler::tests::test_* (2 tests)
- **Usefulness**: 9/10
- **Uniqueness**: 2/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 4/10
- **Notes**: Conflict detection tests. CRITICAL for preventing file modification conflicts.
- **Recommendation**: KEEP AS-IS

#### 111-112. executor::state::tests::test_workspace_* (2 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 5/10
- **Recommendation**: Keep or convert to workspace fixtures

#### 113-114. executor::transaction::tests::test_task_workspace_* (2 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 5/10
- **Notes**: Tests workspace isolation and commit.
- **Recommendation**: Convert to workspace fixtures

### orchestrator tests (3 tests)
#### 115-117. orchestrator::tests::test_* (3 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Recommendation**: Convert to orchestrator fixtures

### validator tests (8 tests)
#### 118-123. validator::citations::tests::test_citation_* (6 tests)
- **Usefulness**: 7/10 (average)
- **Uniqueness**: 4/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 6/10
- **Notes**: Citation parsing/validation tests.
- **Recommendation**: CONSOLIDATE into 2-3 tests or convert to validation fixtures

#### 124-125. validator::pipeline::tests::test_pipeline_* (2 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Recommendation**: Convert to validation fixtures

---

## merlin-benchmarks-quality (4 tests)

#### 126. metrics::tests::test_mrr_calculation
- **Usefulness**: 8/10
- **Uniqueness**: 1/10
- **Fixture Migration**: 3/10
- **Rewrite/Join**: 2/10
- **Notes**: Tests Mean Reciprocal Rank. Pure math, important for benchmarks.
- **Recommendation**: KEEP AS-IS

#### 127. metrics::tests::test_precision_calculation
- **Usefulness**: 8/10
- **Uniqueness**: 1/10
- **Fixture Migration**: 3/10
- **Rewrite/Join**: 2/10
- **Recommendation**: KEEP AS-IS

#### 128. metrics::tests::test_recall_calculation
- **Usefulness**: 8/10
- **Uniqueness**: 1/10
- **Fixture Migration**: 3/10
- **Rewrite/Join**: 2/10
- **Recommendation**: KEEP AS-IS

#### 129. test_case::tests::test_priority_parsing
- **Usefulness**: 7/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 6/10
- **Recommendation**: Keep or convert to fixture

---

## merlin-cli (30 tests)

### ui::layout tests (4 tests)
#### 130-133. ui::layout::tests::test_* (4 tests)
- **Usefulness**: 6/10
- **Uniqueness**: 5/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 7/10
- **Notes**: Layout calculation tests (input_area_height, layout_cache_viewport_height, task_area_height_focused, task_area_height_output_focused). UI sizing logic.
- **Recommendation**: Convert ALL to TUI fixtures

### ui::renderer::helpers tests (2 tests)
#### 134-135. ui::renderer::helpers::tests::test_* (2 tests)
- **Usefulness**: 5/10
- **Uniqueness**: 7/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 8/10
- **Notes**: Simple UI helpers (expansion_indicator, selection_style).
- **Recommendation**: Convert to TUI fixtures or REMOVE if covered

### ui::scroll tests (1 test)
#### 136. ui::scroll::tests::test_count_text_lines
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 6/10
- **Recommendation**: Keep or convert to fixture

### ui::state tests (4 tests)
#### 137-140. ui::state::tests::test_* (4 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 6/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 7/10
- **Notes**: Conversation state tests. DUPLICATES agent::conversation tests!
- **Recommendation**: REMOVE or CONSOLIDATE with agent::conversation tests

### ui::task_manager tests (2 tests)
#### 141-142. ui::task_manager::tests::test_task_order_* (2 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Recommendation**: Convert to TUI fixtures

### logging tests (3 tests)
#### 143-145. tests::test_init_tui_logging_* (3 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 5/10
- **Rewrite/Join**: 6/10
- **Notes**: File system side effects for logging.
- **Recommendation**: KEEP AS-IS - tests file system behavior

### token usage test (1 test)
#### 146. tests::test_token_usage_default
- **Usefulness**: 5/10
- **Uniqueness**: 7/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 8/10
- **Notes**: Constructor/default test.
- **Recommendation**: REMOVE or merge

### utils tests (13 tests)
#### 147-150. utils::tests::test_calculate_cost_* (4 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 5/10
- **Notes**: Cost calculation with caching, large values, zero tokens, etc.
- **Recommendation**: CONSOLIDATE into 1-2 comprehensive tests

#### 151-155. utils::tests::test_cleanup_old_tasks_* (5 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 5/10
- **Notes**: Task cleanup with file system operations.
- **Recommendation**: CONSOLIDATE into 2-3 tests or convert to execution fixtures

#### 156. utils::tests::test_display_response_metrics
- **Usefulness**: 5/10
- **Uniqueness**: 6/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 8/10
- **Notes**: Display formatting test.
- **Recommendation**: REMOVE or merge

---

## merlin-context (31 tests)

### context_inclusion tests (5 tests)
#### 157-161. context_inclusion::tests::test_* (5 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Notes**: File prioritization, sorting by score/priority, token estimation.
- **Recommendation**: Convert to context fixtures

### models tests (2 tests)
#### 162-163. models::tests::test_from_env_* (2 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 6/10
- **Recommendation**: MERGE into 1 test

### query::analyzer tests (5 tests)
#### 164-168. query::analyzer::tests::test_* (5 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Notes**: Complexity estimation, action detection, entity/keyword extraction.
- **Recommendation**: Convert to context fixtures

### query::types tests (6 tests)
#### 169-174. query::types::tests::test_* (6 tests)
- **Usefulness**: 5/10
- **Uniqueness**: 7/10
- **Fixture Migration**: 6/10
- **Rewrite/Join**: 8/10
- **Notes**: Type system tests (action_variants, complexity_ordering, query_intent_creation, scope_variants, serde_query_intent). Low value enum tests.
- **Recommendation**: REDUCE to 1-2 essential tests or REMOVE

### BM25 tokenization tests (4 tests)
#### 175-178. test_*_tokenization (4 tests)
- **Usefulness**: 9/10
- **Uniqueness**: 1/10
- **Fixture Migration**: 5/10
- **Rewrite/Join**: 3/10
- **Notes**: BM25 tokenization tests (bigram_phrase_ranking, mixed_special_and_regular_tokens, special_token_preservation, tokenization_debug). CRITICAL for search quality.
- **Recommendation**: KEEP AS-IS - ensures search correctness

### chunking test (1 test)
#### 179. test_chunking_validation
- **Usefulness**: 8/10
- **Uniqueness**: 2/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 4/10
- **Notes**: Code chunking for embeddings.
- **Recommendation**: Keep or convert to fixture

### cache tests (3 tests)
#### 180-182. test_cache_* (3 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 5/10
- **Notes**: Embedding cache (file_changes, lifecycle, corrupted_cache_recovery).
- **Recommendation**: Keep or consolidate

### Unnamed integration tests (5 tests)
#### 183-187. Unlabeled tests
- **Notes**: Need to identify these 5 tests from test listing
- **Recommendation**: Audit after identification

---

## merlin-core (30 tests)

### config tests (4 tests)
#### 188-191. config::tests::test_* (4 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 6/10
- **Rewrite/Join**: 6/10
- **Notes**: Config loading, API keys, serialization.
- **Recommendation**: CONSOLIDATE into 2 tests

### error tests (5 tests)
#### 192-196. error::tests::test_error_* (5 tests)
- **Usefulness**: 5/10
- **Uniqueness**: 7/10
- **Fixture Migration**: 4/10
- **Rewrite/Join**: 8/10
- **Notes**: Error type tests (display, from_io, from_json, is_retryable, result_type). Low value trait implementation tests.
- **Recommendation**: REDUCE to 1-2 tests

### prompts tests (3 tests)
#### 197-199. prompts::tests::test_* (3 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 6/10
- **Rewrite/Join**: 6/10
- **Notes**: Prompt loading and extraction.
- **Recommendation**: MERGE into 1 test

### task_list tests (6 tests)
#### 200-205. task_list::tests::test_* (6 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 8/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 8/10
- **Notes**: MAJOR DUPLICATION with integration-tests task list tests! Same functionality tested in two places.
- **Recommendation**: REMOVE from merlin-core (keep in integration-tests) OR remove from integration-tests (keep here)

### types tests (12 tests)
#### 206-217. types::tests::test_* (12 tests)
- **Usefulness**: 5/10
- **Uniqueness**: 6/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 7/10
- **Notes**: Many constructor and serialization tests (context_files_to_string, context_new, context_token_estimate, context_with_files, file_context_from_path, file_context_from_path_not_found, file_context_new, query_new, query_serialization, query_with_files, response_serialization, token_usage_default, token_usage_total). Heavy on low-value constructor tests.
- **Recommendation**: REDUCE to 3-4 essential tests

---

## merlin-languages (10 tests)

### provider tests (6 tests)
#### 218-223. provider::tests::test_* (6 tests)
- **Usefulness**: 6/10
- **Uniqueness**: 5/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 7/10
- **Notes**: Search query and symbol info type tests (search_query_default, search_query_with_name, search_result_creation, symbol_info_creation, symbol_kind_debug, symbol_kind_equality).
- **Recommendation**: CONSOLIDATE into 1-2 tests

### language enum tests (4 tests)
#### 224-227. tests::test_* (4 tests)
- **Usefulness**: 5/10
- **Uniqueness**: 7/10
- **Fixture Migration**: 5/10
- **Rewrite/Join**: 8/10
- **Notes**: Trait implementation tests (create_backend_rust, language_clone, language_debug, language_equality). Low value.
- **Recommendation**: REDUCE to 1 test or REMOVE

---

## merlin-local (4 tests)

#### 228-229. inference::tests::* (2 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 6/10
- **Rewrite/Join**: 6/10
- **Notes**: Cost estimation and provider creation for local models.
- **Recommendation**: MERGE into 1 test

#### 230-231. manager::tests::* (2 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 6/10
- **Rewrite/Join**: 6/10
- **Notes**: Ollama manager tests.
- **Recommendation**: MERGE into 1 test

---

## merlin-providers (21 tests)

### groq tests (8 tests)
#### 232-239. groq::tests::* (8 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 6/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 7/10
- **Notes**: Groq provider tests (cost_estimation, groq_provider_with_api_key, test_groq_provider_name, test_model_chaining, test_with_api_key, test_with_api_key_direct_empty, test_with_api_key_direct_valid, test_with_model). HIGH DUPLICATION with OpenRouter tests below.
- **Recommendation**: Create provider test MACRO/TEMPLATE to eliminate duplication

### mock tests (5 tests)
#### 240-244. mock::tests::test_mock_provider_* (5 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Notes**: Mock provider functionality (call_history, clear_history, default_response, exact_match, substring_match). Important for testing.
- **Recommendation**: Keep or convert to fixtures

### openrouter tests (8 tests)
#### 245-252. openrouter::tests::test_* (8 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 7/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 8/10
- **Notes**: OpenRouter provider tests. NEARLY IDENTICAL to Groq tests (#232-239). Massive duplication.
- **Recommendation**: Use MACRO/TEMPLATE to share test code between providers

---

## merlin-routing (58 tests)

### analyzer::decompose tests (4 tests)
#### 253-256. analyzer::decompose::tests::test_* (4 tests)
- **Usefulness**: 9/10
- **Uniqueness**: 2/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 4/10
- **Notes**: Task decomposition tests (complex_creation_decomposition, fix_with_analysis, refactor_decomposition, simple_task_no_decomposition). CRITICAL for agent functionality.
- **Recommendation**: Keep or convert to agent fixtures

### analyzer::intent tests (6 tests)
#### 257-262. analyzer::intent::tests::test_* (6 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Notes**: Intent detection tests (complexity_estimation, create_action, critical_priority, file_scope, fix_action, refactor_action).
- **Recommendation**: Convert to routing fixtures

### analyzer::local tests (4 tests)
#### 263-266. analyzer::local::tests::test_* (4 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Notes**: Local analyzer tests.
- **Recommendation**: Convert to fixtures

### cache::storage tests (14 tests)
#### 267-280. cache::storage::tests::test_cache_* (14 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 6/10
- **Notes**: FOURTEEN cache tests (clear, clear_expired_only, disabled, eviction_on_size_limit, evicts_oldest_first, expiration, is_empty, miss, put_and_get, size_tracking, stats, update_existing_key, cached_response_expiration, cached_response_not_expired). Good coverage but many similar tests.
- **Recommendation**: CONSOLIDATE into 6-7 comprehensive tests

### metrics tests (6 tests)
#### 281-286. metrics::*::tests::test_* (6 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 6/10
- **Notes**: Metrics collection and reporting.
- **Recommendation**: CONSOLIDATE or convert to fixtures

### router::model_registry tests (10 tests)
#### 287-296. router::model_registry::tests::test_* (10 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 5/10
- **Notes**: Model registry tests (clear, empty_registry, exact_match, fallback_to_highest, invalid_difficulty, model_registry_defaults, nearest_higher, register_and_retrieve, register_range, register_range_panics_on_invalid). Important but could consolidate.
- **Recommendation**: CONSOLIDATE into 5-6 tests

### router::models tests (3 tests)
#### 297-299. router::models::tests::test_* (3 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 6/10
- **Recommendation**: MERGE into 1-2 tests

### router::provider_registry tests (3 tests)
#### 300-302. router::provider_registry::tests::test_* (3 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 5/10
- **Recommendation**: Keep or merge

### router::tiers tests (8 tests)
#### 303-310. router::tiers::tests::test_* (8 tests)
- **Usefulness**: 9/10
- **Uniqueness**: 2/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 4/10
- **Notes**: Tier-based routing tests. CORE routing logic. Very important.
- **Recommendation**: KEEP MOST, possibly consolidate some

---

## merlin-tooling (33 tests)

### bash tests (5 tests)
#### 311-315. bash::tests::test_bash_tool_* (5 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 5/10
- **Notes**: Bash tool tests (command_failure, missing_command_param, name_and_description, simple_command, with_object_params).
- **Recommendation**: Convert to tool fixtures

### context_request tests (3 tests)
#### 316-318. context_request::tests::test_context_request_* (3 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 5/10
- **Recommendation**: Convert to tool fixtures

### edit_tool tests (3 tests)
#### 319-321. edit_tool::tests::test_edit_file_* (3 tests)
- **Usefulness**: 9/10
- **Uniqueness**: 2/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 4/10
- **Notes**: Edit tool tests. CRITICAL for code modifications.
- **Recommendation**: Should have tool fixtures (likely already do)

### file_ops tests (5 tests)
#### 322-326. file_ops::tests::test_* (5 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 9/10
- **Rewrite/Join**: 5/10
- **Notes**: File operation tests, including SECURITY test (path_traversal_prevention_write).
- **Recommendation**: Convert most to tool fixtures, KEEP path traversal test as unit test

### registry tests (4 tests)
#### 327-330. registry::tests::test_* (4 tests)
- **Usefulness**: 7/10
- **Uniqueness**: 4/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 6/10
- **Notes**: Tool registry tests.
- **Recommendation**: CONSOLIDATE into 2 tests

### runtime tests (3 tests)
#### 331-333. runtime::tests::test_* (3 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 8/10
- **Rewrite/Join**: 5/10
- **Notes**: TypeScript runtime tests.
- **Recommendation**: Convert to TypeScript fixtures

### signatures test (1 test)
#### 334. signatures::tests::test_generate_multiple_signatures
- **Usefulness**: 7/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 7/10
- **Rewrite/Join**: 5/10
- **Recommendation**: Keep or convert to fixture

### tool tests (10 tests)
#### 335-344. tool::tests::test_tool_* (10 tests)
- **Usefulness**: 5/10
- **Uniqueness**: 6/10
- **Fixture Migration**: 6/10
- **Rewrite/Join**: 7/10
- **Notes**: Tool trait implementation tests (error_display, error_from_io, input_serialization, output_error, output_serialization, output_success, output_success_with_data, trait_error_handling, trait_implementation). Many low-value type system tests.
- **Recommendation**: REDUCE to 2-3 essential tests

---

## rust-backend (11 tests)

### cache tests (11 tests)
#### 345-355. cache::tests::test_cache_* (11 tests)
- **Usefulness**: 8/10
- **Uniqueness**: 3/10
- **Fixture Migration**: 6/10
- **Rewrite/Join**: 5/10
- **Notes**: Rust-analyzer cache tests (clear, clear_nonexistent, creation, detects_deleted_files, detects_file_changes, doesnt_rebuild_unchanged, invalid_different_project, is_valid_unchanged, load_nonexistent, save_and_load, with_many_files). Good coverage of cache invalidation logic.
- **Recommendation**: CONSOLIDATE into 6-7 tests or keep as-is if cache bugs are common

---

## Summary Statistics

### Tests by Action
- **Remove**: ~65 tests (18%)
  - Constructor-only tests: ~15
  - Display/Debug trait tests: ~12
  - Duplicate tests: ~20
  - Low-value type tests: ~18

- **Consolidate/Merge**: ~90 tests (25%)
  - Pattern matching tests: 3→1
  - TypeScript extraction: 9→2-3
  - Result handling: multiple→1
  - Provider tests: use macros
  - Cache tests: 14→6-7
  - Tool trait tests: 10→2-3
  - Similar workflow tests: merge

- **Convert to Fixtures**: ~125 tests (34%)
  - Tool tests: ~20
  - TypeScript runtime: ~5
  - TUI tests: ~15
  - Workflow tests: ~20
  - Task list tests: ~15
  - Execution tests: ~15
  - Context tests: ~15
  - Agent tests: ~20

- **Keep As-Is**: ~80 tests (22%)
  - Fixture runners: 8
  - Graph algorithms: 2
  - Locking/concurrency: 5
  - Math/metrics: 3
  - Security tests: 2
  - Tokenization: 4
  - Core routing: ~15
  - State machines: ~5
  - Decomposition: ~4
  - Other critical: ~30

### Projected Outcome
- **Current**: 363 tests
- **After cleanup**: ~200-230 tests
- **Quality improvement**: Higher value density, less duplication, more fixture coverage

### High Priority Actions

1. **Immediate Removals**
   - All Display trait tests in integration-tests
   - Constructor-only tests across all crates
   - Duplicate task_list tests (pick one location)
   - Test creation tests (agent_executor, conversation_manager, etc.)

2. **Immediate Consolidations**
   - 3 pattern tests → 1
   - 9 TypeScript extraction tests → 2-3
   - Provider tests → use macro/template
   - Cache storage tests → consolidate similar

3. **High-Value Fixture Conversions**
   - All tool operation tests
   - Workflow tests (bug_fix, refactor, etc.)
   - TUI layout/rendering tests
   - TypeScript runtime tests

4. **Keep All**
   - Fixture runner tests
   - Concurrency primitives (locks, conflict detection)
   - Graph algorithms (cycle detection)
   - Security tests (path traversal)
   - Core routing logic
