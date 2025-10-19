//! TypeScript signature generation for tools.
//!
//! This module provides functionality to generate TypeScript function signatures
//! from tool descriptions, allowing LLMs to see proper type information when
//! using the TypeScript runtime.

use std::fmt::{Error as FmtError, Write as _};

use crate::Tool;

/// Generate TypeScript function signatures for a list of tools
///
/// # Errors
/// Returns an error if signature generation fails (e.g., formatting error)
pub fn generate_typescript_signatures(tools: &[&dyn Tool]) -> Result<String, FmtError> {
    let mut output = String::new();

    for tool in tools {
        let signature = generate_tool_signature(*tool)?;
        writeln!(output, "{signature}")?;
        writeln!(output)?;
    }

    Ok(output)
}

/// Generate a TypeScript function signature for a single tool
///
/// # Errors
/// Returns an error if formatting fails
fn generate_tool_signature(tool: &dyn Tool) -> Result<String, FmtError> {
    let mut sig = String::new();

    // Add JSDoc comment with description
    writeln!(sig, "/**")?;
    writeln!(sig, " * {}", tool.description())?;
    writeln!(sig, " */")?;

    // For now, use a simple signature since we don't have parameter schemas
    // Tools can provide TypeScript-compatible parameters
    write!(
        sig,
        "declare function {}(params: any): Promise<any>;",
        to_camel_case(tool.name())
    )?;

    Ok(sig)
}

/// Convert `snake_case` to `camelCase`
fn to_camel_case(input_str: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for character in input_str.chars() {
        if character == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(character.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(character);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ToolInput, ToolOutput, ToolResult};
    use async_trait::async_trait;

    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &'static str {
            "read_file"
        }

        fn description(&self) -> &'static str {
            "Reads a file from the filesystem"
        }

        async fn execute(&self, _input: ToolInput) -> ToolResult<ToolOutput> {
            Ok(ToolOutput::success("test"))
        }
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("read_file"), "readFile");
        assert_eq!(to_camel_case("write_file"), "writeFile");
        assert_eq!(to_camel_case("list_files"), "listFiles");
        assert_eq!(to_camel_case("run_command"), "runCommand");
    }

    #[test]
    fn test_generate_tool_signature() {
        let tool = MockTool;
        let signature = generate_tool_signature(&tool).unwrap();

        assert!(signature.contains("/**"));
        assert!(signature.contains("Reads a file from the filesystem"));
        assert!(signature.contains("*/"));
        assert!(signature.contains("declare function readFile"));
        assert!(signature.contains("Promise<any>"));
    }

    #[test]
    fn test_generate_multiple_signatures() {
        let tools: Vec<&dyn Tool> = vec![&MockTool];
        let signatures = generate_typescript_signatures(&tools).unwrap();

        assert!(signatures.contains("declare function readFile"));
    }
}
