# merlin-tooling

Tool system for agent actions including bash execution, file operations, and TypeScript runtime.

## Purpose

This crate provides the tool system that enables agents to interact with the filesystem, execute commands, and run TypeScript code. Tools are the bridge between LLM outputs and actual system operations.

## Module Structure

- `tool.rs` - `Tool` trait and core types
- `bash.rs` - `BashTool` for shell command execution
- `file_ops.rs` - `ReadFileTool`, `WriteFileTool`, `ListFilesTool`
- `edit_tool.rs` - `EditFileTool` for find-and-replace editing
- `delete_tool.rs` - `DeleteFileTool` for file deletion
- `context_request.rs` - `ContextRequestTool` for dynamic context requests
- `registry.rs` - `ToolRegistry` for tool management
- `runtime.rs` - `TypeScriptRuntime` for TypeScript/JavaScript execution
- `signatures.rs` - TypeScript signature generation

## Public API

**Tool Trait:**
- `Tool` - Core trait for all tools
- `ToolInput`, `ToolOutput`, `ToolError`, `ToolResult` - Core types

**Tools:**
- `BashTool` - Execute shell commands
- `ReadFileTool` - Read file contents
- `WriteFileTool` - Write file contents
- `ListFilesTool` - List directory contents
- `EditFileTool` - Find-and-replace editing
- `DeleteFileTool` - Delete files
- `ContextRequestTool` - Request additional context

**Runtime:**
- `TypeScriptRuntime` - Execute TypeScript code with tool access
- `generate_typescript_signatures()` - Generate TypeScript signatures for LLM context

**Registry:**
- `ToolRegistry` - Manage and execute tools

## Features

### Tool System
- Unified `Tool` trait for all tools
- Async execution
- JSON-based parameter passing
- Comprehensive error handling

### File Operations
- Read, write, edit, delete files
- List directory contents
- Safe file manipulation

### Command Execution
- Cross-platform shell execution (bash/PowerShell)
- Output capture
- Error handling

### TypeScript Runtime
- Execute TypeScript code in sandboxed environment
- Natural function call syntax for LLMs
- Tool integration
- Type definition generation

## Testing Status

**✅ Well-tested**

- **Unit tests**: 6 files with comprehensive coverage
  - `bash.rs`, `file_ops.rs`, `edit_tool.rs`
  - `context_request.rs`, `runtime.rs`, `signatures.rs`
- **Fixture coverage**: 15+ fixtures
  - `tools/` - Tool execution tests (delete, edit, list, show, file_size)
  - `typescript/` - TypeScript runtime tests (9+ fixtures)
    - Basic execution, async execution, agent workflows, etc.

## Code Quality

- ✅ **Documentation**: All public items have comprehensive doc comments with examples
- ✅ **Error handling**: Proper `Result<T, E>` usage throughout
- ✅ **No dead code**: All modules actively used
- ✅ **No TODOs**: Implementation complete

## Dependencies

- `merlin-core` - Core types
- `serde` / `serde_json` - Serialization
- `tokio` - Async runtime
- `rquickjs` - QuickJS runtime for TypeScript execution

## Usage Example

### Basic Tool Usage
```rust
use merlin_tooling::{EditFileTool, Tool, ToolInput};
use serde_json::json;

let tool = EditFileTool::default();
let input = ToolInput {
    params: json!({
        "file_path": "example.txt",
        "old_string": "hello",
        "new_string": "world"
    })
};

let result = tool.execute(input).await?;
```

### TypeScript Runtime
```rust
use merlin_tooling::{TypeScriptRuntime, ReadFileTool};

let mut runtime = TypeScriptRuntime::new();
runtime.register_tool(Arc::new(ReadFileTool));

let code = r#"await readFile("README.md")"#;
let result = runtime.execute(code).await?;
```

### Tool Registry
```rust
use merlin_tooling::ToolRegistry;

let registry = ToolRegistry::new();
registry.register(Arc::new(EditFileTool::default()));

let output = registry.execute("edit", input).await?;
```

## Issues and Recommendations

**None** - This crate is well-maintained with excellent test coverage, comprehensive documentation including examples, and proper error handling.
