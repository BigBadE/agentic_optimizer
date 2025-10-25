//! Mock provider for testing agent responses.
//!
//! Allows defining canned responses for specific queries, enabling
//! end-to-end testing of agent workflows without real API calls.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::option_if_let_else,
    clippy::explicit_iter_loop,
    clippy::must_use_candidate,
    clippy::missing_panics_doc,
    clippy::type_complexity,
    clippy::significant_drop_tightening,
    clippy::return_self_not_must_use,
    reason = "Mock provider is for testing only"
)]

use async_trait::async_trait;
use merlin_core::{Context, ModelProvider, Query, Response, Result, TokenUsage};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock provider that returns pre-defined responses based on query patterns.
///
/// Useful for testing agent workflows end-to-end without making real API calls.
#[derive(Clone)]
pub struct MockProvider {
    /// Name of this mock provider
    name: String,
    /// Predefined responses keyed by query text
    responses: Arc<Mutex<HashMap<String, String>>>,
    /// Default response if no match found
    default_response: Arc<Mutex<Option<String>>>,
    /// Call history for verification
    call_history: Arc<Mutex<Vec<String>>>,
}

impl MockProvider {
    /// Create a new mock provider with a given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            responses: Arc::new(Mutex::new(HashMap::new())),
            default_response: Arc::new(Mutex::new(None)),
            call_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a pattern-based response to the mock provider.
    pub fn with_response(self, pattern: impl Into<String>, response: impl Into<String>) -> Self {
        let mut responses = self.responses.lock().expect("Lock poisoned");
        responses.insert(pattern.into(), response.into());
        drop(responses);
        self
    }

    /// Set a default response for queries that don't match any pattern.
    pub fn with_default_response(self, response: impl Into<String>) -> Self {
        let mut default = self.default_response.lock().expect("Lock poisoned");
        *default = Some(response.into());
        drop(default);
        self
    }

    /// Clear the call history (used for testing).
    pub fn clear_history(&self) {
        let mut history = self.call_history.lock().expect("Lock poisoned");
        history.clear();
    }

    /// Get the call history (list of all queries made).
    ///
    /// # Errors
    /// Returns error if lock is poisoned.
    pub fn get_call_history(&self) -> Result<Vec<String>> {
        let history = self.call_history.lock().expect("Lock poisoned");
        Ok(history.clone())
    }

    /// Get the number of calls made.
    pub fn call_count(&self) -> usize {
        let history = self.call_history.lock().expect("Lock poisoned");
        history.len()
    }

    /// Find a matching response for the given query text.
    fn find_response(&self, query_text: &str) -> Option<String> {
        let responses = self.responses.lock().expect("Lock poisoned");

        // Try exact match first
        if let Some(response) = responses.get(query_text) {
            return Some(response.clone());
        }

        // Try substring match
        for (pattern, response) in responses.iter() {
            if query_text.contains(pattern) {
                return Some(response.clone());
            }
        }

        None
    }
}

#[async_trait]
impl ModelProvider for MockProvider {
    fn name(&self) -> &'static str {
        // We can't return a dynamic string here due to lifetime constraints,
        // so we return a fixed string
        "mock"
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn generate(&self, query: &Query, _context: &Context) -> Result<Response> {
        // Record the call
        let mut history = self.call_history.lock().expect("Lock poisoned");
        history.push(query.text.clone());
        drop(history);

        // Find matching response
        let text = if let Some(response) = self.find_response(&query.text) {
            response
        } else {
            let default = self.default_response.lock().expect("Lock poisoned");
            default
                .clone()
                .unwrap_or_else(|| format!("Mock response for query: {}", query.text))
        };

        Ok(Response {
            text,
            confidence: 1.0,
            tokens_used: TokenUsage {
                input: query.text.len() as u64,
                output: 0,
                cache_read: 0,
                cache_write: 0,
            },
            provider: self.name.clone(),
            latency_ms: 0,
        })
    }

    fn estimate_cost(&self, _context: &Context) -> f64 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider_exact_match() {
        let provider = MockProvider::new("test").with_response("hello", "world");

        let query = Query::new("hello".to_owned());
        let context = Context::new("test");

        let response = provider.generate(&query, &context).await.expect("Failed");
        assert_eq!(response.text, "world");
    }

    #[tokio::test]
    async fn test_mock_provider_substring_match() {
        let provider =
            MockProvider::new("test").with_response("implement", "I will implement that feature");

        let query = Query::new("Please implement a new login system".to_owned());
        let context = Context::new("test");

        let response = provider.generate(&query, &context).await.expect("Failed");
        assert_eq!(response.text, "I will implement that feature");
    }

    #[tokio::test]
    async fn test_mock_provider_default_response() {
        let provider = MockProvider::new("test").with_default_response("Default response");

        let query = Query::new("unmatched query".to_owned());
        let context = Context::new("test");

        let response = provider.generate(&query, &context).await.expect("Failed");
        assert_eq!(response.text, "Default response");
    }

    #[tokio::test]
    async fn test_mock_provider_call_history() {
        let provider = MockProvider::new("test");

        let query1 = Query::new("first query".to_owned());
        let query2 = Query::new("second query".to_owned());
        let context = Context::new("test");

        provider.generate(&query1, &context).await.expect("Failed");
        provider.generate(&query2, &context).await.expect("Failed");

        let history = provider.get_call_history().expect("Failed");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0], "first query");
        assert_eq!(history[1], "second query");
    }

    #[tokio::test]
    async fn test_mock_provider_clear_history() {
        let provider = MockProvider::new("test");

        let query = Query::new("test".to_owned());
        let context = Context::new("test");

        provider.generate(&query, &context).await.expect("Failed");
        assert_eq!(provider.call_count(), 1);

        provider.clear_history();
        assert_eq!(provider.call_count(), 0);
    }
}
