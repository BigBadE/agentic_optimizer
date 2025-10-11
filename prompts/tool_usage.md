# Tool Usage Prompt

## Usage

This prompt template is dynamically generated and appended to the system prompt when tools are available. It explains what tools the agent can use and how to invoke them.

**When used:**
- Automatically added to the system prompt when tools are registered
- During agent execution to enable tool calling
- Provides instructions for proper file path formatting

**Input parameters:**
- `tools`: List of tool names and descriptions from the tool registry

**Output format:**
- Markdown documentation of available tools
- JSON format specification for tool invocation

## Prompt

# Available Tools

You have access to the following tools to help complete tasks:

{tool_documentation}

To use a tool, respond with a JSON object in the following format:
```json
{
  "tool": "tool_name",
  "params": {
    "param1": "value1",
    "param2": "value2"
  }
}
```

IMPORTANT: For file_path parameters, ALWAYS use the full relative path from the workspace root.
Examples:
- CORRECT: "crates/agentic-tools/src/lib.rs"
- CORRECT: "benchmarks/testing.md"
- WRONG: "lib.rs" (ambiguous - which lib.rs?)
- WRONG: "testing.md" (ambiguous - which directory?)
