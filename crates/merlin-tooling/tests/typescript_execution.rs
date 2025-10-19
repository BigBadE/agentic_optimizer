//! Tests for TypeScript code extraction and execution
#![cfg_attr(
    test,
    allow(
        dead_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

use merlin_tooling::{BashTool, TypeScriptRuntime};
use std::sync::Arc;

/// Helper function to extract TypeScript code from model response
/// Mimics the extraction logic in `AgentExecutor`
fn extract_typescript_code(text: &str) -> Option<String> {
    let start_marker = "```typescript";
    let end_marker = "```";

    let start_idx = text.find(start_marker)?;
    let code_start = start_idx + start_marker.len();
    let remaining = &text[code_start..];
    let end_idx = remaining.find(end_marker)?;

    let code = remaining[..end_idx].trim();
    Some(code.to_owned())
}

#[test]
fn test_extract_typescript_code_simple() {
    let response = r#"
I'll execute the task:

```typescript
return { done: true, result: "Success" };
```
"#;

    let code = extract_typescript_code(response);
    assert!(code.is_some());
    assert_eq!(
        code.unwrap(),
        r#"return { done: true, result: "Success" };"#
    );
}

#[test]
fn test_extract_typescript_code_with_explanation() {
    let response = r"
Let me solve this step by step.

First, I'll calculate the value:

```typescript
const x = 1 + 1;
return { done: true, result: `Result is ${x}` };
```

This completes the task.
";

    let code = extract_typescript_code(response);
    assert!(code.is_some());
    let extracted = code.unwrap();
    assert!(extracted.contains("const x = 1 + 1"));
    assert!(extracted.contains("return { done: true"));
}

#[test]
fn test_extract_typescript_code_multiple_blocks() {
    let response = r#"
First approach:

```typescript
return { done: true, result: "First" };
```

Alternative approach:

```typescript
return { done: true, result: "Second" };
```
"#;

    // Should extract first block
    let code = extract_typescript_code(response);
    assert!(code.is_some());
    assert!(code.unwrap().contains("First"));
}

#[test]
fn test_extract_typescript_code_no_block() {
    let response = "This is just text without any code blocks";
    let code = extract_typescript_code(response);
    assert!(code.is_none());
}

#[tokio::test]
async fn test_typescript_runtime_done_result() {
    let bash_tool = Arc::new(BashTool);
    let mut runtime = TypeScriptRuntime::new();

    // Register bash tool
    runtime.register_tool(bash_tool);

    // Execute TypeScript code that returns a string result
    let code = r#"function agent_code() { return "Task completed"; }"#;
    let result = runtime.execute(code).await;

    if let Err(error) = &result {
        panic!("Execution failed: {error:?}");
    }
    let value = result.unwrap();

    // Verify the result is a string
    assert_eq!(value.as_str(), Some("Task completed"));
}

#[tokio::test]
async fn test_typescript_runtime_with_computation() {
    let bash_tool = Arc::new(BashTool);
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(bash_tool);

    // Execute TypeScript code with computation
    let code = r"function agent_code() {
        const x = 5 + 10;
        return `Result: ${x}`;
    }";
    let result = runtime.execute(code).await;

    if let Err(error) = &result {
        panic!("Execution failed: {error:?}");
    }
    let value = result.unwrap();

    assert_eq!(value.as_str(), Some("Result: 15"));
}

#[tokio::test]
async fn test_typescript_runtime_simple_computation() {
    let runtime = TypeScriptRuntime::new();

    // Test simple computation
    let code = r"
function agent_code() {
    const a = 5;
    const b = 10;
    const sum = a + b;
    return `Sum is ${sum}`;
}
";
    let result = runtime.execute(code).await;

    if let Err(error) = &result {
        panic!("Execution failed: {error:?}");
    }
    let value = result.unwrap();
    assert_eq!(value.as_str(), Some("Sum is 15"));
}

#[tokio::test]
async fn test_typescript_runtime_conditional_logic() {
    let runtime = TypeScriptRuntime::new();

    // Test conditional logic
    let code = r"
function agent_code() {
    const value = 42;
    const result = value > 40 ? 'High' : 'Low';
    return result;
}
";
    let result = runtime.execute(code).await;

    if let Err(error) = &result {
        panic!("Execution failed: {error:?}");
    }
    let value = result.unwrap();
    assert_eq!(value.as_str(), Some("High"));
}

#[tokio::test]
async fn test_typescript_runtime_array_operations() {
    let runtime = TypeScriptRuntime::new();

    // Test array operations
    let code = r"
function agent_code() {
    const numbers = [1, 2, 3, 4, 5];
    const sum = numbers.reduce((a, b) => a + b, 0);
    return `Sum is ${sum}`;
}
";
    let result = runtime.execute(code).await;

    if let Err(error) = &result {
        panic!("Execution failed: {error:?}");
    }
    let value = result.unwrap();
    assert_eq!(value.as_str(), Some("Sum is 15"));
}

#[tokio::test]
async fn test_typescript_runtime_error_handling() {
    let runtime = TypeScriptRuntime::new();

    // Test syntax error
    let code = r"
const x = {
return { done: true };
";
    let result = runtime.execute(code).await;

    result.unwrap_err();
}
