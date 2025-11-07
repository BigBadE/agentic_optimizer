//! Response strategy types for mock provider.

use merlin_core::{Context, Query, Result, RoutingError};
use merlin_deps::regex::Regex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Match against specific part of query
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchAgainst {
    /// Match against query text only
    Query,
    /// Match against system prompt only
    System,
    /// Match against combined system + query
    Combined,
}

/// Match type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    /// Exact string match
    Exact,
    /// Contains substring
    Contains,
    /// Regex match
    Regex,
}

/// Pattern trigger configuration
#[derive(Debug, Clone)]
pub struct TriggerConfig {
    /// Pattern to match
    pub pattern: String,
    /// Match type
    pub match_type: MatchType,
    /// What to match against
    pub match_against: MatchAgainst,
    /// Compiled regex (cached)
    regex: Option<Regex>,
}

impl TriggerConfig {
    /// Create new trigger config
    ///
    /// # Errors
    /// Returns error if regex compilation fails
    pub fn new(pattern: String, match_type: MatchType, match_against: MatchAgainst) -> Result<Self> {
        let regex = if matches!(match_type, MatchType::Regex) {
            Some(Regex::new(&pattern)
                .map_err(|err| RoutingError::InvalidTask(format!("Invalid regex: {err}")))?)
        } else {
            None
        };

        Ok(Self {
            pattern,
            match_type,
            match_against,
            regex,
        })
    }

    /// Check if pattern matches text
    pub(super) fn matches(&self, text: &str) -> bool {
        match self.match_type {
            MatchType::Exact => text == self.pattern,
            MatchType::Contains => text.contains(&self.pattern),
            MatchType::Regex => self.regex.as_ref().is_some_and(|regex| regex.is_match(text)),
        }
    }

    /// Get text to match against from query/context
    pub(super) fn extract_text<'a>(&self, query: &'a Query, context: &'a Context) -> &'a str {
        match self.match_against {
            MatchAgainst::Query => &query.text,
            MatchAgainst::System => &context.system_prompt,
            MatchAgainst::Combined => {
                // For combined, we need to use query text as fallback
                // This is a limitation - we'll handle it in the provider
                &query.text
            }
        }
    }
}

/// Response strategy for LLM mocking (no exhaustion tracking)
#[derive(Debug)]
pub enum ResponseStrategy {
    /// Single response (always returns same value)
    Once {
        /// Trigger configuration
        trigger: TriggerConfig,
        /// TypeScript code to return
        typescript: String,
    },

    /// Sequence of responses (cycles through, tracked by atomic counter)
    Sequence {
        /// Trigger configuration
        trigger: TriggerConfig,
        /// Responses in order
        responses: Vec<String>,
        /// Current index (atomic for thread safety)
        index: AtomicUsize,
    },

    /// Repeating response (infinite reuse)
    Repeating {
        /// Trigger configuration
        trigger: TriggerConfig,
        /// TypeScript code to return
        typescript: String,
    },
}

impl ResponseStrategy {
    /// Create a sequence strategy
    pub fn new_sequence(trigger: TriggerConfig, responses: Vec<String>) -> Self {
        Self::Sequence {
            trigger,
            responses,
            index: AtomicUsize::new(0),
        }
    }

    /// Get the trigger for this strategy
    pub(super) fn trigger(&self) -> &TriggerConfig {
        match self {
            Self::Once { trigger, .. }
            | Self::Sequence { trigger, .. }
            | Self::Repeating { trigger, .. } => trigger,
        }
    }

    /// Check if this strategy matches the query
    pub(super) fn matches(&self, query: &Query, context: &Context) -> bool {
        let trigger = self.trigger();

        // Handle Combined match type specially
        if matches!(trigger.match_against, MatchAgainst::Combined) {
            if context.system_prompt.is_empty() {
                return trigger.matches(&query.text);
            }
            if query.text.is_empty() {
                return trigger.matches(&context.system_prompt);
            }
            let combined = format!("{}\n\n{}", context.system_prompt, query.text);
            return trigger.matches(&combined);
        }

        let text = trigger.extract_text(query, context);
        trigger.matches(text)
    }

    /// Get response (no exhaustion - always returns something)
    pub(super) fn get_response(&self) -> Option<String> {
        match self {
            // Always clone, never exhaust
            Self::Once { typescript, .. } => Some(typescript.clone()),

            // Cycle through responses using atomic counter
            Self::Sequence { responses, index, .. } => {
                if responses.is_empty() {
                    return None;
                }
                let idx = index.fetch_add(1, Ordering::SeqCst);
                // Wrap around if we exceed length
                let wrapped_idx = idx % responses.len();
                responses.get(wrapped_idx).cloned()
            }

            Self::Repeating { typescript, .. } => Some(typescript.clone()),
        }
    }

    /// Get description for diagnostics
    pub(super) fn description(&self) -> String {
        match self {
            Self::Once { trigger, .. } => {
                format!("[once] pattern='{}' match={:?} against={:?}",
                    trigger.pattern,
                    trigger.match_type,
                    trigger.match_against)
            }
            Self::Sequence { trigger, responses, index } => {
                let current = index.load(Ordering::SeqCst);
                format!("[sequence, call #{} of {}] pattern='{}' match={:?} against={:?}",
                    current + 1,
                    responses.len(),
                    trigger.pattern,
                    trigger.match_type,
                    trigger.match_against)
            }
            Self::Repeating { trigger, .. } => {
                format!("[repeating] pattern='{}' match={:?} against={:?}",
                    trigger.pattern,
                    trigger.match_type,
                    trigger.match_against)
            }
        }
    }
}

// Manual Clone implementation because AtomicUsize doesn't implement Clone
impl Clone for ResponseStrategy {
    fn clone(&self) -> Self {
        match self {
            Self::Once { trigger, typescript } => Self::Once {
                trigger: trigger.clone(),
                typescript: typescript.clone(),
            },
            Self::Sequence { trigger, responses, index } => Self::Sequence {
                trigger: trigger.clone(),
                responses: responses.clone(),
                index: AtomicUsize::new(index.load(Ordering::SeqCst)),
            },
            Self::Repeating { trigger, typescript } => Self::Repeating {
                trigger: trigger.clone(),
                typescript: typescript.clone(),
            },
        }
    }
}
