# Tool Integration in Agent

## Overview

The agent automatically includes tool descriptions in its system prompt, allowing the LLM to understand and use available tools.

## Architecture

```
AgentExecutor
├── ToolRegistry (automatically initialized)
│   ├── EditTool
│   ├── ShowTool
│   └── BashTool
└── build_system_prompt() (called during context building)
    └── Appends tool descriptions to system prompt
```

## How It Works

1. **Tool Registration**: When `AgentExecutor` is created, it initializes a `ToolRegistry` with all available tools
2. **Context Building**: When building context for a request, the executor calls `build_system_prompt()`
3. **Prompt Enhancement**: The method retrieves all registered tools and appends their descriptions to the base system prompt
4. **LLM Awareness**: The LLM receives the enhanced prompt and knows what tools are available

## System Prompt Format

The tools are appended to the base system prompt in this format:

```
<base system prompt>

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

## Usage Example

```rust
use agentic_agent::{Agent, AgentConfig, AgentRequest};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = /* your provider */;
    let agent = Agent::new(provider);
    let mut executor = agent.executor();
    
    let request = AgentRequest::new(
        "Edit the main.rs file to fix the bug",
        PathBuf::from(".")
    );
    
    // The executor will automatically include tool descriptions in the system prompt
    let result = executor.execute(request).await?;
    
    println!("Response: {}", result.response.content);
    
    Ok(())
}
```

## Adding New Tools

To add a new tool:

1. Implement the `Tool` trait in `merlin-tools`
2. Register it in `ToolRegistry::default()` in `crates/merlin-agent/src/tools.rs`
3. The tool will automatically appear in the system prompt

Example:

```rust
// In merlin-tools/src/my_tool.rs
pub struct MyTool;

impl Tool for MyTool {
    fn name(&self) -> &'static str {
        "my_tool"
    }
    
    fn description(&self) -> &'static str {
        "Description of what my tool does. Parameters: param1 (type), param2 (type)"
    }
    
    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        // Implementation
    }
}

// In merlin-agent/src/tools.rs
impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::default(),
        };
        
        registry.register(Arc::new(EditTool::default()));
        registry.register(Arc::new(ShowTool::default()));
        registry.register(Arc::new(BashTool::default()));
        registry.register(Arc::new(MyTool::default())); // Add your tool here
        
        registry
    }
}
```

