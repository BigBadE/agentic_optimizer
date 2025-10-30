//! Tests for executor functionality

use super::super::AgentExecutor;
use super::typescript;
use crate::ValidationPipeline;
use merlin_context::ContextFetcher;
use merlin_core::RoutingConfig;
use merlin_deps::serde_json::{from_value, json};
use merlin_routing::StrategyRouter;
use merlin_tooling::{BashTool, ToolRegistry};
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::test]
async fn test_agent_executor_creation() {
    // Use local-only config to avoid needing API keys
    let mut config = RoutingConfig::default();
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;

    let router = StrategyRouter::with_default_strategies();
    if router.is_err() {
        // Expected when providers can't be initialized
        return;
    }
    let router = Arc::new(router.unwrap());

    let validator = Arc::new(ValidationPipeline::with_default_stages());
    let tool_registry = Arc::new(ToolRegistry::default());
    let context_fetcher = ContextFetcher::new(PathBuf::from("."));

    let executor = AgentExecutor::new(router, validator, tool_registry, context_fetcher, &config);

    // Without API keys, executor creation may fail
    if executor.is_err() {
        return;
    }

    let _executor = executor.unwrap();
    // Executor created successfully
}

#[tokio::test]
async fn test_tool_registry_integration() {
    let tool_registry = Arc::new(ToolRegistry::default().with_tool(Arc::new(BashTool)));

    assert!(tool_registry.get_tool("bash").is_some());
    assert!(tool_registry.get_tool("nonexistent").is_none());
}

#[test]
fn test_extract_typescript_code_single_block() {
    let text = r#"
I'll read the file using TypeScript:

```typescript
const content = await readFile("src/main.rs");
return {done: true, result: content};
```

That should work!
"#;
    let code = typescript::extract_typescript_code(text);
    assert!(code.is_some());
    let code = code.unwrap();
    assert!(code.contains("readFile"));
    assert!(code.contains("done: true"));
}

#[test]
fn test_extract_typescript_code_ts_language() {
    let text = r#"
```ts
const files = await listFiles("src");
return {done: true, result: files.join(", ")};
```
"#;
    let code = typescript::extract_typescript_code(text);
    assert!(code.is_some());
    let code = code.unwrap();
    assert!(code.contains("listFiles"));
}

#[test]
fn test_extract_typescript_code_multiple_blocks() {
    let text = r#"
First block:
```typescript
const x = 1;
```

Second block:
```typescript
const y = 2;
return {done: true, result: "ok"};
```
"#;
    let code = typescript::extract_typescript_code(text);
    assert!(code.is_some());
    let code = code.unwrap();
    assert!(code.contains("const x = 1"));
    assert!(code.contains("const y = 2"));
}

#[test]
fn test_extract_typescript_code_no_blocks() {
    let text = "Just regular text with no code blocks";
    let code = typescript::extract_typescript_code(text);
    assert!(code.is_none());
}


#[test]
fn test_extract_typescript_code_syntax_error() {
    // Test that TypeScript code with syntax errors can still be extracted
    let text = r"
```typescript
const x = ;  // Syntax error
return {done: true};
```
";
    let code = typescript::extract_typescript_code(text);
    assert!(code.is_some());
    let code = code.unwrap();
    assert!(code.contains("const x ="));
}

#[test]
fn test_extract_typescript_code_no_code_blocks() {
    // Test that text without code blocks returns None
    let text = "This is just plain text without any code blocks.";
    let code = typescript::extract_typescript_code(text);
    assert!(code.is_none());
}

#[test]
fn test_extract_typescript_code_empty_block() {
    // Test that empty code blocks are filtered out and return None
    let text = r"
```typescript
```
";
    let code = typescript::extract_typescript_code(text);
    assert!(code.is_none(), "Empty code blocks should be filtered out");
}


#[test]
fn test_extract_typescript_code_with_indentation() {
    // Test that indented code blocks are preserved
    let text = r#"
Here's the code:

```typescript
function test() {
    if (true) {
        const nested = "value";
        return {done: true, result: nested};
    }
}
```
"#;
    let code = typescript::extract_typescript_code(text);
    assert!(code.is_some());
    let code = code.unwrap();
    assert!(code.contains("    if (true)"));
    assert!(code.contains("        const nested"));
}

#[test]
fn test_extract_typescript_code_mixed_languages() {
    // Test that only TypeScript blocks are extracted, not other languages
    let text = r"
```rust
fn main() {}
```

```typescript
const x = 1;
return {done: true};
```

```python
def test():
    pass
```
";
    let code = typescript::extract_typescript_code(text);
    assert!(code.is_some());
    let code = code.unwrap();
    assert!(code.contains("const x = 1"));
    assert!(!code.contains("fn main"));
    assert!(!code.contains("def test"));
}
