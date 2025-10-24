# Comprehensive E2E Testing Framework

## Overview

This document describes the new comprehensive end-to-end (E2E) testing framework for the Merlin agent system. The framework provides full verification of agent workflows using real code paths with only provider responses mocked.

## Design Principles

### 1. **Real Code Paths**
- Uses production `RoutingOrchestrator`, `AgentExecutor`, and all routing components
- Only mocks provider responses (via `StatefulMockProvider`)
- No mocking of tools, validators, or execution logic
- Providers never attempt to initialize (injected via `new_with_router`)

### 2. **Comprehensive Verification**
- File operations (creation, modification, deletion)
- Response content patterns
- Provider call counts and patterns
- Tool call tracking
- Validation results
- Task completion status

### 3. **Stateful Tracking**
- Full call history with timestamps
- Pattern matching with use-once support
- Error injection for negative testing
- Detailed verification reporting

### 4. **Fixture-Based Testing**
- JSON fixtures define complete test scenarios
- Separate positive and negative test cases
- Tag-based test organization
- Automatic fixture discovery

## Architecture

### Components

```
e2e_framework/
├── fixture.rs          # Enhanced fixture format with comprehensive verification
├── mock_provider.rs    # Stateful mock provider with call tracking
├── runner.rs           # Test runner using real code paths
├── verifier.rs         # Comprehensive result verification
└── mod.rs              # Module exports
```

### Test File Structure

```
tests/
├── comprehensive_e2e_tests.rs      # Main test suite
├── e2e_framework/                  # Framework implementation
└── fixtures/
    └── e2e/                        # Test fixtures
        ├── simple_response.json
        ├── negative_missing_response.json
        ├── negative_provider_error.json
        ├── negative_insufficient_responses.json
        └── negative_excessive_calls.json
```

## Fixture Format

### Complete Example

```json
{
  "name": "Simple Response Test",
  "description": "Tests basic agent response without tools",
  "initial_query": "What is 2 + 2?",
  "mock_responses": [
    {
      "pattern": "What is 2 + 2",
      "response": "The answer is 4.",
      "expected_tool_calls": [],
      "use_once": false,
      "should_fail": false
    }
  ],
  "expected_task_list": null,
  "expected_outcomes": {
    "all_tasks_completed": true,
    "files": [],
    "validation_passed": true,
    "response": {
      "contains": ["4"],
      "not_contains": [],
      "min_length": 5
    },
    "min_tool_calls": 0,
    "max_tool_calls": 0,
    "min_provider_calls": 1,
    "max_provider_calls": 2
  },
  "setup_files": {},
  "env_vars": {},
  "tags": ["basic", "simple"]
}
```

### Fixture Fields

#### Mock Responses
- `pattern`: Substring to match in query
- `response`: Text to return (string or array of lines)
- `expected_tool_calls`: Expected tools used (future feature)
- `use_once`: Whether pattern should only match once
- `should_fail`: Inject error for negative testing
- `error_message`: Error message if should_fail is true

#### Expected Outcomes
- `all_tasks_completed`: Whether execution should succeed
- `files`: Array of file verifications
- `validation_passed`: Expected validation result
- `response`: Response content verification
- `min_tool_calls` / `max_tool_calls`: Tool call bounds
- `min_provider_calls` / `max_provider_calls`: Provider call bounds

#### File Verification
- `path`: File path relative to workspace
- `contains`: Patterns that must be in file
- `not_contains`: Patterns that must NOT be in file
- `exact_content`: Exact content match (ignores contains/not_contains)
- `must_exist`: File must exist
- `must_not_exist`: File must NOT exist (for deletion tests)

## Usage

### Running Tests

```bash
# Run all E2E tests
cargo nextest run -p merlin-agent comprehensive_e2e_tests

# Run specific test
cargo nextest run -p merlin-agent test_simple_response

# Run only positive tests
cargo nextest run -p merlin-agent test_all_positive_fixtures

# Run only negative tests
cargo nextest run -p merlin-agent test_all_negative_fixtures

# Validate fixture structures
cargo nextest run -p merlin-agent test_all_fixtures_structure

# Debug a single fixture
cargo nextest run -p merlin-agent test_single_fixture_debug -- --ignored
```

### Creating New Fixtures

1. Create JSON file in `tests/fixtures/e2e/`
2. Define mock responses for expected queries
3. Specify expected outcomes
4. Add test tags for organization
5. Run structure validation: `cargo nextest run test_all_fixtures_structure`

### Negative Testing

Negative tests verify error handling:

```json
{
  "name": "Negative Test - Provider Error",
  "mock_responses": [
    {
      "pattern": "query",
      "response": "",
      "should_fail": true,
      "error_message": "Provider API error: rate limit exceeded"
    }
  ],
  "expected_outcomes": {
    "all_tasks_completed": false,
    "validation_passed": false
  },
  "tags": ["negative", "error_handling", "provider_error"]
}
```

## Implementation Details

### Mock Provider Injection

The framework injects mock providers without initializing real providers:

```rust
// Create mock provider registry
let provider_registry = ProviderRegistry::with_mock_provider(&mock_provider)?;

// Create orchestrator with injected registry (bypasses real provider init)
let orchestrator = RoutingOrchestrator::new_with_router(
    config,
    router,
    Arc::new(provider_registry)
)?;
```

### Verification Flow

1. **Execute Test**: Runner executes fixture with orchestrator
2. **Collect Results**: Captures task result, workspace state, call history
3. **Verify All Aspects**:
   - Task completion status
   - File operations and content
   - Response patterns
   - Provider call counts and patterns
   - Validation results
4. **Report**: Detailed pass/fail reporting with specific failures

### Call Tracking

The stateful mock provider tracks:
- Query text
- Matched pattern
- Response returned
- Timestamp
- Error status
- Use counts per pattern

## Current Limitations

### TypeScript Runtime Functions

Tests using TypeScript runtime functions (`readFile`, `writeFile`, etc.) are currently marked as `#[ignore]` with TODO comments. These require the TypeScript runtime to provide file operation functions.

**Affected Tests:**
- `test_simple_calculator`
- `test_file_read_write`
- `test_parallel_tasks`
- `test_sequential_dependencies`

**Solution Path:**
The TypeScript runtime needs to provide these functions as built-ins when executing agent code. The framework is ready to test these once the runtime is enhanced.

### Tool Call Verification

The `expected_tool_calls` field in mock responses is defined but not yet verified. This will require integration with tool registry tracking.

## Test Coverage

### Positive Tests
- ✅ `simple_response` - Basic query/response with no tools
- 🚧 `simple_calculator` - Task list execution with file creation (pending TS runtime)
- 🚧 `file_read_write` - File modification operations (pending TS runtime)
- 🚧 `parallel_tasks` - Independent task execution (pending TS runtime)
- 🚧 `sequential_dependencies` - Dependent task execution (pending TS runtime)

### Negative Tests
- ✅ `negative_missing_response` - Missing mock response handling
- ✅ `negative_provider_error` - Provider error injection
- ✅ `negative_insufficient_responses` - Incomplete task list responses
- ✅ `negative_excessive_calls` - Excessive call detection

## Benefits

### For Developers
1. **Confidence**: Tests use real production code paths
2. **Debugging**: Detailed verification reporting shows exactly what failed
3. **Flexibility**: Easy to add new test scenarios via JSON
4. **Coverage**: Comprehensive verification of all execution aspects

### For CI/CD
1. **Fast**: No real API calls, all mocked
2. **Reliable**: Deterministic results
3. **Isolated**: Each test gets fresh workspace
4. **Comprehensive**: Catches integration issues early

## Future Enhancements

1. **Tool Call Verification**: Verify expected tool calls were made
2. **TypeScript Runtime Integration**: Enable file operation tests
3. **Streaming Verification**: Verify UI events and streaming behavior
4. **Performance Metrics**: Track execution time and resource usage
5. **Coverage Reporting**: Integration with coverage tools
6. **Snapshot Testing**: Capture and compare full execution traces

## Contributing

When adding new tests:
1. Use JSON fixtures for maintainability
2. Provide clear descriptions
3. Add appropriate tags
4. Include both positive and negative cases
5. Verify fixtures with structure validation
6. Document any new fixture fields

## Example: Complete Test Flow

```rust
// 1. Create fixture
let fixture = E2EFixture::load("tests/fixtures/e2e/my_test.json")?;

// 2. Create runner
let mut runner = E2ERunner::new()?;

// 3. Execute test
let result = runner.execute_fixture(&fixture).await?;

// 4. Verify results
let verifier = E2EVerifier::new(&fixture, &result.workspace_root);
let verification = verifier.verify_all(&result.task_result, &result.mock_provider);

// 5. Check passed
assert!(verification.passed);
```

## Conclusion

This framework provides comprehensive E2E testing for the Merlin agent system, verifying real code paths while maintaining test speed and reliability through strategic mocking. It's designed to grow with the system and catch integration issues early in development.
