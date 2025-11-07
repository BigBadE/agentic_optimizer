//! Mock provider implementation.

use super::strategy::ResponseStrategy;
use async_trait::async_trait;
use merlin_core::{Context, ModelProvider, Query, Response, Result, RoutingError, TokenUsage};
use merlin_deps::tracing;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Mutex;

/// Type alias for the strategies map
type StrategyMap = HashMap<String, Vec<ResponseStrategy>>;

/// Helper to ignore writeln! results without clippy warnings
fn write_diagnostic<T: Display>(target: &mut String, content: T) {
    use std::fmt::Write as _;
    if let Err(err) = writeln!(target, "{content}") {
        // Formatting into a String should never fail, but handle it defensively
        tracing::warn!("Failed to format diagnostic message: {err}");
    }
}

/// Mock provider with event-mapped response strategies
pub struct MockProvider {
    /// Provider name
    name: &'static str,
    /// Map of event ID to strategies (thread-safe interior mutability)
    strategies: Mutex<StrategyMap>,
    /// Current event ID being processed (thread-safe interior mutability)
    current_event: Mutex<Option<String>>,
}

impl MockProvider {
    /// Create new mock provider with pre-built strategy map
    #[must_use]
    pub fn new(name: &'static str, strategies: StrategyMap) -> Self {
        Self {
            name,
            strategies: Mutex::new(strategies),
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

    /// Find and return a matching strategy response for given event
    ///
    /// # Errors
    /// Returns error if event is not found or no matching strategy is found
    fn find_match(&self, event_id: &str, query: &Query, context: &Context) -> Result<String> {
        let strategies_map = self
            .strategies
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock poisoned: {err}")))?;

        let Some(strategies) = strategies_map.get(event_id) else {
            return Err(Self::generate_event_not_found_error(
                event_id,
                &strategies_map,
            ));
        };

        // Find first matching strategy
        for (strategy_idx, strategy) in strategies.iter().enumerate() {
            if strategy.matches(query, context)
                && let Some(typescript) = strategy.get_response()
            {
                tracing::info!(
                    "Matched strategy for event={}, strategy={}: {}",
                    event_id,
                    strategy_idx,
                    strategy.description()
                );
                return Ok(typescript);
            }
        }

        // No match found in this event's strategies
        Err(Self::generate_no_match_error(
            event_id, strategies, query, context,
        ))
    }

    /// Generate error for event not found
    fn generate_event_not_found_error(
        event_id: &str,
        strategies_map: &StrategyMap,
    ) -> RoutingError {
        let mut error = format!("Event ID not registered: {event_id:?}\n\n");
        error.push_str("Available event IDs:\n");
        for registered_id in strategies_map.keys() {
            write_diagnostic(&mut error, format!("  - {registered_id:?}"));
        }
        RoutingError::ExecutionFailed(error)
    }

    /// Generate detailed error message for no pattern match
    fn generate_no_match_error(
        event_id: &str,
        strategies: &[ResponseStrategy],
        query: &Query,
        context: &Context,
    ) -> RoutingError {
        let mut error = format!("No matching pattern for event {event_id:?}\n\n");

        // Show query details
        write_diagnostic(&mut error, format!("Query text: {:?}", query.text));
        let system_preview = if context.system_prompt.len() > 80 {
            format!("{}...", &context.system_prompt[..80])
        } else {
            context.system_prompt.clone()
        };
        write_diagnostic(&mut error, format!("System prompt: {system_preview:?}"));

        // Show available patterns for this event
        write_diagnostic(
            &mut error,
            format!(
                "\nAvailable patterns for event {event_id:?} ({} strategies):",
                strategies.len()
            ),
        );
        for (idx, strategy) in strategies.iter().enumerate() {
            write_diagnostic(&mut error, format!("  {idx}. {}", strategy.description()));
        }

        RoutingError::ExecutionFailed(error)
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
        // Get current event ID (must be set by test runner - no fallback)
        let current_event = self
            .current_event
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock poisoned: {err}")))?;

        let event_id = current_event.as_ref().ok_or_else(|| {
            RoutingError::ExecutionFailed(
                "No current event set. Test runner must call set_current_event() before LLM queries.".to_owned()
            )
        })?.clone();

        drop(current_event);

        let typescript_code = self.find_match(&event_id, query, context)?;

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
