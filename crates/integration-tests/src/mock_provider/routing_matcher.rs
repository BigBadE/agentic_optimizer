//! Routing-based matching for test fixtures.
//!
//! Matches queries based on routing decisions rather than string patterns.

use merlin_core::{Context, ContextType, ExecutionResult, PromptType, Query};

/// Routing-based matcher for test fixtures.
///
/// Matches based on routing decisions (context type, prompt type, tier, etc.)
/// rather than query string content.
#[derive(Debug, Clone)]
pub struct RoutingMatcher {
    /// Expected context type (None matches any)
    pub context_type: Option<ContextType>,
    /// Expected prompt type (None matches any)
    pub prompt_type: Option<PromptType>,
    /// Expected difficulty range (None matches any)
    pub difficulty_range: Option<(u8, u8)>,
    /// Expected retry attempt (None matches any)
    pub retry_attempt: Option<u8>,
    /// Expected previous result (None matches any)
    pub previous_result: Option<ExecutionResult>,
}

impl RoutingMatcher {
    /// Create a new routing matcher that matches any query.
    #[must_use]
    pub fn any() -> Self {
        Self {
            context_type: None,
            prompt_type: None,
            difficulty_range: None,
            retry_attempt: None,
            previous_result: None,
        }
    }

    /// Check if this matcher matches the given query and context.
    pub fn matches(&self, query: &Query, _context: &Context) -> bool {
        let rc = &query.routing_context;

        // Check context type
        if let Some(expected) = self.context_type
            && rc.context_type != expected
        {
            return false;
        }

        // Check prompt type
        if let Some(expected) = self.prompt_type
            && rc.prompt_type != expected
        {
            return false;
        }

        // Check difficulty range
        if let Some((min, max)) = self.difficulty_range {
            let Some(difficulty) = rc.estimated_difficulty else {
                return false;
            };
            if difficulty < min || difficulty > max {
                return false;
            }
        }

        // Check retry attempt
        if let Some(expected) = self.retry_attempt
            && rc.retry_attempt != expected
        {
            return false;
        }

        // Check previous result
        if let Some(expected) = self.previous_result
            && rc.previous_result != Some(expected)
        {
            return false;
        }

        true
    }

    /// Get description of this matcher for diagnostics.
    #[must_use]
    pub fn description(&self) -> String {
        let mut parts = Vec::new();

        if let Some(context_type_value) = self.context_type {
            parts.push(format!("context={context_type_value:?}"));
        }

        if let Some(prompt_type_value) = self.prompt_type {
            parts.push(format!("prompt={prompt_type_value:?}"));
        }

        if let Some((min, max)) = self.difficulty_range {
            parts.push(format!("difficulty=[{min}-{max}]"));
        }

        if let Some(retry) = self.retry_attempt {
            parts.push(format!("retry={retry}"));
        }

        if let Some(result) = self.previous_result {
            parts.push(format!("previous={result:?}"));
        }

        if parts.is_empty() {
            "matches any".to_owned()
        } else {
            parts.join(", ")
        }
    }
}
