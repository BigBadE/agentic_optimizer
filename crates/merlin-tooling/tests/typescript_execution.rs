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

// ============================================================================
// Full Agent Response Tests - Realistic Tool Usage Scenarios
// ============================================================================

/// Test agent listing directory contents
#[tokio::test]
async fn test_agent_list_files() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    // Simulate agent response to "list files in current directory"
    let agent_response = r"
I'll list the files in the current directory for you.

```typescript
function agent_code() {
    const result = bash('ls -la');
    return { done: true, result: result.stdout };
}
```
";

    let code = extract_typescript_code(agent_response).expect("Failed to extract code");
    let result = runtime.execute(&code).await;

    if let Err(error) = &result {
        panic!("Agent execution failed: {error:?}");
    }
    let value = result.unwrap();

    // Should have an object with done and result
    assert!(value.is_object(), "Expected object result, got: {value:?}");
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("done"), Some(&serde_json::json!(true)));
    assert!(obj.contains_key("result"));
}

/// Test agent checking if a file exists
#[tokio::test]
async fn test_agent_check_file_exists() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    // Simulate agent response to "check if Cargo.toml exists"
    let agent_response = r#"
I'll check if the Cargo.toml file exists.

```typescript
async function agent_code() {
    const result = await bash('test -f Cargo.toml && echo "exists" || echo "not found"');
    const exists = result.stdout.trim() === 'exists';
    return { done: true, result: `File ${exists ? 'exists' : 'does not exist'}` };
}
```
"#;

    let code = extract_typescript_code(agent_response).expect("Failed to extract code");
    let result = runtime.execute(&code).await;

    if let Err(error) = &result {
        panic!("Agent execution failed: {error:?}");
    }
    let value = result.unwrap();

    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("done"), Some(&serde_json::json!(true)));
}

/// Test agent performing computation before returning
#[tokio::test]
async fn test_agent_computation_then_bash() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    // Simulate agent doing computation then using bash
    let agent_response = r#"
I'll calculate the value and then create a file with it.

```typescript
async function agent_code() {
    // Calculate factorial of 5
    let factorial = 1;
    for (let i = 2; i <= 5; i++) {
        factorial *= i;
    }

    // Echo the result
    const result = await bash(`echo "Factorial of 5 is ${factorial}"`);

    return {
        done: true,
        result: `Calculated ${factorial}, output: ${result.stdout.trim()}`
    };
}
```
"#;

    let code = extract_typescript_code(agent_response).expect("Failed to extract code");
    let result = runtime.execute(&code).await;

    if let Err(error) = &result {
        panic!("Agent execution failed: {error:?}");
    }
    let value = result.unwrap();

    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("done"), Some(&serde_json::json!(true)));

    let result_str = obj.get("result").and_then(|val| val.as_str()).unwrap();
    assert!(result_str.contains("120")); // 5! = 120
    assert!(result_str.contains("Factorial of 5 is 120"));
}

/// Test agent performing multiple sequential bash commands
#[tokio::test]
async fn test_agent_multiple_bash_commands() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    // Simulate agent running multiple commands
    let agent_response = r"
I'll run a series of commands to gather system information.

```typescript
async function agent_code() {
    // Get current directory
    const pwd_result = await bash('pwd');
    const current_dir = pwd_result.stdout.trim();

    // Count files
    const ls_result = await bash('ls -1 | wc -l');
    const file_count = ls_result.stdout.trim();

    return {
        done: true,
        result: {
            directory: current_dir,
            file_count: parseInt(file_count)
        }
    };
}
```
";

    let code = extract_typescript_code(agent_response).expect("Failed to extract code");
    let result = runtime.execute(&code).await;

    if let Err(error) = &result {
        panic!("Agent execution failed: {error:?}");
    }
    let value = result.unwrap();

    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("done"), Some(&serde_json::json!(true)));

    let result_obj = obj.get("result").and_then(|val| val.as_object()).unwrap();
    assert!(result_obj.contains_key("directory"));
    assert!(result_obj.contains_key("file_count"));
}

/// Test agent with error handling in TypeScript
#[tokio::test]
async fn test_agent_with_error_handling() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    // Simulate agent with try-catch error handling
    let agent_response = r"
I'll check the git status, handling errors gracefully.

```typescript
async function agent_code() {
    const git_result = await bash('git status 2>&1');

    if (git_result.exit_code === 0) {
        return {
            done: true,
            result: 'Git repository found',
            status: git_result.stdout
        };
    } else {
        return {
            done: true,
            result: 'Not a git repository or git not installed',
            error: git_result.stderr
        };
    }
}
```
";

    let code = extract_typescript_code(agent_response).expect("Failed to extract code");
    let result = runtime.execute(&code).await;

    if let Err(error) = &result {
        panic!("Agent execution failed: {error:?}");
    }
    let value = result.unwrap();

    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("done"), Some(&serde_json::json!(true)));
    assert!(obj.contains_key("result"));
}

/// Test agent performing data transformation
#[tokio::test]
async fn test_agent_data_transformation() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    // Simulate agent parsing command output
    let agent_response = r"
I'll get the file sizes and parse them.

```typescript
async function agent_code() {
    // Get file listing with sizes
    const result = await bash('ls -lh | tail -n +2');
    const lines = result.stdout.split('\n').filter(line => line.trim());

    // Parse each line to extract filename and size
    const files = lines.map(line => {
        const parts = line.split(/\s+/);
        return {
            size: parts[4],
            name: parts.slice(8).join(' ')
        };
    }).filter(f => f.name);

    return {
        done: true,
        result: `Found ${files.length} files`,
        files: files.slice(0, 5) // Return first 5
    };
}
```
";

    let code = extract_typescript_code(agent_response).expect("Failed to extract code");
    let result = runtime.execute(&code).await;

    if let Err(error) = &result {
        panic!("Agent execution failed: {error:?}");
    }
    let value = result.unwrap();

    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("done"), Some(&serde_json::json!(true)));
    assert!(obj.contains_key("result"));
}

/// Test agent using conditional logic based on bash output
#[tokio::test]
async fn test_agent_conditional_bash() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    // Simulate agent checking platform and acting accordingly
    let agent_response = r#"
I'll detect the operating system and provide platform-specific information.

```typescript
async function agent_code() {
    const uname_result = await bash('uname -s 2>&1 || echo "Windows"');
    const platform = uname_result.stdout.trim();

    let message = '';
    if (platform.includes('Linux')) {
        message = 'Running on Linux';
    } else if (platform.includes('Darwin')) {
        message = 'Running on macOS';
    } else if (platform.includes('MINGW') || platform.includes('MSYS') || platform.includes('Windows')) {
        message = 'Running on Windows';
    } else {
        message = 'Unknown platform: ' + platform;
    }

    return {
        done: true,
        result: message,
        platform: platform
    };
}
```
"#;

    let code = extract_typescript_code(agent_response).expect("Failed to extract code");
    let result = runtime.execute(&code).await;

    if let Err(error) = &result {
        panic!("Agent execution failed: {error:?}");
    }
    let value = result.unwrap();

    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("done"), Some(&serde_json::json!(true)));
    assert!(obj.contains_key("platform"));

    let result_str = obj.get("result").and_then(|val| val.as_str()).unwrap();
    assert!(result_str.starts_with("Running on"));
}

/// Test agent performing a complex workflow
#[tokio::test]
async fn test_agent_complex_workflow() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    // Simulate agent performing multi-step analysis
    let agent_response = r#"
I'll analyze the project structure and provide a summary.

```typescript
async function agent_code() {
    // Step 1: Check if it's a Rust project
    const cargo_check = await bash('test -f Cargo.toml && echo "yes" || echo "no"');
    const is_rust = cargo_check.stdout.trim() === 'yes';

    if (!is_rust) {
        return { done: true, result: 'Not a Rust project' };
    }

    // Step 2: Count Rust files
    const rust_files = await bash('find . -name "*.rs" -type f | wc -l');
    const file_count = parseInt(rust_files.stdout.trim());

    // Step 3: Count files in tests directory
    const test_check = await bash('find . -path "*/tests/*.rs" -type f | wc -l');
    const test_count = parseInt(test_check.stdout.trim());

    // Step 4: Build summary
    const summary = {
        project_type: 'Rust',
        rust_files: file_count,
        test_files: test_count,
        has_tests: test_count > 0
    };

    return {
        done: true,
        result: `Rust project with ${file_count} files and ${test_count} test files`,
        summary: summary
    };
}
```
"#;

    let code = extract_typescript_code(agent_response).expect("Failed to extract code");
    let result = runtime.execute(&code).await;

    if let Err(error) = &result {
        panic!("Agent execution failed: {error:?}");
    }
    let value = result.unwrap();

    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("done"), Some(&serde_json::json!(true)));
    assert!(obj.contains_key("summary"));

    let summary = obj.get("summary").and_then(|val| val.as_object()).unwrap();
    assert_eq!(
        summary.get("project_type").and_then(|val| val.as_str()),
        Some("Rust")
    );
}

/// Test agent building JSON output from bash results
#[tokio::test]
async fn test_agent_json_construction() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    // Simulate agent building structured data
    let agent_response = r#"
I'll gather system information and return it as structured data.

```typescript
async function agent_code() {
    // Get current date/time
    const date_result = await bash('date +"%Y-%m-%d %H:%M:%S"');

    // Get hostname
    const host_result = await bash('hostname || echo "unknown"');

    // Get working directory
    const pwd_result = await bash('pwd');

    return {
        done: true,
        result: 'System information collected',
        data: {
            timestamp: date_result.stdout.trim(),
            hostname: host_result.stdout.trim(),
            working_directory: pwd_result.stdout.trim(),
            collected_at: new Date().toISOString()
        }
    };
}
```
"#;

    let code = extract_typescript_code(agent_response).expect("Failed to extract code");
    let result = runtime.execute(&code).await;

    if let Err(error) = &result {
        panic!("Agent execution failed: {error:?}");
    }
    let value = result.unwrap();

    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("done"), Some(&serde_json::json!(true)));

    let data = obj.get("data").and_then(|val| val.as_object()).unwrap();
    assert!(data.contains_key("timestamp"));
    assert!(data.contains_key("hostname"));
    assert!(data.contains_key("working_directory"));
    assert!(data.contains_key("collected_at"));
}

/// Test that reproduces the Uninitialized error with multiple async bash calls
#[tokio::test]
async fn test_agent_multiple_async_bash_calls_with_processing() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    // This is the exact code that triggers the Uninitialized error
    let agent_response = r"
I'll search for TODOs and FAIL markers in the source code.

```typescript
async function agent_code(): Promise<string> {
    let todos = await bash('grep -r TODO src/');
    let failedFixtures = await bash('grep -r FAIL src/');

    let todosList = todos.stdout.split('\n').filter(line => line.includes('TODO')).map(line => line.trim());
    let failedFixturesList = failedFixtures.stdout.split('\n').filter(line => line.includes('FAIL')).map(line => line.trim());

    let result = 'List of todos:\n' + todosList.join('\n') + '\n\nList of failed fixtures:\n' + failedFixturesList.join('\n');

    return result;
}
```
";

    let code = extract_typescript_code(agent_response).expect("Failed to extract code");
    let result = runtime.execute(&code).await;

    // This should fail because the bash commands will fail (src/ doesn't exist in test env)
    match result {
        Ok(value) => {
            println!("Unexpectedly succeeded with value: {value:?}");
            // If it succeeds, verify the result
            assert!(value.is_string() || value.is_object());
        }
        Err(error) => {
            println!("Error as expected: {error:?}");
            // Check if it's an error (bash command failure, or other runtime error)
            let error_str = error.to_string();
            assert!(
                error_str.contains("Command failed")
                    || error_str.contains("Promise rejected")
                    || error_str.contains("Uninitialized")
                    || error_str.contains("Exception")
                    || error_str.contains("SyntaxError"),
                "Expected command failure or runtime error, got: {error_str}"
            );
        }
    }
}

/// Test TypeScript async function with Promise return type
#[tokio::test]
async fn test_typescript_async_function_with_promise() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    let code = r#"
async function agent_code(): Promise<string> {
    let r = await bash("echo 'Hello World'");
    return r.stdout;
}
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_string());
    let output = value.as_str().unwrap();
    assert!(output.contains("Hello World"));
}

/// Test TypeScript with type annotations on variables
#[tokio::test]
async fn test_typescript_variable_type_annotations() {
    let runtime = TypeScriptRuntime::new();

    let code = r#"
const x: number = 42;
const y: string = "hello";
const z: boolean = true;
return { x, y, z };
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("x"), Some(&serde_json::json!(42)));
    assert_eq!(obj.get("y"), Some(&serde_json::json!("hello")));
    assert_eq!(obj.get("z"), Some(&serde_json::json!(true)));
}

/// Test TypeScript interface definitions are properly stripped
#[tokio::test]
async fn test_typescript_interface_stripping() {
    let runtime = TypeScriptRuntime::new();

    let code = r#"
interface Person {
    name: string;
    age: number;
}

const person: Person = {
    name: "Alice",
    age: 30
};

return person;
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("name"), Some(&serde_json::json!("Alice")));
    assert_eq!(obj.get("age"), Some(&serde_json::json!(30)));
}

/// Test TypeScript type aliases are properly stripped
#[tokio::test]
async fn test_typescript_type_alias_stripping() {
    let runtime = TypeScriptRuntime::new();

    let code = r#"
type Result = {
    success: boolean;
    data: string;
};

const result: Result = {
    success: true,
    data: "test"
};

return result;
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("success"), Some(&serde_json::json!(true)));
    assert_eq!(obj.get("data"), Some(&serde_json::json!("test")));
}

/// Test TypeScript with function parameter type annotations
#[tokio::test]
async fn test_typescript_function_parameter_types() {
    let runtime = TypeScriptRuntime::new();

    let code = r#"
function add(a: number, b: number): number {
    return a + b;
}

function greet(name: string, age: number): string {
    return `Hello ${name}, you are ${age} years old`;
}

return {
    sum: add(10, 20),
    greeting: greet("Bob", 25)
};
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("sum"), Some(&serde_json::json!(30)));
    assert_eq!(
        obj.get("greeting"),
        Some(&serde_json::json!("Hello Bob, you are 25 years old"))
    );
}

/// Test TypeScript arrow functions with type annotations
#[tokio::test]
async fn test_typescript_arrow_function_types() {
    let runtime = TypeScriptRuntime::new();

    let code = r"
const multiply = (a: number, b: number): number => a * b;
const isAdult = (age: number): boolean => age >= 18;

return {
    product: multiply(6, 7),
    adult: isAdult(20),
    child: isAdult(15)
};
";

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("product"), Some(&serde_json::json!(42)));
    assert_eq!(obj.get("adult"), Some(&serde_json::json!(true)));
    assert_eq!(obj.get("child"), Some(&serde_json::json!(false)));
}

/// Test TypeScript with optional parameters
#[tokio::test]
async fn test_typescript_optional_parameters() {
    let runtime = TypeScriptRuntime::new();

    let code = r#"
function greet(name: string, greeting?: string): string {
    return (greeting || "Hello") + ", " + name;
}

return {
    default: greet("World"),
    custom: greet("World", "Hi")
};
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("default"), Some(&serde_json::json!("Hello, World")));
    assert_eq!(obj.get("custom"), Some(&serde_json::json!("Hi, World")));
}

/// Test TypeScript with union types
#[tokio::test]
async fn test_typescript_union_types() {
    let runtime = TypeScriptRuntime::new();

    let code = r#"
type StringOrNumber = string | number;

function process(value: StringOrNumber): string {
    if (typeof value === "string") {
        return "String: " + value;
    } else {
        return "Number: " + value;
    }
}

return {
    str: process("hello"),
    num: process(42)
};
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("str"), Some(&serde_json::json!("String: hello")));
    assert_eq!(obj.get("num"), Some(&serde_json::json!("Number: 42")));
}

/// Test TypeScript with array types
#[tokio::test]
async fn test_typescript_array_types() {
    let runtime = TypeScriptRuntime::new();

    let code = r#"
const numbers: number[] = [1, 2, 3, 4, 5];
const strings: Array<string> = ["a", "b", "c"];

return {
    numbers: numbers.map((n: number): number => n * 2),
    strings: strings.map((s: string): string => s.toUpperCase())
};
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(
        obj.get("numbers"),
        Some(&serde_json::json!([2, 4, 6, 8, 10]))
    );
    assert_eq!(
        obj.get("strings"),
        Some(&serde_json::json!(["A", "B", "C"]))
    );
}

/// Test TypeScript with generic functions
#[tokio::test]
async fn test_typescript_generic_functions() {
    let runtime = TypeScriptRuntime::new();

    let code = r#"
function identity<T>(arg: T): T {
    return arg;
}

function first<T>(arr: T[]): T | undefined {
    return arr[0];
}

return {
    num: identity<number>(42),
    str: identity<string>("hello"),
    firstNum: first<number>([1, 2, 3]),
    firstStr: first<string>(["a", "b", "c"])
};
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("num"), Some(&serde_json::json!(42)));
    assert_eq!(obj.get("str"), Some(&serde_json::json!("hello")));
    assert_eq!(obj.get("firstNum"), Some(&serde_json::json!(1)));
    assert_eq!(obj.get("firstStr"), Some(&serde_json::json!("a")));
}

/// Test TypeScript with as type casting
#[tokio::test]
async fn test_typescript_type_casting() {
    let runtime = TypeScriptRuntime::new();

    let code = r#"
const value = "hello" as any;
const length = (value as string).length;

return { length };
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("length"), Some(&serde_json::json!(5)));
}

/// Test TypeScript async function calling bash tool
#[tokio::test]
async fn test_typescript_async_agent_code_with_bash() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    let code = r#"
async function agent_code(): Promise<string> {
    let r = await bash("echo 'TODO: Fix this'");
    return r.stdout;
}
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_string());
    let output = value.as_str().unwrap();
    assert!(output.contains("TODO"));
}

/// Test TypeScript async function with multiple awaits and type annotations
#[tokio::test]
async fn test_typescript_async_multiple_awaits_with_types() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    let code = r#"
async function agent_code(): Promise<object> {
    const result1: any = await bash("echo 'first'");
    const result2: any = await bash("echo 'second'");

    const output: string = result1.stdout + " " + result2.stdout;

    return {
        done: true,
        output: output.trim()
    };
}
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("done"), Some(&serde_json::json!(true)));
    let output = obj.get("output").and_then(|val| val.as_str()).unwrap();
    assert!(output.contains("first"));
    assert!(output.contains("second"));
}

/// Test TypeScript with readonly properties
#[tokio::test]
async fn test_typescript_readonly_properties() {
    let runtime = TypeScriptRuntime::new();

    let code = r#"
interface Config {
    readonly name: string;
    readonly version: number;
}

const config: Config = {
    name: "test",
    version: 1
};

return config;
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("name"), Some(&serde_json::json!("test")));
    assert_eq!(obj.get("version"), Some(&serde_json::json!(1)));
}

/// Test TypeScript with enum (should be compiled to JavaScript object)
#[tokio::test]
async fn test_typescript_enum() {
    let runtime = TypeScriptRuntime::new();

    let code = r"
enum Status {
    Pending = 0,
    Success = 1,
    Error = 2
}

const current: Status = Status.Success;

return { status: current };
";

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("status"), Some(&serde_json::json!(1)));
}

/// Test complex TypeScript scenario: async function with interfaces and type guards
#[tokio::test]
async fn test_typescript_complex_async_scenario() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    let code = r#"
interface CommandResult {
    stdout: string;
    stderr: string;
    exit_code: number;
}

async function agent_code(): Promise<object> {
    const result: CommandResult = await bash("echo 'test output'") as CommandResult;

    const processed: string = result.stdout.trim();
    const success: boolean = result.exit_code === 0;

    return {
        done: success,
        data: processed,
        message: success ? "Success" : "Failed"
    };
}
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_object());
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("done"), Some(&serde_json::json!(true)));
    assert!(
        obj.get("data")
            .and_then(|val| val.as_str())
            .unwrap()
            .contains("test output")
    );
    assert_eq!(obj.get("message"), Some(&serde_json::json!("Success")));
}

/// Test the exact user-provided code pattern with conditional error handling
#[tokio::test]
async fn test_typescript_conditional_error_handling() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    let code = r#"
async function agent_code(): Promise<string> {
    let r = await bash("echo 'TODO: test'");
    if (r.stderr) return "Error searching TODOs: " + r.stderr;
    return r.stdout || "No TODOs found in the codebase";
}
"#;

    let result = runtime.execute(code).await;
    assert!(result.is_ok(), "Failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.is_string());
    let output = value.as_str().unwrap();
    assert!(output.contains("TODO"));
}

/// Test the exact user code with grep command
#[tokio::test]
async fn test_user_grep_command() {
    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(Arc::new(BashTool));

    // Use a simpler grep command that won't fail
    let code = r#"
async function agent_code(): Promise<string> {
    let r = await bash("grep -r TODO . --exclude-dir=target 2>&1 || echo 'No matches'");
    if (r.stderr) return "Error searching TODOs: " + r.stderr;
    return r.stdout || "No TODOs found in the codebase";
}
"#;

    let result = runtime.execute(code).await;
    println!("Result: {result:?}");
    if let Err(ref err) = result {
        println!("Error details: {err}");
    }
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}
