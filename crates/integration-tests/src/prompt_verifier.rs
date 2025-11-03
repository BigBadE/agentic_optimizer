//! Prompt verification for testing prompt correctness.

use super::verification_result::VerificationResult;
use super::verify::PromptVerify;
use merlin_core::prompts::load_prompt;

/// Prompt verifier
pub struct PromptVerifier;

impl PromptVerifier {
    /// Verify a captured prompt against expectations
    pub fn verify_prompt(
        result: &mut VerificationResult,
        captured_prompt: Option<&str>,
        verify: &PromptVerify,
    ) {
        let Some(prompt) = captured_prompt else {
            result.add_failure("No prompt captured for verification".to_owned());
            return;
        };

        // Verify prompt matches expected prompt file header
        if let Some(prompt_file) = &verify.prompt_file {
            Self::verify_prompt_file(result, prompt, prompt_file);
        }

        // Verify contains patterns
        for pattern in &verify.contains {
            if !prompt.contains(pattern) {
                result.add_failure(format!("Prompt missing expected pattern: '{pattern}'"));
            }
        }

        // Verify not_contains patterns
        for pattern in &verify.not_contains {
            if prompt.contains(pattern) {
                result.add_failure(format!("Prompt contains unexpected pattern: '{pattern}'"));
            }
        }

        // Verify tool signatures
        for tool_name in &verify.has_tool_signatures {
            if !Self::has_tool_signature(prompt, tool_name) {
                result.add_failure(format!("Prompt missing tool signature for: '{tool_name}'"));
            }
        }

        // Verify type definitions
        for type_name in &verify.has_type_definitions {
            if !Self::has_type_definition(prompt, type_name) {
                result.add_failure(format!("Prompt missing type definition for: '{type_name}'"));
            }
        }
    }

    /// Verify prompt matches the expected prompt file
    fn verify_prompt_file(result: &mut VerificationResult, prompt: &str, prompt_file: &str) {
        // Load the expected prompt file (load_prompt already extracts the "## Prompt" section)
        let expected_prompt = match load_prompt(prompt_file) {
            Ok(content) => content,
            Err(err) => {
                result.add_failure(format!(
                    "Failed to load expected prompt file '{prompt_file}': {err}"
                ));
                return;
            }
        };

        // Get first significant line from the expected prompt (skip empty lines and headers)
        if let Some(first_line) = expected_prompt.lines().find(|line| {
            !line.trim().is_empty() && !line.starts_with('#') && !line.starts_with('━')
        }) {
            let first_line = first_line.trim();
            if !prompt.contains(first_line) {
                result.add_failure(format!(
                    "Captured prompt does not match expected prompt file '{prompt_file}'. \
                     Expected to find: '{first_line}'"
                ));
            }
        } else {
            result.add_failure(format!(
                "Could not extract identifiable content from prompt file '{prompt_file}'"
            ));
        }
    }

    /// Check if prompt has a tool signature
    fn has_tool_signature(prompt: &str, tool_name: &str) -> bool {
        // Look for async function declaration with the tool name
        prompt.contains(&format!("async function {tool_name}("))
            || prompt.contains(&format!("function {tool_name}("))
            || prompt.contains(&format!("{tool_name}:"))
    }

    /// Check if prompt has a type definition with full signature verification
    /// Checks that the interface/type declaration exists with opening brace
    fn has_type_definition(prompt: &str, type_name: &str) -> bool {
        // Look for interface or type definition with structure
        // Must have: "interface TypeName {" or "type TypeName =" or "class TypeName {"
        let interface_pattern = format!("interface {type_name} {{");
        let interface_newline_pattern = format!("interface {type_name}\n");
        let type_pattern = format!("type {type_name} =");
        let class_pattern = format!("class {type_name} {{");

        prompt.contains(&interface_pattern)
            || prompt.contains(&interface_newline_pattern)
            || prompt.contains(&type_pattern)
            || prompt.contains(&class_pattern)
    }
}
