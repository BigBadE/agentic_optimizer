//! Mock provider for testing agent responses.
//!
//! Allows defining canned responses for specific queries, enabling
//! end-to-end testing of agent workflows without real API calls.

use async_trait::async_trait;
use merlin_core::{Context, ModelProvider, Query, Response, Result, TokenUsage};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Response storage type
type ResponseMap = Arc<Mutex<HashMap<String, String>>>;

/// Mock provider that returns pre-defined responses based on query patterns.
///
/// Useful for testing agent workflows end-to-end without making real API calls.
#[derive(Clone)]
pub struct MockProvider {
    /// Name of this mock provider
    name: String,
    /// Predefined responses keyed by query text
    responses: ResponseMap,
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
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (another thread panicked while holding the lock).
    #[must_use]
    pub fn with_response(self, pattern: impl Into<String>, response: impl Into<String>) -> Self {
        match self.responses.lock() {
            Ok(mut responses) => {
                responses.insert(pattern.into(), response.into());
            }
            Err(err) => panic!("Lock poisoned: {err}"),
        }
        self
    }

    /// Set a default response for queries that don't match any pattern.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (another thread panicked while holding the lock).
    #[must_use]
    pub fn with_default_response(self, response: impl Into<String>) -> Self {
        match self.default_response.lock() {
            Ok(mut default) => {
                *default = Some(response.into());
            }
            Err(err) => panic!("Lock poisoned: {err}"),
        }
        self
    }

    /// Clear the call history (used for testing).
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (another thread panicked while holding the lock).
    pub fn clear_history(&self) {
        match self.call_history.lock() {
            Ok(mut history) => history.clear(),
            Err(err) => panic!("Lock poisoned: {err}"),
        }
    }

    /// Get the call history (list of all queries made).
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (another thread panicked while holding the lock).
    #[must_use]
    pub fn get_call_history(&self) -> Vec<String> {
        match self.call_history.lock() {
            Ok(history) => history.clone(),
            Err(err) => panic!("Lock poisoned: {err}"),
        }
    }

    /// Get the number of calls made.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (another thread panicked while holding the lock).
    #[must_use]
    pub fn call_count(&self) -> usize {
        match self.call_history.lock() {
            Ok(history) => history.len(),
            Err(err) => panic!("Lock poisoned: {err}"),
        }
    }

    /// Find a matching response for the given query text.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (another thread panicked while holding the lock).
    fn find_response(&self, query_text: &str) -> Option<String> {
        let responses = match self.responses.lock() {
            Ok(res) => res,
            Err(err) => panic!("Lock poisoned: {err}"),
        };

        // Try exact match first
        if let Some(response) = responses.get(query_text) {
            let result = response.clone();
            drop(responses);
            return Some(result);
        }

        // Try substring match
        for (pattern, response) in &*responses {
            if query_text.contains(pattern) {
                let result = response.clone();
                drop(responses);
                return Some(result);
            }
        }

        drop(responses);
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
        match self.call_history.lock() {
            Ok(mut history) => history.push(query.text.clone()),
            Err(err) => panic!("Lock poisoned: {err}"),
        }

        // Find matching response
        let text = self.find_response(&query.text).unwrap_or_else(|| {
            let default = match self.default_response.lock() {
                Ok(def) => def,
                Err(err) => panic!("Lock poisoned: {err}"),
            };
            default
                .clone()
                .unwrap_or_else(|| format!("Mock response for query: {}", query.text))
        });

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

        let response = provider.generate(&query, &context).await;
        assert!(response.is_ok(), "Failed to generate response");
        if let Ok(resp) = response {
            assert_eq!(resp.text, "world");
        }
    }

    #[tokio::test]
    async fn test_mock_provider_substring_match() {
        let provider =
            MockProvider::new("test").with_response("implement", "I will implement that feature");

        let query = Query::new("Please implement a new login system".to_owned());
        let context = Context::new("test");

        let response = provider.generate(&query, &context).await;
        assert!(response.is_ok(), "Failed to generate response");
        if let Ok(resp) = response {
            assert_eq!(resp.text, "I will implement that feature");
        }
    }

    #[tokio::test]
    async fn test_mock_provider_default_response() {
        let provider = MockProvider::new("test").with_default_response("Default response");

        let query = Query::new("unmatched query".to_owned());
        let context = Context::new("test");

        let response = provider.generate(&query, &context).await;
        assert!(response.is_ok(), "Failed to generate response");
        if let Ok(resp) = response {
            assert_eq!(resp.text, "Default response");
        }
    }

    #[tokio::test]
    async fn test_mock_provider_call_history() {
        let provider = MockProvider::new("test");

        let query1 = Query::new("first query".to_owned());
        let query2 = Query::new("second query".to_owned());
        let context = Context::new("test");

        let res1 = provider.generate(&query1, &context).await;
        assert!(res1.is_ok(), "Failed to generate first response");
        let res2 = provider.generate(&query2, &context).await;
        assert!(res2.is_ok(), "Failed to generate second response");

        let history = provider.get_call_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0], "first query");
        assert_eq!(history[1], "second query");
    }

    #[tokio::test]
    async fn test_mock_provider_clear_history() {
        let provider = MockProvider::new("test");

        let query = Query::new("test".to_owned());
        let context = Context::new("test");

        let res = provider.generate(&query, &context).await;
        assert!(res.is_ok(), "Failed to generate response");
        assert_eq!(provider.call_count(), 1);

        provider.clear_history();
        assert_eq!(provider.call_count(), 0);
    }
}
