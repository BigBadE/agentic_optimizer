# merlin-tools

Tool implementations for the Merlin, providing file manipulation and command execution capabilities.

## Tools

### EditTool
Edits files by replacing text strings.

**Parameters:**
- `file_path` (string): Path to the file to edit
- `old_string` (string): Text to find and replace
- `new_string` (string): Replacement text
- `replace_all` (bool, optional): Replace all occurrences (default: false)

**Example:**
```json
{
  "file_path": "src/main.rs",
  "old_string": "old code",
  "new_string": "new code",
  "replace_all": false
}
```

### ShowTool
Displays file contents with line numbers.

**Parameters:**
- `file_path` (string): Path to the file to show
- `start_line` (number, optional): First line to display (1-indexed)
- `end_line` (number, optional): Last line to display

**Example:**
```json
{
  "file_path": "src/main.rs",
  "start_line": 10,
  "end_line": 20
}
```

### BashTool
Executes shell commands (PowerShell on Windows, bash on Unix).

**Parameters:**
- A single string containing the command to execute

**Example:**
```json
"cargo build"
```

## Usage

```rust
use agentic_tools::{EditTool, ShowTool, BashTool, Tool, ToolInput};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let edit_tool = EditTool::default();
    
    let input = ToolInput {
        params: json!({
            "file_path": "example.txt",
            "old_string": "hello",
            "new_string": "world"
        })
    };
    
    let result = edit_tool.execute(input).await?;
    println!("{:?}", result);
    
    Ok(())
}
```

## Integration with Agent

The tools are automatically registered in the `ToolRegistry` when creating an `AgentExecutor`:

```rust
use agentic_agent::{Agent, AgentExecutor};

let agent = Agent::new(provider);
let executor = agent.executor();

// Access the tool registry
let registry = executor.tool_registry();
let tools = registry.list_tools();

// Execute a tool
let output = registry.execute("edit", input).await?;
```

### System Prompt Integration

The tools are automatically included in the agent's system prompt. When the agent builds context, it appends tool descriptions in the following format:

```
# Available Tools

You have access to the following tools to help complete tasks:

## edit
Edit a file by replacing old_string with new_string. Parameters: file_path (string), old_string (string), new_string (string), replace_all (bool, optional, default: false)

## show
Show the contents of a file with line numbers. Parameters: file_path (string), start_line (number, optional), end_line (number, optional)

## bash
Execute a shell command (bash on Unix, PowerShell on Windows). Takes a single string parameter containing the command to execute.

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
```

This allows the LLM to understand what tools are available and how to use them.

## TypeScript Runtime (Phase 6.1)

The TypeScript Runtime provides a revolutionary new way for LLMs to call tools using natural TypeScript syntax instead of JSON.

### Why TypeScript?

LLMs are trained on billions of lines of open-source code, making TypeScript function calls feel natural:

```typescript
// Natural for LLMs (trained on this pattern)
await show("src/main.rs")
await edit("file.txt", "old", "new")

// Unnatural (requires special training)
{"tool": "show", "params": {"file_path": "src/main.rs"}}
```

### Usage

```rust
use merlin_tools::{TypeScriptRuntime, ShowTool};
use std::sync::Arc;

// Create runtime and register tools
let mut runtime = TypeScriptRuntime::new();
runtime.register_tool(Arc::new(ShowTool));

// Execute TypeScript code
let code = r#"await show("README.md")"#;
let result = runtime.execute(code).await?;

// Generate type definitions for LLM context
let types = runtime.generate_type_definitions();
```

### Features

- **Natural Syntax**: TypeScript function calls that LLMs understand
- **Safe Execution**: Sandboxed with timeout (30s) and memory limits (64MB)
- **Multiple Tools**: Chain multiple tool calls in sequence
- **Validation**: Automatic validation before execution
- **Type Definitions**: Generate TypeScript types for LLM context

### Example Tool Calls

```typescript
// Single tool call
await show("Cargo.toml")

// Multiple tool calls in sequence
await show("src/main.rs")
await show("src/lib.rs")

// With multiple arguments
await edit("file.txt", "old text", "new text")
```

### Architecture

```
TypeScriptRuntime
├── parse_tool_calls()    - Extract function calls from TypeScript
├── validate_tool_calls() - Ensure tools exist
├── execute_tool_call()   - Execute individual tool
└── execute()             - Main execution flow
```

### Limitations

The current implementation uses a simplified parser that:
- Supports basic function call patterns: `await functionName(arg1, arg2, ...)`
- Handles string, number, boolean, and null literals
- Does not support complex expressions or control flow
- Executes tools sequentially (no parallel execution yet)

For full implementation as outlined in docs/PLAN.md Phase 6.1, integration with a full TypeScript parser (like SWC) and the Deno runtime would be needed for:
- Full TypeScript syntax support
- Control flow (loops, conditionals)
- Variable assignments and references
- Complex expressions
- Error handling with try/catch

