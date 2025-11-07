//! Response strategy types for mock provider.

use super::routing_matcher::RoutingMatcher;
use merlin_core::{Context, Query};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Response strategy for LLM mocking (no exhaustion tracking)
#[derive(Debug)]
pub enum ResponseStrategy {
    /// Single response (always returns same value)
    Once {
        /// Routing matcher (if None, matches any)
        routing_match: Option<RoutingMatcher>,
        /// TypeScript code to return
        typescript: String,
    },

    /// Sequence of responses (cycles through, tracked by atomic counter)
    Sequence {
        /// Routing matcher (if None, matches any)
        routing_match: Option<RoutingMatcher>,
        /// Responses in order
        responses: Vec<String>,
        /// Current index (atomic for thread safety)
        index: AtomicUsize,
    },

    /// Repeating response (infinite reuse)
    Repeating {
        /// Routing matcher (if None, matches any)
        routing_match: Option<RoutingMatcher>,
        /// TypeScript code to return
        typescript: String,
    },
}

impl ResponseStrategy {
    /// Get the routing matcher for this strategy
    pub(super) fn routing_matcher(&self) -> Option<&RoutingMatcher> {
        match self {
            Self::Once { routing_match, .. }
            | Self::Sequence { routing_match, .. }
            | Self::Repeating { routing_match, .. } => routing_match.as_ref(),
        }
    }

    /// Check if this strategy matches the query
    pub(super) fn matches(&self, query: &Query, context: &Context) -> bool {
        self.routing_matcher()
            .is_none_or(|routing_matcher| routing_matcher.matches(query, context))
    }

    /// Get response (no exhaustion - always returns something)
    pub(super) fn get_response(&self) -> Option<String> {
        match self {
            // Cycle through responses using atomic counter
            Self::Sequence {
                responses, index, ..
            } => {
                if responses.is_empty() {
                    return None;
                }
                let idx = index.fetch_add(1, Ordering::SeqCst);
                // Wrap around if we exceed length
                let wrapped_idx = idx % responses.len();
                responses.get(wrapped_idx).cloned()
            }

            // Always clone, never exhaust (Once and Repeating have same behavior)
            Self::Once { typescript, .. } | Self::Repeating { typescript, .. } => {
                Some(typescript.clone())
            }
        }
    }

    /// Get description for diagnostics
    pub(super) fn description(&self) -> String {
        let strategy_type = match self {
            Self::Once { .. } => "once",
            Self::Sequence {
                index, responses, ..
            } => {
                let current = index.load(Ordering::SeqCst);
                return format!(
                    "[sequence, call #{} of {}] {}",
                    current + 1,
                    responses.len(),
                    self.matcher_description()
                );
            }
            Self::Repeating { .. } => "repeating",
        };

        format!("[{}] {}", strategy_type, self.matcher_description())
    }

    /// Get description of the matcher for diagnostics
    fn matcher_description(&self) -> String {
        self.routing_matcher().map_or_else(
            || "matches any".to_owned(),
            super::routing_matcher::RoutingMatcher::description,
        )
    }
}

// Manual Clone implementation because AtomicUsize doesn't implement Clone
impl Clone for ResponseStrategy {
    fn clone(&self) -> Self {
        match self {
            Self::Once {
                routing_match,
                typescript,
            } => Self::Once {
                routing_match: routing_match.clone(),
                typescript: typescript.clone(),
            },
            Self::Sequence {
                routing_match,
                responses,
                index,
            } => Self::Sequence {
                routing_match: routing_match.clone(),
                responses: responses.clone(),
                index: AtomicUsize::new(index.load(Ordering::SeqCst)),
            },
            Self::Repeating {
                routing_match,
                typescript,
            } => Self::Repeating {
                routing_match: routing_match.clone(),
                typescript: typescript.clone(),
            },
        }
    }
}
