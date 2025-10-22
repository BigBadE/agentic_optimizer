# Task List Test Fixtures

This directory contains JSON fixtures for testing agent task decomposition and execution.

## Fixture Structure

Each fixture file defines:
- **name**: Test scenario name
- **description**: What the test verifies
- **initial_query**: User's initial request
- **mock_responses**: Agent responses for each query pattern
- **expected_task_list**: Expected task decomposition structure
- **expected_outcomes**: Expected results after execution

## Available Fixtures

### 1. simple_implementation.json
**Scenario**: Basic sequential task execution

- Agent decomposes a simple request into 3 sequential tasks
- Each task depends on the previous one completing
- Tests basic task chaining: Create → Test → Verify

**Dependency Chain**: `[[], [1], [2]]`
- Task 1: No dependencies
- Task 2: Depends on task 1
- Task 3: Depends on task 2

### 2. parallel_tasks.json
**Scenario**: Parallel task execution with convergence

- Agent creates independent tasks that can run in parallel
- Later tasks depend on multiple parallel tasks completing
- Tests parallel execution and dependency merging

**Dependency Chain**: `[[], [], [1, 2], [3]]`
- Task 1: Independent
- Task 2: Independent (can run parallel to task 1)
- Task 3: Depends on both tasks 1 and 2
- Task 4: Depends on task 3

### 3. test_failure_recovery.json
**Scenario**: Agent handles test failures and fixes bugs

- Agent writes initial implementation with a bug
- Tests fail and report the error
- Agent adds additional tasks to fix the bug
- Tests pass after the fix

**Dependency Chain**: `[[], [1], [2], [3], [4]]`
- Task 1: Create buggy implementation
- Task 2: Write tests
- Task 3: Run tests (fails)
- Task 4: Fix implementation based on test failure
- Task 5: Verify fix works

**Key Feature**: Demonstrates self-correction after validation failure

### 4. deep_dependency_chain.json
**Scenario**: Long sequential dependency chain

- Tests agent's ability to handle deep sequential dependencies
- 5-step pipeline: Parse → Validate → Transform → Report → Email
- Each step depends only on immediate predecessor

**Dependency Chain**: `[[], [1], [2], [3], [4]]`
- Validates linear pipeline execution without parallelization

### 5. complex_dag.json
**Scenario**: Complex directed acyclic graph (DAG)

- Multiple independent tasks converge at multiple points
- Tests ability to handle complex dependency graphs
- 6 tasks with 2 convergence points

**Dependency Chain**: `[[], [], [1, 2], [3], [], [4, 5]]`
- Tasks 1 & 2: Independent (source/tests)
- Task 3: Converges 1 & 2 (compilation)
- Task 4: Depends on 3 (linking)
- Task 5: Independent (documentation)
- Task 6: Converges 4 & 5 (packaging)

### 6. empty_task_list.json
**Scenario**: Queries that don't require task decomposition

- Simple questions that can be answered immediately
- No task list needed
- Tests agent's ability to recognize trivial queries

**Dependency Chain**: `[]` (empty)
- Validates handling of zero-task scenarios

### 7. circular_dependency_detection.json
**Scenario**: Handling of impossible circular dependencies

- Agent should detect and restructure to avoid circular deps
- Tests error handling and intelligent restructuring
- Fallback to single task if decomposition impossible

**Dependency Chain**: `[[]]` (single independent task)
- Validates circular dependency avoidance

### 8. multiple_failures_retry.json
**Scenario**: Multiple consecutive failures with retries

- Implementation fails multiple times
- Agent iteratively fixes issues
- Tests persistence and multi-step debugging

**Dependency Chain**: `[[], [1], [2], [3]]`
- Task 1: Initial buggy implementation
- Task 2: First fix attempt (still buggy)
- Task 3: Second fix attempt (still buggy)
- Task 4: Final successful fix

**Key Feature**: Demonstrates resilience through multiple failure cycles

## Dependency Chain Format

Dependencies are represented as `Vec<Vec<u32>>`, where:
- Outer array index = task index (0-based)
- Inner array = list of task IDs this task depends on (1-based)

Example:
```json
"dependency_chain": [[], [1], [2, 3]]
```
Means:
- Task 1 (index 0): No dependencies
- Task 2 (index 1): Depends on task ID 1
- Task 3 (index 2): Depends on task IDs 2 and 3

## Usage in Tests

```rust
use task_list_fixture_runner::TaskListFixture;

let fixture = TaskListFixture::load("tests/fixtures/task_lists/simple_implementation.json")?;

// Create mock provider with fixture responses
let provider = fixture.create_mock_provider();

// Run your agent execution
let actual_tasks = run_agent_execution(...);

// Verify against expected structure
fixture.verify_task_list(&actual_tasks)?;
```

## Verification

The fixture verification checks:
1. **Task count**: Actual matches expected number
2. **Descriptions**: Actual descriptions contain expected substrings
3. **Dependencies**: Actual dependencies match expected dependency chain

All three checks must pass for verification to succeed.
