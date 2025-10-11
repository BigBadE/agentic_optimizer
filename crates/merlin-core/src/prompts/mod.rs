//! Prompt loading utilities
//!
//! This module provides functions to load prompts from the central prompts directory.
//! Each prompt file is a markdown document with Usage and Prompt sections.
//! Prompts are embedded at compile time using `include_str!`.

// Embed prompt files at compile time
const CODING_ASSISTANT_MD: &str = include_str!("../../../../prompts/coding_assistant.md");
const CONTEXT_PLANNING_MD: &str = include_str!("../../../../prompts/context_planning.md");
const TASK_ASSESSMENT_MD: &str = include_str!("../../../../prompts/task_assessment.md");

/// Loads a prompt by name
///
/// # Errors
/// Returns an error if the prompt name is unknown or the prompt section cannot be extracted
pub fn load_prompt(name: &str) -> Result<String, String> {
    let content = match name {
        "coding_assistant" => CODING_ASSISTANT_MD,
        "context_planning" => CONTEXT_PLANNING_MD,
        "task_assessment" => TASK_ASSESSMENT_MD,
        _ => return Err(format!("Unknown prompt: {name}")),
    };

    extract_prompt_section(content)
}

/// Extracts the Prompt section from a markdown file
///
/// # Errors
/// Returns an error if the Prompt section cannot be found
fn extract_prompt_section(content: &str) -> Result<String, String> {
    // Find the "## Prompt" header
    let prompt_start = content
        .find("## Prompt")
        .ok_or_else(|| "Prompt section not found".to_string())?;

    // Skip past the header line
    let prompt_content_start = content[prompt_start..]
        .find('\n')
        .ok_or_else(|| "Invalid prompt format".to_string())?
        + prompt_start
        + 1;

    // Find the next section or end of file
    let prompt_end = content[prompt_content_start..]
        .find("\n## ")
        .map_or(content.len(), |pos| prompt_content_start + pos);

    // Extract and trim the prompt content
    Ok(content[prompt_content_start..prompt_end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_prompt_section() {
        let markdown = r"# Test Prompt

## Usage

This is usage info.

## Prompt

This is the actual prompt content.

It can have multiple lines.
";

        let result = extract_prompt_section(markdown).unwrap();
        assert_eq!(
            result,
            "This is the actual prompt content.\n\nIt can have multiple lines."
        );
    }

    #[test]
    fn test_load_coding_assistant_prompt() {
        let result = load_prompt("coding_assistant");
        assert!(
            result.is_ok(),
            "Failed to load coding_assistant prompt: {:?}",
            result.err()
        );
        let prompt = result.unwrap();
        // Ensure Usage section is not included in the extracted prompt
        assert!(!prompt.contains("## Usage"));
        assert!(!prompt.contains("When used:"));
    }

    #[test]
    fn test_load_context_planning_prompt() {
        let result = load_prompt("context_planning");
        assert!(
            result.is_ok(),
            "Failed to load context_planning prompt: {:?}",
            result.err()
        );
        let prompt = result.unwrap();
        // Ensure Usage section is not included in the extracted prompt
        assert!(!prompt.contains("## Usage"));
        assert!(!prompt.contains("When used:"));
    }

    #[test]
    fn test_load_task_assessment_prompt() {
        let result = load_prompt("task_assessment");
        assert!(
            result.is_ok(),
            "Failed to load task_assessment prompt: {:?}",
            result.err()
        );
        let prompt = result.unwrap();
        // Ensure Usage section is not included in the extracted prompt
        assert!(!prompt.contains("## Usage"));
        assert!(!prompt.contains("When used:"));
    }
}
