//! Prompt loading utilities
//!
//! This module provides functions to load prompts from the central prompts directory.
//! Each prompt file is a markdown document with Usage and Prompt sections.
//! Prompts are embedded at compile time using `include_str!`.

// Embed prompt files at compile time
const CONTEXT_PLANNING_MD: &str = include_str!("../../../../prompts/context_planning.md");
const TYPESCRIPT_AGENT_MD: &str = include_str!("../../../../prompts/typescript_agent.md");

/// Loads a prompt by name
///
/// # Errors
/// Returns an error if the prompt name is unknown or the prompt section cannot be extracted
pub fn load_prompt(name: &str) -> Result<String, String> {
    let content = match name {
        "context_planning" => CONTEXT_PLANNING_MD,
        "typescript_agent" => TYPESCRIPT_AGENT_MD,
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

    // Take everything after "## Prompt" to the end of the file
    // (all prompt files have ## Prompt as the last top-level section)
    Ok(content[prompt_content_start..].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_deps::anyhow::Result;

    #[test]
    fn test_extract_prompt_section() -> Result<()> {
        let markdown = r"# Test Prompt

## Usage

This is usage info.

## Prompt

This is the actual prompt content.

It can have multiple lines.
";

        let result = extract_prompt_section(markdown)
            .map_err(|err| merlin_deps::anyhow::anyhow!("{err}"))?;
        assert_eq!(
            result,
            "This is the actual prompt content.\n\nIt can have multiple lines."
        );
        Ok(())
    }

    #[test]
    fn test_load_context_planning_prompt() -> Result<()> {
        let prompt =
            load_prompt("context_planning").map_err(|err| merlin_deps::anyhow::anyhow!("{err}"))?;
        // Ensure Usage section is not included in the extracted prompt
        assert!(!prompt.contains("## Usage"));
        assert!(!prompt.contains("When used:"));
        Ok(())
    }

    #[test]
    fn test_load_typescript_agent_prompt() -> Result<()> {
        let prompt =
            load_prompt("typescript_agent").map_err(|err| merlin_deps::anyhow::anyhow!("{err}"))?;
        // Ensure Usage section is not included in the extracted prompt
        assert!(!prompt.contains("## Usage"));
        assert!(!prompt.contains("When used:"));
        Ok(())
    }
}
