//! Mock provider implementation.

use super::strategy::ResponseStrategy;
use async_trait::async_trait;
use merlin_core::{Context, ModelProvider, Query, Response, Result, RoutingError, TokenUsage};
use merlin_deps::tracing;
use std::collections::HashMap;
use std::sync::Mutex;

/// Mock provider with event-mapped response strategies
pub struct MockProvider {
    /// Provider name
    name: &'static str,
    /// Map of event ID to strategies (thread-safe interior mutability)
    strategies: Mutex<HashMap<String, Vec<ResponseStrategy>>>,
    /// Current event ID being processed (thread-safe interior mutability)
    current_event: Mutex<Option<String>>,
}

impl MockProvider {
    /// Create new mock provider
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            strategies: Mutex::new(HashMap::new()),
            current_event: Mutex::new(None),
        }
    }

    /// Set the current event ID being processed
    ///
    /// # Errors
    /// Returns error if lock fails
    pub fn set_current_event(&self, event_id: Option<String>) -> Result<()> {
        *self
            .current_event
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock poisoned: {err}")))? = event_id;
        Ok(())
    }

    /// Register strategies for an event
    ///
    /// # Errors
    /// Returns error if registration fails
    pub fn register_event(&self, event_id: String, strategies: Vec<ResponseStrategy>) -> Result<()> {
        tracing::debug!(
            "Registering event: event_id={:?}, strategies={}",
            event_id,
            strategies.len()
        );
        self.strategies
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock poisoned: {err}")))?
            .insert(event_id, strategies);
        Ok(())
    }

    /// Find and return a matching strategy response for given event
    fn find_match(&self, event_id: &str, query: &Query, context: &Context) -> Result<String> {
        let strategies_map = self
            .strategies
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock poisoned: {err}")))?;

        let Some(strategies) = strategies_map.get(event_id) else {
            return Err(self.generate_event_not_found_error(event_id, &strategies_map));
        };

        // Find first matching strategy
        for (strategy_idx, strategy) in strategies.iter().enumerate() {
            if strategy.matches(query, context) {
                if let Some(typescript) = strategy.get_response() {
                    tracing::info!(
                        "Matched strategy for event={}, strategy={}: {}",
                        event_id,
                        strategy_idx,
                        strategy.description()
                    );
                    return Ok(typescript);
                }
            }
        }

        // No match found in this event's strategies
        Err(self.generate_no_match_error(event_id, strategies, query, context))
    }

    /// Generate error for event not found
    fn generate_event_not_found_error(
        &self,
        event_id: &str,
        strategies_map: &HashMap<String, Vec<ResponseStrategy>>,
    ) -> RoutingError {
        let mut error = format!("Event ID not registered: {:?}\n\n", event_id);
        error.push_str("Available event IDs:\n");
        for registered_id in strategies_map.keys() {
            error.push_str(&format!("  - {:?}\n", registered_id));
        }
        RoutingError::ExecutionFailed(error)
    }

    /// Generate detailed error message for no pattern match
    fn generate_no_match_error(
        &self,
        event_id: &str,
        strategies: &[ResponseStrategy],
        query: &Query,
        context: &Context,
    ) -> RoutingError {
        let mut error = String::new();
        error.push_str(&format!(
            "No matching pattern for event {:?}\n\n",
            event_id
        ));

        // Show query details
        error.push_str(&format!("Query text: {:?}\n", query.text));
        error.push_str(&format!(
            "System prompt: {:?}\n",
            if context.system_prompt.len() > 80 {
                format!("{}...", &context.system_prompt[..80])
            } else {
                context.system_prompt.clone()
            }
        ));

        // Show available patterns for this event
        error.push_str(&format!(
            "\nAvailable patterns for event {:?} ({} strategies):\n",
            event_id,
            strategies.len()
        ));
        for (idx, strategy) in strategies.iter().enumerate() {
            error.push_str(&format!("  {}. {}\n", idx, strategy.description()));
        }

        RoutingError::ExecutionFailed(error)
    }

    /// Clear all registered strategies (for testing)
    pub fn clear(&self) {
        if let Ok(mut map) = self.strategies.lock() {
            map.clear();
        }
    }
}

#[async_trait]
impl ModelProvider for MockProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn generate(&self, query: &Query, context: &Context) -> Result<Response> {
        // Get current event ID (set by test runner)
        let current_event = self
            .current_event
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock poisoned: {err}")))?;

        let event_id = current_event
            .as_ref()
            .ok_or_else(|| RoutingError::Other("No current event set in mock provider".to_owned()))?;

        let typescript_code = self.find_match(event_id, query, context)?;

        Ok(Response {
            text: format!("```typescript\n{typescript_code}\n```"),
            confidence: 1.0,
            tokens_used: TokenUsage {
                input: query.text.len() as u64,
                output: typescript_code.len() as u64,
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
