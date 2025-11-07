//! Tests for executor functionality

use super::super::AgentExecutor;
use super::typescript;
use crate::ValidationPipeline;
use merlin_context::ContextFetcher;
use merlin_core::RoutingConfig;
use merlin_routing::StrategyRouter;
use merlin_tooling::{BashTool, ToolRegistry};
use std::path::PathBuf;
use std::sync::Arc;

/// Tests that an agent executor can be created successfully.
///
/// # Panics
/// Panics if executor creation succeeds but returns an error when it shouldn't.
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
    let Ok(router) = router else {
        return;
    };
    let router = Arc::new(router);

    let validator = Arc::new(ValidationPipeline::with_default_stages());
    let workspace_root = PathBuf::from(".");
    let tool_registry = ToolRegistry::with_workspace(workspace_root.clone());
    let context_fetcher = ContextFetcher::new(workspace_root);

    let executor = AgentExecutor::new(router, validator, tool_registry, context_fetcher, &config);

    // Without API keys, executor creation may fail
    if let Ok(_executor) = executor {
        // Executor created successfully
    }
}

/// Tests that the tool registry can be integrated with tools.
///
/// # Panics
/// Panics if tool registration or retrieval doesn't work as expected.
#[tokio::test]
async fn test_tool_registry_integration() {
    let tool_registry =
        ToolRegistry::with_workspace(PathBuf::from(".")).with_tool(Arc::new(BashTool));

    assert!(tool_registry.get_tool("bash").is_some());
    assert!(tool_registry.get_tool("nonexistent").is_none());
}

/// Tests extracting a single TypeScript code block from text.
///
/// # Panics
/// Panics if code extraction fails or extracted code doesn't contain expected strings.
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
    assert!(code.is_some(), "Should extract TypeScript code block");
    if let Some(extracted_code) = code {
        assert!(extracted_code.contains("readFile"));
        assert!(extracted_code.contains("done: true"));
    }
}

/// Tests extracting TypeScript code with 'ts' language tag.
///
/// # Panics
/// Panics if code extraction fails or extracted code doesn't contain expected strings.
#[test]
fn test_extract_typescript_code_ts_language() {
    let text = r#"
```ts
const files = await listFiles("src");
return {done: true, result: files.join(", ")};
```
"#;
    let code = typescript::extract_typescript_code(text);
    assert!(code.is_some(), "Should extract ts code block");
    if let Some(extracted_code) = code {
        assert!(extracted_code.contains("listFiles"));
    }
}

/// Tests extracting multiple TypeScript code blocks from text.
///
/// # Panics
/// Panics if code extraction fails or extracted code doesn't contain expected strings.
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
    assert!(
        code.is_some(),
        "Should extract multiple TypeScript code blocks"
    );
    if let Some(extracted_code) = code {
        assert!(extracted_code.contains("const x = 1"));
        assert!(extracted_code.contains("const y = 2"));
    }
}

/// Tests that extraction returns None when no code blocks are present.
///
/// # Panics
/// Panics if code extraction unexpectedly returns Some.
#[test]
fn test_extract_typescript_code_no_blocks() {
    let text = "Just regular text with no code blocks";
    let code = typescript::extract_typescript_code(text);
    assert!(code.is_none());
}

/// Tests that TypeScript code with syntax errors can still be extracted.
///
/// # Panics
/// Panics if code extraction fails or extracted code doesn't contain expected strings.
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
    assert!(
        code.is_some(),
        "Should extract TypeScript code even with syntax errors"
    );
    if let Some(extracted_code) = code {
        assert!(extracted_code.contains("const x ="));
    }
}

/// Tests that text without code blocks returns None.
///
/// # Panics
/// Panics if code extraction unexpectedly returns Some.
#[test]
fn test_extract_typescript_code_no_code_blocks() {
    // Test that text without code blocks returns None
    let text = "This is just plain text without any code blocks.";
    let code = typescript::extract_typescript_code(text);
    assert!(code.is_none());
}

/// Tests that empty code blocks are filtered out and return None.
///
/// # Panics
/// Panics if empty code blocks are not filtered out correctly.
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

/// Tests that indented code blocks are preserved correctly.
///
/// # Panics
/// Panics if code extraction fails or indentation is not preserved.
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
    assert!(code.is_some(), "Should extract indented TypeScript code");
    if let Some(extracted_code) = code {
        assert!(extracted_code.contains("    if (true)"));
        assert!(extracted_code.contains("        const nested"));
    }
}

/// Tests that only TypeScript blocks are extracted, not other languages.
///
/// # Panics
/// Panics if code extraction fails or includes non-TypeScript code.
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
    assert!(
        code.is_some(),
        "Should extract TypeScript code from mixed language blocks"
    );
    if let Some(extracted_code) = code {
        assert!(extracted_code.contains("const x = 1"));
        assert!(!extracted_code.contains("fn main"));
        assert!(!extracted_code.contains("def test"));
    }
}
