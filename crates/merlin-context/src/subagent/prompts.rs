//! Prompt templates for the context planning agent.

use crate::query::QueryIntent;
use merlin_core::prompts::load_prompt;

/// Generate a system prompt for the context planning agent
///
/// # Panics
/// Panics if the `context_planning` prompt cannot be loaded (should never happen as prompts are embedded)
pub fn system_prompt() -> String {
    load_prompt("context_planning")
        .unwrap_or_else(|err| panic!("Failed to load context_planning prompt: {err}"))
}

/// Generate a user prompt for the context planning agent
pub fn user_prompt(query_text: &str, intent: &QueryIntent, file_tree: &str) -> String {
    format!(
        r#"User Query: \"{}\"

Extracted Intent:
- Action: {:?}
- Scope: {:?}
- Complexity: {:?}
- Keywords: {}
- Entities: {}

{}

Generate a context plan to find the most relevant code files for this query. Use the project structure above to identify actual directories and files that match the query intent."#,
        query_text,
        intent.action,
        intent.scope,
        intent.complexity,
        intent.keywords.join(", "),
        intent.entities.join(", "),
        file_tree
    )
}
