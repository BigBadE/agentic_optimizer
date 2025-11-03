//! Mock provider for testing.

use super::fixture::{MatchType, TriggerConfig};
use async_trait::async_trait;
use merlin_core::{Context, ModelProvider, Query, Response, Result, RoutingError, TokenUsage};
use merlin_deps::{regex::Regex, tracing};
use merlin_routing::{Model, ModelRouter, RoutingDecision, Task as RoutingTask};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::Mutex as TokioMutex;

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
    /// The prompt that triggered this response (captured when matched)
    captured_prompt: Option<String>,
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
            captured_prompt: None,
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

/// Type alias for event prompt mapping
type EventPromptMap = HashMap<String, String>;

/// Mock provider for testing
pub struct MockProvider {
    /// Provider name
    name: &'static str,
    /// Pattern responses
    responses: Arc<TokioMutex<Vec<PatternResponse>>>,
    /// Call counter for debugging
    call_count: Arc<AtomicUsize>,
    /// Captured prompts for verification
    captured_prompts: Arc<StdMutex<Vec<String>>>,
    /// Event ID to prompt mapping for scoped verification
    event_prompts: Arc<StdMutex<EventPromptMap>>,
    /// Verification errors (root cause messages only)
    verification_errors: Arc<StdMutex<Vec<String>>>,
}

impl MockProvider {
    /// Create new mock provider
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            responses: Arc::new(TokioMutex::new(Vec::new())),
            call_count: Arc::new(AtomicUsize::new(0)),
            captured_prompts: Arc::new(StdMutex::new(Vec::new())),
            event_prompts: Arc::new(StdMutex::new(HashMap::new())),
            verification_errors: Arc::new(StdMutex::new(Vec::new())),
        }
    }

    /// Get prompt for specific event ID
    ///
    /// # Errors
    /// Returns error if lock acquisition fails
    pub fn get_prompt_for_event(&self, event_id: &str) -> Result<Option<String>> {
        self.event_prompts
            .lock()
            .map(|prompts| prompts.get(event_id).cloned())
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))
    }

    /// Capture prompt for specific event ID
    ///
    /// # Errors
    /// Returns error if lock acquisition fails
    pub fn capture_prompt_for_event(&self, event_id: String, prompt: String) -> Result<()> {
        self.event_prompts
            .lock()
            .map(|mut prompts| {
                prompts.insert(event_id, prompt);
            })
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))
    }

    /// Get all captured prompts
    ///
    /// # Errors
    /// Returns error if lock acquisition fails
    pub fn get_captured_prompts(&self) -> Result<Vec<String>> {
        self.captured_prompts
            .lock()
            .map(|prompts| prompts.clone())
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))
    }

    /// Get the last captured prompt
    ///
    /// # Errors
    /// Returns error if lock acquisition fails
    pub fn get_last_prompt(&self) -> Result<Option<String>> {
        self.captured_prompts
            .lock()
            .map(|prompts| prompts.last().cloned())
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))
    }

    /// Get the prompt that triggered the most recent matched response
    ///
    /// # Errors
    /// Returns error if lock acquisition fails
    pub async fn get_last_matched_prompt(&self) -> Result<Option<String>> {
        let responses = self.responses.lock().await;
        Ok(responses
            .iter()
            .rev()
            .find(|resp| resp.used)
            .and_then(|resp| resp.captured_prompt.clone()))
    }

    /// Get all verification errors
    ///
    /// # Errors
    /// Returns error if lock acquisition fails
    pub fn get_verification_errors(&self) -> Result<Vec<String>> {
        self.verification_errors
            .lock()
            .map(|errors| errors.clone())
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))
    }

    /// Add response pattern
    ///
    /// # Errors
    /// Returns error if pattern is invalid
    pub async fn add_response(&self, trigger: &TriggerConfig, typescript: String) -> Result<()> {
        let response = PatternResponse::new(trigger, typescript)?;
        self.responses.lock().await.push(response);
        Ok(())
    }

    /// Reset all patterns to unused (for testing)
    ///
    /// # Errors
    /// Returns error if lock acquisition fails
    pub async fn reset(&self) -> Result<()> {
        {
            let mut responses = self.responses.lock().await;
            for response in responses.iter_mut() {
                response.used = false;
            }
        }
        self.call_count.store(0, Ordering::SeqCst);
        self.captured_prompts
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
            .clear();
        self.event_prompts
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
            .clear();
        self.verification_errors
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
            .clear();
        Ok(())
    }

    /// Find matching pattern from candidates
    ///
    /// # Errors
    /// Returns error if no matching pattern found
    async fn find_matching_pattern(&self, candidates: &[String]) -> Result<(String, String)> {
        let mut responses = self.responses.lock().await;

        let mut matched_prompt = None;
        let mut matched_response = None;

        for (idx, candidate) in candidates.iter().enumerate() {
            if candidate.is_empty() {
                continue;
            }

            if let Some(resp) = responses
                .iter_mut()
                .find(|response| !response.used && response.matches(candidate))
            {
                tracing::info!(
                    "Matched pattern '{}' (match_type={:?}) against candidate #{} (first 80 chars: '{}')",
                    resp.pattern,
                    resp.match_type,
                    idx,
                    candidate.chars().take(80).collect::<String>()
                );
                resp.used = true;
                resp.captured_prompt = Some(candidate.clone());
                matched_prompt = Some(candidate.clone());
                matched_response = Some(resp.typescript.clone());
                break;
            }

            tracing::debug!("No match for candidate #{}", idx);
        }

        drop(responses);

        if let (Some(prompt), Some(response)) = (matched_prompt, matched_response) {
            Ok((prompt, response))
        } else {
            // Extract just the user query for error message
            let user_query = candidates
                .first()
                .filter(|candidate| !candidate.is_empty())
                .unwrap_or(&String::new())
                .clone();

            // Store clean error message for verification
            self.verification_errors
                .lock()
                .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
                .push(user_query.clone());

            Err(RoutingError::ExecutionFailed(format!(
                "No matching pattern for query: {user_query}"
            )))
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
        self.call_count.fetch_add(1, Ordering::SeqCst);

        // Try multiple prompt representations
        let candidates = [
            query.text.clone(),
            context.system_prompt.clone(),
            if !context.system_prompt.is_empty() && !query.text.is_empty() {
                format!("{}\n\n{}", context.system_prompt, query.text)
            } else {
                String::new()
            },
        ];

        let (prompt_for_matching, typescript_code) = self.find_matching_pattern(&candidates).await?;

        // Capture matched prompt
        self.captured_prompts
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
            .push(prompt_for_matching.clone());

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

/// Mock router that can simulate multi-tier routing and escalation
pub struct MockRouter {
    /// Current tier being used (increments on each route call for escalation testing)
    tier: Arc<AtomicUsize>,
    /// Available tiers (provider names in order of escalation)
    tiers: Vec<&'static str>,
}

impl MockRouter {
    /// Create a new mock router with default single tier
    #[must_use]
    pub fn new() -> Self {
        Self {
            tier: Arc::new(AtomicUsize::new(0)),
            tiers: vec!["test-mock"],
        }
    }
}

impl Default for MockRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelRouter for MockRouter {
    async fn route(&self, _task: &RoutingTask) -> Result<RoutingDecision> {
        let tier_idx = self.tier.fetch_add(1, Ordering::SeqCst);
        let tier_name = self
            .tiers
            .get(tier_idx.min(self.tiers.len() - 1))
            .unwrap_or(&"test-mock");

        tracing::debug!("MockRouter routing to tier {} ({})", tier_idx, tier_name);

        // Always route to the default model for tests
        Ok(RoutingDecision {
            model: Model::Qwen25Coder32B,
            estimated_cost: 0.0,
            estimated_latency_ms: 100,
            reasoning: format!("Test routing to tier {tier_idx}"),
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
    /// # Errors
    /// Returns error if `PatternResponse` creation fails
    ///
    /// # Panics
    /// Panics if assertions fail during test execution
    #[test]
    fn test_pattern_response_exact_match() -> Result<()> {
        let trigger = TriggerConfig {
            pattern: "hello world".to_owned(),
            match_type: MatchType::Exact,
        };
        let response = PatternResponse::new(&trigger, "test".to_owned())?;
        assert!(response.matches("hello world"));
        assert!(!response.matches("hello"));
        assert!(!response.matches("hello world!"));
        Ok(())
    }

    /// Tests contains pattern matching
    ///
    /// # Errors
    /// Returns error if `PatternResponse` creation fails
    ///
    /// # Panics
    /// Panics if assertions fail during test execution
    #[test]
    fn test_pattern_response_contains_match() -> Result<()> {
        let trigger = TriggerConfig {
            pattern: "world".to_owned(),
            match_type: MatchType::Contains,
        };
        let response = PatternResponse::new(&trigger, "test".to_owned())?;
        assert!(response.matches("hello world"));
        assert!(response.matches("world"));
        assert!(response.matches("world hello"));
        assert!(!response.matches("hello"));
        Ok(())
    }

    /// Tests regex pattern matching
    ///
    /// # Errors
    /// Returns error if `PatternResponse` creation fails
    ///
    /// # Panics
    /// Panics if assertions fail during test execution
    #[test]
    fn test_pattern_response_regex_match() -> Result<()> {
        let trigger = TriggerConfig {
            pattern: r"hello\s+\w+".to_owned(),
            match_type: MatchType::Regex,
        };
        let response = PatternResponse::new(&trigger, "test".to_owned())?;
        assert!(response.matches("hello world"));
        assert!(response.matches("hello there"));
        assert!(!response.matches("hello"));
        assert!(!response.matches("helloworld"));
        Ok(())
    }
}
