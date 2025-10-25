# Task List Test Fixtures

This directory contains JSON fixtures for end-to-end testing of task decomposition and execution.

## Purpose

These fixtures enable comprehensive testing of the agent's task list functionality without requiring real API calls. Each fixture defines:

- An initial user query
- Mock agent responses (as TypeScript code) for different query patterns
- Expected task list structure
- Expected outcomes (files created, tests passed, etc.)

## How It Works

1. **Initial Query**: The user's task request (e.g., "Implement a calculator")
2. **Mock Response**: The agent returns TypeScript code that either:
   - Returns a `TaskList` object (for task decomposition)
   - Returns a `string` result (for executing a single task step)
3. **Tool Execution**: The TypeScript code calls tools like `bash()` to perform actual work
4. **Verification**: The test verifies the task list structure and outcomes

## Fixture Format

```json
{
  "name": "Test Scenario Name",
  "description": "What this test verifies",
  "initial_query": "User's initial request to the agent",
  "mock_responses": [
    {
      "pattern": "substring to match in query",
      "response": [
        "```typescript",
        "async function agent_code(): Promise<TaskList> {",
        "  return { ... };",
        "}",
        "```"
      ]
    },
    {
      "pattern": "Create some file",
      "response": [
        "```typescript",
        "async function agent_code(): Promise<string> {",
        "  await bash('echo content > file.rs');",
        "  return 'Created file';",
        "}",
        "```"
      ]
    }
  ],
  "expected_task_list": {
    "total_tasks": 3,
    "task_descriptions": [
      "Description of task 1",
      "Description of task 2",
      "Description of task 3"
    ],
    "dependency_chain": [
      [],          // Task 1 has no dependencies
      ["task_1"],  // Task 2 depends on task_1
      ["task_2"]   // Task 3 depends on task_2
    ]
  },
  "expected_outcomes": {
    "all_tasks_completed": true,
    "files_created": ["path/to/file1.rs", "path/to/file2.rs"],
    "tests_passed": true
  }
}
```

## Mock Response Format

All mock responses must be valid TypeScript code as an **array of strings** (one per line), wrapped in triple backticks.

**Important**: The `response` field is an array of strings, not a single string. Each line of code is a separate array element. This makes the fixtures much easier to read and edit.

### Task Decomposition Response

Returns a `TaskList` object to decompose a complex task into steps:

```json
{
  "pattern": "implement feature",
  "response": [
    "```typescript",
    "async function agent_code(): Promise<TaskList> {",
    "  return {",
    "    id: \"unique_id\",",
    "    title: \"Task title\",",
    "    steps: [",
    "      {",
    "        id: \"step_1\",",
    "        step_type: \"Feature\",",
    "        description: \"Do something\",",
    "        verification: \"Verify it works\"",
    "      },",
    "      {",
    "        id: \"step_2\",",
    "        step_type: \"Test\",",
    "        description: \"Do another thing\",",
    "        verification: \"Tests pass\"",
    "      }",
    "    ]",
    "  };",
    "}",
    "```"
  ]
}
```

**Note**: The `status` field is optional for both `TaskList` and `TaskStep` - it defaults to `"NotStarted"` and `"Pending"` respectively if not specified.

### Task Execution Response

Returns a string result after executing tools:

```json
{
  "pattern": "Create calculator file",
  "response": [
    "```typescript",
    "async function agent_code(): Promise<string> {",
    "  const code = `pub fn add(a: i32, b: i32) -> i32 {",
    "    a + b",
    "}`;",
    "  await bash(`echo '${code}' > src/calc.rs`);",
    "  return \"Created calculator file\";",
    "}",
    "```"
  ]
}
```

## Tool Usage in Fixtures

Fixtures should demonstrate realistic tool usage:

### Bash Tool

The primary tool for file operations and command execution:

```typescript
// Read files
const content = await bash("cat src/main.rs");

// Write files
await bash(`echo 'code here' > src/file.rs`);

// Append to files
await bash(`echo 'more code' >> src/file.rs`);

// Run commands
const result = await bash("cargo test");
if (result.exit_code === 0) {
  return "Tests passed";
}

// Search for patterns
const matches = await bash("grep -r 'pattern' src/");

// Create directories
await bash("mkdir -p tests/integration");
```

## Example Fixtures

### simple_implementation.json

Tests basic task decomposition for a simple feature:
1. **Initial query**: "Implement a simple calculator function"
2. **Agent response**: Returns `TaskList` with 3 steps
3. **Step execution**: Each step uses `bash()` to create files and run tests
4. **Verification**: Checks task structure and that files were created

### parallel_tasks.json

Tests parallel task execution with dependencies:
1. **Initial query**: "Create a web API with user and product endpoints"
2. **Agent response**: Returns `TaskList` with 2 independent tasks (user model, product model)
3. **Dependencies**: Router task depends on both models
4. **Verification**: Ensures parallel tasks can run concurrently

## Adding New Fixtures

1. Create a new `.json` file in this directory
2. Follow the format above with TypeScript code blocks
3. Include realistic `bash()` tool calls
4. Add corresponding test in `tests/task_list_e2e.rs`
5. Verify with `cargo test --test task_list_e2e`

## Best Practices

- **Use TypeScript code blocks**: All responses must be wrapped in ```typescript```
- **Include tool calls**: Show realistic `bash()` usage
- **Be specific**: Use unique patterns that won't accidentally match other queries
- **Test edge cases**: Include fixtures for error scenarios, empty task lists
- **Keep it realistic**: Mock responses should resemble actual agent output
- **Document intent**: Use clear names and descriptions

## Testing the Fixtures

The fixture runner will:

1. Load the JSON fixture
2. Create a `MockProvider` with the mock responses
3. Send the initial query to the agent
4. Execute the TypeScript code returned by the mock
5. Verify the task list structure matches expected
6. Optionally verify tool calls and outcomes

This allows full end-to-end testing of the agent system without real API calls or model inference.
