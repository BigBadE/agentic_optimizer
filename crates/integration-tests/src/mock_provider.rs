//! Mock provider for testing.

use super::fixture::{MatchType, TriggerConfig};
use async_trait::async_trait;
use merlin_core::{Context, ModelProvider, Query, Response, Result, RoutingError, TokenUsage};
use merlin_deps::regex::Regex;
use merlin_routing::{Model, ModelRouter, RoutingDecision, Task as RoutingTask};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

/// Pattern response configuration
struct PatternResponse {
    /// Trigger pattern
    pattern: String,
    /// Match type
    match_type: MatchType,
    /// TypeScript response
    typescript: String,
    /// Whether this response has been used
    used: bool,
    /// Compiled regex (if `match_type` is Regex)
    regex: Option<Regex>,
}

impl PatternResponse {
    /// Create new pattern response
    ///
    /// # Errors
    /// Returns error if regex compilation fails
    fn new(trigger: &TriggerConfig, typescript: String) -> Result<Self> {
        let regex = matches!(trigger.match_type, MatchType::Regex)
            .then(|| {
                Regex::new(&trigger.pattern)
                    .map_err(|err| RoutingError::InvalidTask(format!("Invalid regex: {err}")))
            })
            .transpose()?;

        Ok(Self {
            pattern: trigger.pattern.clone(),
            match_type: trigger.match_type,
            typescript,
            used: false,
            regex,
        })
    }

    /// Check if this pattern matches the query
    fn matches(&self, query_text: &str) -> bool {
        match self.match_type {
            MatchType::Exact => query_text == self.pattern,
            MatchType::Contains => query_text.contains(&self.pattern),
            MatchType::Regex => self
                .regex
                .as_ref()
                .is_some_and(|regex| regex.is_match(query_text)),
        }
    }
}

/// Pattern-based mock provider
pub struct PatternMockProvider {
    /// Provider name
    name: &'static str,
    /// Pattern responses
    responses: Arc<Mutex<Vec<PatternResponse>>>,
    /// Call counter for debugging
    call_count: Arc<AtomicUsize>,
}

impl PatternMockProvider {
    /// Create new pattern mock provider
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            responses: Arc::new(Mutex::new(Vec::new())),
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Add response pattern
    ///
    /// # Errors
    /// Returns error if pattern is invalid
    pub fn add_response(&self, trigger: &TriggerConfig, typescript: String) -> Result<()> {
        let response = PatternResponse::new(trigger, typescript)?;
        self.responses
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
            .push(response);
        Ok(())
    }

    /// Get matching response for query
    ///
    /// # Errors
    /// Returns error if no matching pattern found
    fn get_response(&self, query_text: &str) -> Result<String> {
        let mut responses = self
            .responses
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?;

        // Find first unused matching pattern
        let result = responses
            .iter_mut()
            .find(|resp| !resp.used && resp.matches(query_text))
            .map(|resp| {
                resp.used = true;
                resp.typescript.clone()
            });

        drop(responses);

        result.ok_or_else(|| {
            RoutingError::ExecutionFailed(format!("No matching pattern for query: {query_text}"))
        })
    }

    /// Reset all patterns to unused (for testing)
    ///
    /// # Errors
    /// Returns error if lock acquisition fails
    pub fn reset(&self) -> Result<()> {
        {
            let mut responses = self
                .responses
                .lock()
                .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?;
            for response in responses.iter_mut() {
                response.used = false;
            }
        }
        self.call_count.store(0, Ordering::SeqCst);
        Ok(())
    }
}

#[async_trait]
impl ModelProvider for PatternMockProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn generate(&self, query: &Query, _context: &Context) -> Result<Response> {
        // Increment call count
        self.call_count.fetch_add(1, Ordering::SeqCst);

        // Get matching response
        let typescript = self.get_response(&query.text)?;

        // Wrap TypeScript in code block
        let content = format!("```typescript\n{typescript}\n```");

        Ok(Response {
            text: content,
            confidence: 1.0,
            tokens_used: TokenUsage {
                input: query.text.len() as u64,
                output: typescript.len() as u64,
                cache_read: 0,
                cache_write: 0,
            },
            provider: self.name.to_owned(),
            latency_ms: 0,
        })
    }

    fn estimate_cost(&self, _context: &Context) -> f64 {
        0.0 // Mock provider is free
    }
}

/// Simple mock router that always returns the default model
pub struct MockRouter;

#[async_trait]
impl ModelRouter for MockRouter {
    async fn route(&self, _task: &RoutingTask) -> Result<RoutingDecision> {
        // Always route to the default model for tests
        Ok(RoutingDecision {
            model: Model::Qwen25Coder32B,
            estimated_cost: 0.0,
            estimated_latency_ms: 100,
            reasoning: "Test routing".to_owned(),
        })
    }

    async fn is_available(&self, _model: &Model) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests exact pattern matching
    ///
    /// # Panics
    /// Panics if pattern creation fails
    #[test]
    #[cfg_attr(test, allow(clippy::unwrap_used, reason = "Allow for tests"))]
    fn test_pattern_response_exact_match() {
        let trigger = TriggerConfig {
            pattern: "hello world".to_owned(),
            match_type: MatchType::Exact,
        };
        let response = PatternResponse::new(&trigger, "test".to_owned()).unwrap();
        assert!(response.matches("hello world"));
        assert!(!response.matches("hello"));
        assert!(!response.matches("hello world!"));
    }

    /// Tests contains pattern matching
    ///
    /// # Panics
    /// Panics if pattern creation fails
    #[test]
    #[cfg_attr(test, allow(clippy::unwrap_used, reason = "Allow for tests"))]
    fn test_pattern_response_contains_match() {
        let trigger = TriggerConfig {
            pattern: "world".to_owned(),
            match_type: MatchType::Contains,
        };
        let response = PatternResponse::new(&trigger, "test".to_owned()).unwrap();
        assert!(response.matches("hello world"));
        assert!(response.matches("world"));
        assert!(response.matches("world hello"));
        assert!(!response.matches("hello"));
    }

    /// Tests regex pattern matching
    ///
    /// # Panics
    /// Panics if pattern creation fails
    #[test]
    #[cfg_attr(test, allow(clippy::unwrap_used, reason = "Allow for tests"))]
    fn test_pattern_response_regex_match() {
        let trigger = TriggerConfig {
            pattern: r"hello\s+\w+".to_owned(),
            match_type: MatchType::Regex,
        };
        let response = PatternResponse::new(&trigger, "test".to_owned()).unwrap();
        assert!(response.matches("hello world"));
        assert!(response.matches("hello there"));
        assert!(!response.matches("hello"));
        assert!(!response.matches("helloworld"));
    }
}
