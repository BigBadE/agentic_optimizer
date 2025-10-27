//! Query intent classification for determining context needs

/// Intent classification for queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryIntent {
    /// Conversational query - no file context needed
    Conversational,
    /// Code query - needs file context but no modification
    CodeQuery,
    /// Code modification - needs file context and write capability
    CodeModification,
}

impl QueryIntent {
    /// Classify the intent of a query to determine context needs
    pub fn classify(description: &str) -> Self {
        let desc_lower = description.to_lowercase();
        let word_count = description.split_whitespace().count();

        // Conversational patterns - no file context needed
        if desc_lower == "hi"
            || desc_lower == "hello"
            || desc_lower == "hey"
            || desc_lower == "thanks"
            || desc_lower == "thank you"
            || desc_lower.starts_with("say hi")
            || desc_lower.starts_with("say hello")
        {
            return Self::Conversational;
        }

        // Memory/recall patterns
        if desc_lower.contains("remember")
            || desc_lower.contains("what did i")
            || desc_lower.contains("what was the")
            || desc_lower.contains("recall")
            || (desc_lower.contains("what") && desc_lower.contains("told you"))
            || (desc_lower.contains("what") && desc_lower.contains("said"))
        {
            return Self::Conversational;
        }

        // Very short requests - likely conversational
        if word_count <= 3 {
            return Self::Conversational;
        }

        // Code modification keywords
        if desc_lower.contains("add ")
            || desc_lower.contains("create ")
            || desc_lower.contains("implement")
            || desc_lower.contains("write ")
            || desc_lower.contains("modify")
            || desc_lower.contains("change ")
            || desc_lower.contains("fix ")
            || desc_lower.contains("update ")
            || desc_lower.contains("refactor")
        {
            return Self::CodeModification;
        }

        // Default to code query for anything else
        Self::CodeQuery
    }

    /// Check if a request is simple enough to skip assessment
    pub fn is_simple(description: &str) -> bool {
        matches!(Self::classify(description), Self::Conversational)
    }
}
