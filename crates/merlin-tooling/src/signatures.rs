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
        writeln!(output, "{}", tool.typescript_signature())?;
        writeln!(output)?;
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ToolInput, ToolOutput, ToolResult};
    use anyhow::Result;
    use async_trait::async_trait;

    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &'static str {
            "read_file"
        }

        fn typescript_signature(&self) -> &'static str {
            "/**\n * Reads a file from the filesystem\n */\ndeclare function readFile(path: string): Promise<string>;"
        }

        async fn execute(&self, _input: ToolInput) -> ToolResult<ToolOutput> {
            Ok(ToolOutput::success("test"))
        }
    }

    /// Tests TypeScript signature generation for multiple tools.
    ///
    /// # Errors
    /// Returns an error if signature generation fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_generate_multiple_signatures() -> Result<()> {
        let tools: Vec<&dyn Tool> = vec![&MockTool];
        let signatures = generate_typescript_signatures(&tools)?;

        assert!(signatures.contains("declare function readFile"));
        assert!(signatures.contains("Reads a file from the filesystem"));
        assert!(signatures.contains("Promise<string>"));
        Ok(())
    }
}
