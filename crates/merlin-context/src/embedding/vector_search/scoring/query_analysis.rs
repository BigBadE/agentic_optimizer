//! Query intent detection and adaptive weight calculation.

/// Detect query intent from keywords
pub fn detect_query_intent(query: &str) -> &'static str {
    let query_lower = query.to_lowercase();

    if query_lower.starts_with("how") || query_lower.contains(" work") {
        "explanation"
    } else if query_lower.starts_with("implement") || query_lower.starts_with("add") {
        "implementation"
    } else if query_lower.starts_with("fix")
        || query_lower.starts_with("debug")
        || query_lower.starts_with("where")
    {
        "debugging"
    } else {
        "general"
    }
}

/// Calculate adaptive weights based on query characteristics
pub fn calculate_adaptive_weights(query: &str) -> (f32, f32) {
    // Detect special tokens that indicate exact matching is important
    let has_special_tokens = query.contains("::") || query.contains("--") || query.contains("#[");
    let intent = detect_query_intent(query);

    if has_special_tokens {
        // Favor BM25 for exact matches
        (0.7, 0.3)
    } else {
        match intent {
            "explanation" => (0.3, 0.7),    // Favor semantics for "how does X work"
            "implementation" => (0.5, 0.5), // Balanced for "implement X"
            "debugging" => (0.6, 0.4),      // Favor keywords for "fix/where is X"
            _ => (0.4, 0.6),                // Default
        }
    }
}
