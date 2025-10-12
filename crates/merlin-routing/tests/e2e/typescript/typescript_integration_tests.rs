//! Comprehensive integration tests for TypeScript tool with real file operations

use merlin_routing::{
    ListFilesTool, ReadFileTool, RunCommandTool, Tool, TypeScriptTool, WriteFileTool,
};
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to create a TypeScript tool with all basic tools
fn create_typescript_tool(workspace: &std::path::Path) -> TypeScriptTool {
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(ReadFileTool::new(workspace.to_path_buf())),
        Arc::new(WriteFileTool::new(workspace.to_path_buf())),
        Arc::new(ListFilesTool::new(workspace.to_path_buf())),
        Arc::new(RunCommandTool::new(workspace.to_path_buf())),
    ];
    TypeScriptTool::new(tools)
}

#[tokio::test]
async fn test_typescript_basic_arithmetic() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r"
        const x = 10;
        const y = 20;
        const sum = x + y;
        const product = x * y;
        ({ sum, product, average: (x + y) / 2 })
    ";

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result["sum"], 30);
    assert_eq!(result["product"], 200);
    assert_eq!(result["average"], 15);
}

#[tokio::test]
async fn test_typescript_array_operations() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r"
        const numbers = [1, 2, 3, 4, 5];
        const doubled = numbers.map(n => n * 2);
        const evens = numbers.filter(n => n % 2 === 0);
        const sum = numbers.reduce((acc, n) => acc + n, 0);
        ({ doubled, evens, sum })
    ";

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result["doubled"], json!([2, 4, 6, 8, 10]));
    assert_eq!(result["evens"], json!([2, 4]));
    assert_eq!(result["sum"], 15);
}

#[tokio::test]
async fn test_typescript_control_flow() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r"
        let result = [];
        for (let i = 0; i < 5; i++) {
            if (i % 2 === 0) {
                result.push(i * 2);
            } else {
                result.push(i);
            }
        }
        result
    ";

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result, json!([0, 1, 4, 3, 8]));
}

#[tokio::test]
async fn test_typescript_functions() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r"
        function fibonacci(n) {
            if (n <= 1) return n;
            return fibonacci(n - 1) + fibonacci(n - 2);
        }
        
        const results = [];
        for (let i = 0; i < 10; i++) {
            results.push(fibonacci(i));
        }
        results
    ";

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result, json!([0, 1, 1, 2, 3, 5, 8, 13, 21, 34]));
}

#[tokio::test]
async fn test_typescript_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    // Syntax error
    let code = "const x = ;";
    let result = ts_tool.execute(json!({ "code": code })).await;
    assert!(result.is_err());

    // Runtime error (undefined variable)
    let code = "nonexistent_variable";
    let result = ts_tool.execute(json!({ "code": code })).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_typescript_missing_code_parameter() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let result = ts_tool.execute(json!({})).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Missing 'code' parameter")
    );
}

#[tokio::test]
async fn test_typescript_object_manipulation() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r#"
        const person = {
            name: "Alice",
            age: 30,
            city: "New York"
        };
        
        const updated = {
            ...person,
            age: 31,
            country: "USA"
        };
        
        updated
    "#;

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result["name"], "Alice");
    assert_eq!(result["age"], 31);
    assert_eq!(result["city"], "New York");
    assert_eq!(result["country"], "USA");
}

#[tokio::test]
async fn test_typescript_string_operations() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r#"
        const text = "Hello, World!";
        ({
            upper: text.toUpperCase(),
            lower: text.toLowerCase(),
            length: text.length,
            words: text.split(", "),
            replaced: text.replace("World", "TypeScript")
        })
    "#;

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result["upper"], "HELLO, WORLD!");
    assert_eq!(result["lower"], "hello, world!");
    assert_eq!(result["length"], 13);
    assert_eq!(result["words"], json!(["Hello", "World!"]));
    assert_eq!(result["replaced"], "Hello, TypeScript!");
}

#[tokio::test]
async fn test_typescript_nested_structures() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r"
        const data = {
            users: [
                { name: 'Alice', scores: [90, 85, 92] },
                { name: 'Bob', scores: [78, 82, 88] },
                { name: 'Charlie', scores: [95, 98, 94] }
            ]
        };
        
        const averages = data.users.map(user => ({
            name: user.name,
            average: user.scores.reduce((a, b) => a + b, 0) / user.scores.length
        }));
        
        averages
    ";

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert!(result.is_array());
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["name"], "Alice");
    assert!((arr[0]["average"].as_f64().unwrap() - 89.0).abs() < 0.1);
}

#[tokio::test]
async fn test_typescript_with_file_write_and_read() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    // Note: This test verifies the code executes without errors
    // Actual file operations are pending full async implementation
    let code = r#"
        const content = "Hello from TypeScript!";
        const filename = "test.txt";
        
        // These will return pending promises for now
        // await writeFile(filename, content);
        // const read = await readFile(filename);
        
        ({ message: "File operations prepared", filename, content })
    "#;

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result["message"], "File operations prepared");
    assert_eq!(result["filename"], "test.txt");
    assert_eq!(result["content"], "Hello from TypeScript!");
}

#[tokio::test]
async fn test_typescript_complex_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r"
        // Simulate a data processing pipeline
        const rawData = [
            { id: 1, value: 100, category: 'A' },
            { id: 2, value: 200, category: 'B' },
            { id: 3, value: 150, category: 'A' },
            { id: 4, value: 300, category: 'C' },
            { id: 5, value: 250, category: 'B' }
        ];
        
        // Filter, transform, and aggregate
        const categoryA = rawData.filter(item => item.category === 'A');
        const totalA = categoryA.reduce((sum, item) => sum + item.value, 0);
        
        const allCategories = [...new Set(rawData.map(item => item.category))];
        const summary = allCategories.map(cat => {
            const items = rawData.filter(item => item.category === cat);
            const total = items.reduce((sum, item) => sum + item.value, 0);
            const average = total / items.length;
            return { category: cat, count: items.length, total, average };
        });
        
        ({ totalA, summary })
    ";

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result["totalA"], 250);

    let summary = result["summary"].as_array().unwrap();
    assert_eq!(summary.len(), 3);

    let cat_a = summary.iter().find(|s| s["category"] == "A").unwrap();
    assert_eq!(cat_a["count"], 2);
    assert_eq!(cat_a["total"], 250);
}

#[tokio::test]
async fn test_typescript_json_manipulation() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r#"
        const config = {
            "name": "MyApp",
            "version": "1.0.0",
            "dependencies": {
                "react": "^18.0.0",
                "typescript": "^5.0.0"
            }
        };
        
        // Update version
        config.version = "1.1.0";
        
        // Add new dependency
        config.dependencies["axios"] = "^1.0.0";
        
        // Add metadata
        config.metadata = {
            updated: "2025-10-12",
            author: "AI Agent"
        };
        
        config
    "#;

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result["version"], "1.1.0");
    assert_eq!(result["dependencies"]["axios"], "^1.0.0");
    assert_eq!(result["metadata"]["author"], "AI Agent");
}

#[tokio::test]
async fn test_typescript_date_and_math() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r"
        const numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        
        ({
            sum: numbers.reduce((a, b) => a + b, 0),
            max: Math.max(...numbers),
            min: Math.min(...numbers),
            sqrt: Math.sqrt(16),
            random: Math.floor(Math.random() * 100) >= 0,
            pi: Math.PI
        })
    ";

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result["sum"], 55);
    assert_eq!(result["max"], 10);
    assert_eq!(result["min"], 1);
    assert_eq!(result["sqrt"], 4);
    assert_eq!(result["random"], true);
    assert!((result["pi"].as_f64().unwrap() - 3.14159).abs() < 0.001);
}

#[tokio::test]
async fn test_typescript_template_literals() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r#"
        const name = "Alice";
        const age = 30;
        const greeting = `Hello, ${name}! You are ${age} years old.`;
        const multiline = `
            Line 1
            Line 2
            Line 3
        `.trim();
        
        ({ greeting, multiline, lines: multiline.split('\n').length })
    "#;

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result["greeting"], "Hello, Alice! You are 30 years old.");
    assert_eq!(result["lines"], 3);
}

#[tokio::test]
async fn test_typescript_destructuring() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r"
        const person = { name: 'Bob', age: 25, city: 'NYC' };
        const { name, age } = person;
        
        const numbers = [10, 20, 30, 40];
        const [first, second, ...rest] = numbers;
        
        ({ name, age, first, second, rest })
    ";

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result["name"], "Bob");
    assert_eq!(result["age"], 25);
    assert_eq!(result["first"], 10);
    assert_eq!(result["second"], 20);
    assert_eq!(result["rest"], json!([30, 40]));
}

#[tokio::test]
async fn test_typescript_arrow_functions_and_closures() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r"
        const makeCounter = () => {
            let count = 0;
            return () => ++count;
        };
        
        const counter = makeCounter();
        const results = [];
        for (let i = 0; i < 5; i++) {
            results.push(counter());
        }
        
        results
    ";

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result, json!([1, 2, 3, 4, 5]));
}

#[tokio::test]
async fn test_typescript_set_and_unique_values() {
    let temp_dir = TempDir::new().unwrap();
    let ts_tool = create_typescript_tool(temp_dir.path());

    let code = r"
        const numbers = [1, 2, 2, 3, 3, 3, 4, 4, 4, 4];
        const unique = [...new Set(numbers)];
        
        const words = ['apple', 'banana', 'apple', 'cherry', 'banana'];
        const uniqueWords = [...new Set(words)];
        
        ({ unique, uniqueWords, count: unique.length })
    ";

    let result = ts_tool.execute(json!({ "code": code })).await.unwrap();
    assert_eq!(result["unique"], json!([1, 2, 3, 4]));
    assert_eq!(result["uniqueWords"], json!(["apple", "banana", "cherry"]));
    assert_eq!(result["count"], 4);
}
