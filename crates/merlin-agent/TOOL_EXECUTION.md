# Tool Call Execution Flow

## Overview

The agent now automatically detects and executes tool calls from the AI's response. When the LLM responds with a tool call in JSON format, the agent intercepts it, executes the tool, and returns the result.

## How It Works

### 1. System Prompt Enhancement
When building context, the agent appends tool descriptions to the system prompt:

```
# Available Tools

## edit
Edit a file by replacing old_string with new_string...

## show
Show the contents of a file with line numbers...

## bash
Execute a shell command...

To use a tool, respond with a JSON object in the following format:
```json
{
  "tool": "tool_name",
  "params": {
    "param1": "value1"
  }
}
```
```

### 2. Response Interception
After the LLM generates a response, the agent checks if it contains a tool call:

```rust
// In AgentExecutor::execute()
let mut response = self.provider.generate(&query, &context).await?;

// Check if response contains a tool call and execute it
if let Some(tool_result) = self.try_execute_tool_call(&response.text).await {
    info!("Tool call detected and executed");
    response.text = tool_result;
}
```

### 3. Tool Call Extraction
The `extract_tool_call()` method parses the response to find JSON tool calls:

- Looks for JSON in markdown code blocks: ` ```json ... ``` `
- Falls back to finding JSON objects: `{ ... }`
- Parses the JSON to extract `tool` name and `params`

### 3.5. Path Resolution
Before executing the tool, file paths are resolved to absolute paths:

- The LLM is instructed to provide full relative paths from workspace root
- Example: `"benchmarks/testing.md"` not just `"testing.md"`
- The agent resolves these to absolute paths: `"/path/to/workspace/benchmarks/testing.md"`
- This prevents ambiguity with common filenames like `lib.rs` or `main.rs`

The system prompt explicitly instructs:
```
IMPORTANT: For file_path parameters, ALWAYS use the full relative path from the workspace root.
Examples:
- CORRECT: "crates/merlin-tools/src/lib.rs"
- CORRECT: "benchmarks/testing.md"
- WRONG: "lib.rs" (ambiguous - which lib.rs?)
- WRONG: "testing.md" (ambiguous - which directory?)
```

### 4. Tool Execution
Once a tool call is detected:

```rust
match self.tool_registry.execute(&tool_call.tool, tool_call.input).await {
    Ok(output) => {
        if output.success {
            format!("Tool '{}' executed successfully:\n{}\n\nData: {:?}", 
                tool_call.tool, output.message, output.data)
        } else {
            format!("Tool '{}' failed:\n{}", tool_call.tool, output.message)
        }
    }
    Err(error) => {
        format!("Tool execution failed: {error}")
    }
}
```

### 5. Result Replacement
The tool execution result replaces the original LLM response, so the user sees:
- The tool execution status
- Any output or error messages
- Data returned by the tool
- **On failure**: The input parameters that were used (for debugging)

## Example Flow

**User Request:** "Edit main.rs to add a println statement"

**LLM Response:**
```json
{
  "tool": "edit",
  "params": {
    "file_path": "crates/merlin-cli/src/main.rs",
    "old_string": "fn main() {",
    "new_string": "fn main() {\n    println!(\"Hello, world!\");"
  }
}
```

**Agent Action:**
1. Detects tool call in response
2. Extracts tool name: `"edit"`
3. Extracts params as JSON
4. Resolves `"crates/merlin-cli/src/main.rs"` to absolute path
5. Calls `EditTool::execute()` with resolved params
6. EditTool reads file, performs replacement, writes file
7. Returns success/failure message

**Final Response to User:**
```
Tool 'edit' executed successfully:
File edited successfully: /workspace/crates/merlin-cli/src/main.rs

Data: None
```

## Supported Tool Call Formats

### Format 1: Markdown Code Block
```
Here's how to fix it:

```json
{
  "tool": "edit",
  "params": {
    "file_path": "benchmarks/test.txt",
    "old_string": "old",
    "new_string": "new"
  }
}
```
```

### Format 2: Plain JSON
```
{
  "tool": "show",
  "params": {
    "file_path": "README.md",
    "start_line": 1,
    "end_line": 10
  }
}
```

**Note:** In Format 2, `README.md` is at the workspace root, so the full relative path is just `README.md`.

## Error Handling

- **Tool not found:** Returns error message to user
- **Invalid JSON:** Tool call is ignored, original response returned
- **Tool execution fails:** Error message with input params returned to user
- **Missing required params:** Tool returns validation error

### Error Output Example

When a tool fails, the user sees the input that was used:

```
Tool execution failed: Invalid input: File does not exist: /workspace/testing.md

Input: Object {
    "file_path": String("/workspace/testing.md"),
    "start_line": Number(1),
    "end_line": Number(10)
}
```

This helps debug issues like:
- Wrong file paths (now shows the resolved absolute path)
- Missing parameters
- Invalid parameter values

## Adding New Tools

To add a new tool that will be automatically intercepted:

1. Implement the `Tool` trait in `merlin-tools`
2. Register it in `ToolRegistry::new()`
3. The tool will automatically appear in the system prompt
4. Tool calls will be intercepted and executed

No changes needed to the interception logic!

