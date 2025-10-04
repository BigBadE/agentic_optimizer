# Path Resolution for Tool Calls

## Problem

Tools need absolute paths to find files, but the LLM should specify paths relative to the workspace root. Additionally, ambiguous filenames like `lib.rs` or `main.rs` exist in multiple locations, so the LLM must provide the full relative path.

## Solution

The agent requires the LLM to provide full relative paths from the workspace root, then resolves them to absolute paths before tool execution:

### How It Works

1. **System Prompt**: Instructs the LLM to always use full relative paths from workspace root
2. **Detection**: After extracting a tool call, check if params contain a `file_path` field
3. **Resolution**: Join the path with the workspace root to get absolute path
4. **Replacement**: Update the `file_path` param with the absolute path
5. **Logging**: Log the resolution for debugging

### System Prompt Instructions

The agent adds these instructions to the system prompt:

```
IMPORTANT: For file_path parameters, ALWAYS use the full relative path from the workspace root.
Examples:
- CORRECT: "crates/merlin-tools/src/lib.rs"
- CORRECT: "benchmarks/testing.md"
- WRONG: "lib.rs" (ambiguous - which lib.rs?)
- WRONG: "testing.md" (ambiguous - which directory?)
```

### Code

```rust
// In try_execute_tool_call()
if let Some(params_obj) = tool_call.input.params.as_object_mut()
    && let Some(file_path) = params_obj.get("file_path").and_then(|value| value.as_str()) {
    let path = Path::new(file_path);
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };
    let absolute_path_str = absolute_path.to_string_lossy().to_string();
    info!("Resolved path '{}' to '{}'", file_path, absolute_path.display());
    params_obj.insert("file_path".to_owned(), serde_json::json!(absolute_path_str));
}
```

### Example

**LLM generates (following instructions):**
```json
{
  "tool": "show",
  "params": {
    "file_path": "benchmarks/testing.md"
  }
}
```

**Agent resolves to:**
```json
{
  "tool": "show",
  "params": {
    "file_path": "/home/user/workspace/benchmarks/testing.md"
  }
}
```

**Log output:**
```
Resolved path 'benchmarks/testing.md' to '/home/user/workspace/benchmarks/testing.md'
```

## Benefits

1. **No Ambiguity**: Full relative paths prevent confusion between files with the same name
2. **LLM Clarity**: The LLM knows exactly which file to reference (e.g., `crates/merlin-tools/src/lib.rs` vs `crates/merlin-agent/src/lib.rs`)
3. **Reliability**: Tools always receive valid absolute paths
4. **Debugging**: Logs show exactly what path was resolved
5. **Error Messages**: Failed tool calls show the resolved absolute path, making issues obvious

## Error Messages

When a tool fails, the error now includes the resolved path:

**Before:**
```
Tool execution failed: Invalid input: File does not exist: testing.md
```

**After:**
```
Tool execution failed: Invalid input: File does not exist: /home/user/workspace/benchmarks/testing.md

Input: Object {
    "file_path": String("/home/user/workspace/benchmarks/testing.md")
}
```

This makes it immediately clear:
- The LLM provided the full relative path `benchmarks/testing.md`
- The path was resolved correctly to the absolute path
- The file genuinely doesn't exist at that location
- Not a path resolution or ambiguity issue

## Why Full Relative Paths?

### Problem with Short Paths

If the LLM could use just `"lib.rs"`, which file does it mean?
- `crates/merlin-tools/src/lib.rs`
- `crates/merlin-agent/src/lib.rs`
- `crates/merlin-context/src/lib.rs`
- `crates/merlin-core/src/lib.rs`
- `crates/merlin-languages/src/lib.rs`

The agent would have to guess or search, leading to errors.

### Solution: Explicit Paths

By requiring full relative paths from the workspace root:
- `"crates/merlin-tools/src/lib.rs"` - unambiguous
- `"crates/merlin-agent/src/lib.rs"` - unambiguous
- No guessing needed
- LLM has context about file structure from the files it sees

## Limitations

- Only resolves `file_path` parameter (not other path-like params)
- Assumes workspace root is the correct base for resolution
- Relies on LLM following instructions to provide full paths

## Future Enhancements

Could extend to:
- Resolve `working_dir` in bash tool
- Handle multiple path parameters
- Support path arrays (e.g., multiple files)
- Validate resolved paths exist before execution
- Provide helpful error if LLM uses ambiguous short path

