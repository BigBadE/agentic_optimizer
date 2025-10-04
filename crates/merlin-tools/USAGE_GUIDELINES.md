# Tool Usage Guidelines for LLMs

## File Path Requirements

When calling tools that accept a `file_path` parameter, you **MUST** provide the full relative path from the workspace root.

### ✅ CORRECT Examples

```json
{
  "tool": "edit",
  "params": {
    "file_path": "crates/merlin-tools/src/lib.rs"
  }
}
```

```json
{
  "tool": "show",
  "params": {
    "file_path": "benchmarks/testing.md"
  }
}
```

```json
{
  "tool": "bash",
  "params": {
    "command": "cargo build",
    "working_dir": "crates/merlin-agent"
  }
}
```

### ❌ WRONG Examples

```json
{
  "tool": "edit",
  "params": {
    "file_path": "lib.rs"  // ❌ Which lib.rs? There are many!
  }
}
```

```json
{
  "tool": "show",
  "params": {
    "file_path": "testing.md"  // ❌ Which directory is it in?
  }
}
```

```json
{
  "tool": "edit",
  "params": {
    "file_path": "src/main.rs"  // ❌ Which crate's main.rs?
  }
}
```

## Why Full Paths?

Many projects have files with the same name in different locations:
- `crates/merlin-tools/src/lib.rs`
- `crates/merlin-agent/src/lib.rs`
- `crates/merlin-context/src/lib.rs`
- `crates/merlin-core/src/lib.rs`

Using just `"lib.rs"` is ambiguous and will cause errors.

## How to Find the Correct Path

1. Look at the context files provided - they show full paths
2. Use the file tree structure to determine the path
3. Always start from the workspace root
4. Use forward slashes `/` even on Windows

## Special Cases

### Files at Workspace Root

If a file is at the root of the workspace, just use the filename:

```json
{
  "tool": "show",
  "params": {
    "file_path": "README.md"  // ✅ At workspace root
  }
}
```

### Nested Directories

Always include the full directory structure:

```json
{
  "tool": "edit",
  "params": {
    "file_path": "crates/merlin-languages/languages/rust-backend/src/lib.rs"
  }
}
```

## Tool-Specific Guidelines

### edit Tool

```json
{
  "tool": "edit",
  "params": {
    "file_path": "crates/merlin-agent/src/executor.rs",  // Full path required
    "old_string": "text to replace",
    "new_string": "replacement text",
    "replace_all": false  // Optional
  }
}
```

### show Tool

```json
{
  "tool": "show",
  "params": {
    "file_path": "crates/merlin-tools/src/tool.rs",  // Full path required
    "start_line": 10,  // Optional
    "end_line": 20     // Optional
  }
}
```

### bash Tool

```json
{
  "tool": "bash",
  "params": {
    "command": "cargo test",
    "working_dir": "crates/merlin-agent",  // Full path from root
    "timeout_secs": 60  // Optional
  }
}
```

## Error Messages

If you provide an ambiguous or incorrect path, you'll see an error like:

```
Tool execution failed: Invalid input: File does not exist: /workspace/lib.rs

Input: Object {
    "file_path": String("/workspace/lib.rs")
}
```

This tells you:
1. The exact path that was tried
2. The file doesn't exist at that location
3. You need to provide the full relative path

## Summary

**Always use full relative paths from the workspace root for file_path parameters.**

This ensures:
- No ambiguity between files with the same name
- Tools can find the correct file
- Error messages are clear and helpful

