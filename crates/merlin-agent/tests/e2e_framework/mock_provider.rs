//! Stateful mock provider with comprehensive call tracking and verification.

use super::fixture::MockResponse;
use async_trait::async_trait;
use merlin_core::{Context, ModelProvider, Query, Response, Result, RoutingError, TokenUsage};
use std::sync::{Arc, Mutex};

/// Detailed call record for verification
#[derive(Debug, Clone)]
pub struct CallRecord {
    /// The query text
    pub query: String,
    /// The response pattern that matched
    pub matched_pattern: String,
    /// The response text returned
    pub response: String,
    /// Timestamp of the call
    pub timestamp: std::time::Instant,
    /// Whether the call resulted in an error
    pub was_error: bool,
}

/// Internal state for the mock provider
#[derive(Debug)]
struct ProviderState {
    /// Mock responses available
    responses: Vec<MockResponse>,
    /// Call history
    calls: Vec<CallRecord>,
    /// Count of how many times each pattern has been used
    pattern_use_count: std::collections::HashMap<String, usize>,
}

/// Stateful mock provider with comprehensive tracking
#[derive(Clone)]
pub struct StatefulMockProvider {
    name: String,
    state: Arc<Mutex<ProviderState>>,
}

impl StatefulMockProvider {
    /// Create a new stateful mock provider
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            state: Arc::new(Mutex::new(ProviderState {
                responses: Vec::new(),
                calls: Vec::new(),
                pattern_use_count: std::collections::HashMap::new(),
            })),
        }
    }

    /// Add a mock response
    #[allow(clippy::needless_pass_by_ref_mut, reason = "API consistency")]
    pub fn add_response(&mut self, response: MockResponse) {
        let mut state = self.state.lock().expect("Lock poisoned");
        state.responses.push(response);
    }

    /// Add multiple mock responses
    #[allow(clippy::needless_pass_by_ref_mut, reason = "API consistency")]
    pub fn add_responses(&mut self, responses: Vec<MockResponse>) {
        let mut state = self.state.lock().expect("Lock poisoned");
        state.responses.extend(responses);
    }

    /// Get the call history
    #[must_use]
    pub fn get_call_history(&self) -> Vec<CallRecord> {
        let state = self.state.lock().expect("Lock poisoned");
        state.calls.clone()
    }

    /// Get the number of calls made
    #[must_use]
    pub fn call_count(&self) -> usize {
        let state = self.state.lock().expect("Lock poisoned");
        state.calls.len()
    }

    /// Clear the call history
    pub fn clear_history(&self) {
        let mut state = self.state.lock().expect("Lock poisoned");
        state.calls.clear();
        state.pattern_use_count.clear();
    }

    /// Get pattern use counts
    #[must_use]
    pub fn get_pattern_use_counts(&self) -> std::collections::HashMap<String, usize> {
        let state = self.state.lock().expect("Lock poisoned");
        state.pattern_use_count.clone()
    }

    /// Find a matching response for the given query
    fn find_response(query_text: &str, state: &mut ProviderState) -> Option<MockResponse> {
        // Try to find a matching response
        for (idx, mock_response) in state.responses.iter().enumerate() {
            // Check if pattern matches
            let matches = if mock_response.pattern.is_empty() {
                true // Empty pattern matches everything (default response)
            } else {
                query_text.contains(&mock_response.pattern)
            };

            if !matches {
                continue;
            }

            // Check if this is a use_once response that's already been used
            if mock_response.use_once {
                let use_count = state
                    .pattern_use_count
                    .get(&mock_response.pattern)
                    .copied()
                    .unwrap_or(0);
                if use_count > 0 {
                    continue; // Already used, skip it
                }
            }

            // Found a match, increment use count and return
            *state
                .pattern_use_count
                .entry(mock_response.pattern.clone())
                .or_insert(0) += 1;

            return Some(state.responses[idx].clone());
        }

        None
    }
}

#[async_trait]
impl ModelProvider for StatefulMockProvider {
    fn name(&self) -> &'static str {
        "stateful_mock"
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn generate(&self, query: &Query, _context: &Context) -> Result<Response> {
        let mut state = self.state.lock().expect("Lock poisoned");

        // Find matching response
        let mock_response = Self::find_response(&query.text, &mut state);

        if let Some(mock) = mock_response {
            // Check if this response should fail
            if mock.should_fail {
                let error_msg = mock.error_message.unwrap_or_else(String::new);

                // Record the error call
                state.calls.push(CallRecord {
                    query: query.text.clone(),
                    matched_pattern: mock.pattern,
                    response: error_msg.clone(),
                    timestamp: std::time::Instant::now(),
                    was_error: true,
                });

                return Err(RoutingError::Other(error_msg));
            }

            // Record successful call
            state.calls.push(CallRecord {
                query: query.text.clone(),
                matched_pattern: mock.pattern.clone(),
                response: mock.response.clone(),
                timestamp: std::time::Instant::now(),
                was_error: false,
            });

            Ok(Response {
                text: mock.response.clone(),
                confidence: 1.0,
                tokens_used: TokenUsage {
                    input: query.text.len() as u64,
                    output: mock.response.len() as u64,
                    cache_read: 0,
                    cache_write: 0,
                },
                provider: self.name.clone(),
                latency_ms: 0,
            })
        } else {
            // No matching response found - this is a test failure
            let error_msg = format!(
                "No mock response found for query (patterns tried: {}): {}",
                state
                    .responses
                    .iter()
                    .map(|r| r.pattern.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                query.text
            );

            state.calls.push(CallRecord {
                query: query.text.clone(),
                matched_pattern: String::new(),
                response: error_msg.clone(),
                timestamp: std::time::Instant::now(),
                was_error: true,
            });
            drop(state);

            Err(RoutingError::Other(error_msg))
        }
    }

    fn estimate_cost(&self, _context: &Context) -> f64 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stateful_mock_provider_basic() {
        let mut provider = StatefulMockProvider::new("test");
        provider.add_response(MockResponse {
            pattern: "hello".to_owned(),
            response: "world".to_owned(),
            expected_tool_calls: vec![],
            use_once: false,
            should_fail: false,
            error_message: None,
        });

        let query = Query::new("hello".to_owned());
        let context = Context::new("test");

        let response = provider.generate(&query, &context).await.expect("Failed");
        assert_eq!(response.text, "world");
        assert_eq!(provider.call_count(), 1);
    }

    #[tokio::test]
    async fn test_use_once_response() {
        let mut provider = StatefulMockProvider::new("test");
        provider.add_response(MockResponse {
            pattern: "once".to_owned(),
            response: "first".to_owned(),
            expected_tool_calls: vec![],
            use_once: true,
            should_fail: false,
            error_message: None,
        });
        provider.add_response(MockResponse {
            pattern: "once".to_owned(),
            response: "second".to_owned(),
            expected_tool_calls: vec![],
            use_once: false,
            should_fail: false,
            error_message: None,
        });

        let query = Query::new("once".to_owned());
        let context = Context::new("test");

        let response1 = provider.generate(&query, &context).await.expect("Failed");
        assert_eq!(response1.text, "first");

        let response2 = provider.generate(&query, &context).await.expect("Failed");
        assert_eq!(response2.text, "second");
    }

    #[tokio::test]
    async fn test_failing_response() {
        let mut provider = StatefulMockProvider::new("test");
        provider.add_response(MockResponse {
            pattern: "fail".to_owned(),
            response: String::new(),
            expected_tool_calls: vec![],
            use_once: false,
            should_fail: true,
            error_message: Some("Test error".to_owned()),
        });

        let query = Query::new("fail".to_owned());
        let context = Context::new("test");

        let result = provider.generate(&query, &context).await;
        result.unwrap_err();

        let history = provider.get_call_history();
        assert_eq!(history.len(), 1);
        assert!(history[0].was_error);
    }
}
