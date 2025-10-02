//! Prompt templates for the context planning agent.

use crate::query::QueryIntent;

/// Generate a system prompt for the context planning agent
#[must_use]
pub fn system_prompt() -> String {
    r#"You are a context planning assistant for a code analysis tool. Your job is to analyze user queries and generate a structured plan for gathering relevant code files.

Given a user's query and extracted intent, you should:
1. Identify key symbols, types, and functions that need to be found
2. Determine file patterns that are likely relevant
3. Decide on an expansion strategy (focused, broad, entry-point-based, or semantic)
4. Set appropriate depth for traversing dependencies
5. Decide whether to include test files

Respond ONLY with a valid JSON object matching this schema:
{
  "keywords": ["list", "of", "keywords"],
  "symbols_to_find": ["SymbolName", "FunctionName"],
  "file_patterns": ["auth", "user", "session"],
  "include_tests": false,
  "max_depth": 2,
  "strategy": {
    "Focused": { "symbols": ["SymbolName"] }
    // OR "Broad": { "patterns": ["pattern"] }
    // OR "EntryPointBased": { "entry_files": ["/path/to/main.rs"] }
    // OR "Semantic": { "query": "authentication logic", "top_k": 10 }
  },
  "reasoning": "Brief explanation of your choices"
}

Be concise and practical. Focus on what will actually help find relevant code."#.to_string()
}

/// Generate a user prompt for the context planning agent
#[must_use]
pub fn user_prompt(query_text: &str, intent: &QueryIntent) -> String {
    format!(
        r#"User Query: "{}"

Extracted Intent:
- Action: {:?}
- Scope: {:?}
- Complexity: {:?}
- Keywords: {}
- Entities: {}

Generate a context plan to find the most relevant code files for this query."#,
        query_text,
        intent.action,
        intent.scope,
        intent.complexity,
        intent.keywords.join(", "),
        intent.entities.join(", ")
    )
}
