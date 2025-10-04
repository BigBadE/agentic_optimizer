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
- `command` (string): Command to execute
- `working_dir` (string, optional): Working directory for the command
- `timeout_secs` (number, optional): Timeout in seconds (default: 30)

**Example:**
```json
{
  "command": "cargo build",
  "working_dir": "/path/to/project",
  "timeout_secs": 60
}
```

## Usage

```rust
use agentic_tools::{EditTool, ShowTool, BashTool, Tool, ToolInput};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let edit_tool = EditTool::new();
    
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
Execute a shell command (bash on Unix, PowerShell on Windows). Parameters: command (string), working_dir (string, optional), timeout_secs (number, optional, default: 30)

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

