//! Metrics collection for tracking task execution statistics.

use merlin_core::TokenUsage;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Metrics for a single request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetrics {
    /// When the request was made
    pub timestamp: SystemTime,
    /// The query text
    pub query: String,
    /// Model tier used
    pub tier_used: String,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Tokens used
    pub tokens_used: TokenUsage,
    /// Estimated cost in USD
    pub cost: f64,
    /// Whether the request succeeded
    pub success: bool,
    /// Whether the request was escalated to a higher tier
    pub escalated: bool,
}

/// Builder for creating request metrics
pub struct RequestMetricsBuilder {
    query: String,
    tier_used: String,
    latency_ms: u64,
    tokens_used: TokenUsage,
    success: bool,
    escalated: bool,
}

impl RequestMetricsBuilder {
    /// Creates a new builder
    pub fn new(query: String, tier_used: String) -> Self {
        Self {
            query,
            tier_used,
            latency_ms: 0,
            tokens_used: TokenUsage::default(),
            success: true,
            escalated: false,
        }
    }

    /// Sets the latency
    #[must_use]
    pub fn latency_ms(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    /// Sets the token usage
    #[must_use]
    pub fn tokens_used(mut self, tokens_used: TokenUsage) -> Self {
        self.tokens_used = tokens_used;
        self
    }

    /// Sets the success flag
    #[must_use]
    pub fn success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }

    /// Sets the escalated flag
    #[must_use]
    pub fn escalated(mut self, escalated: bool) -> Self {
        self.escalated = escalated;
        self
    }

    /// Builds the request metrics
    pub fn build(self) -> RequestMetrics {
        let cost = RequestMetrics::estimate_cost(&self.tier_used, &self.tokens_used);

        RequestMetrics {
            timestamp: SystemTime::now(),
            query: self.query,
            tier_used: self.tier_used,
            latency_ms: self.latency_ms,
            tokens_used: self.tokens_used,
            cost,
            success: self.success,
            escalated: self.escalated,
        }
    }
}

/// Parameters for creating request metrics
pub struct RequestMetricsParams {
    /// The query text
    pub query: String,
    /// Model tier used
    pub tier_used: String,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Tokens used
    pub tokens_used: TokenUsage,
    /// Whether the request succeeded
    pub success: bool,
    /// Whether the request was escalated to a higher tier
    pub escalated: bool,
}

impl RequestMetrics {
    /// Creates new request metrics
    pub fn new(params: RequestMetricsParams) -> Self {
        RequestMetricsBuilder::new(params.query, params.tier_used)
            .latency_ms(params.latency_ms)
            .tokens_used(params.tokens_used)
            .success(params.success)
            .escalated(params.escalated)
            .build()
    }

    /// Estimates cost based on tier and token usage
    fn estimate_cost(tier: &str, tokens: &TokenUsage) -> f64 {
        // Cost estimates per 1M tokens (input/output)
        let (input_cost, output_cost) = match tier {
            tier if tier.contains("local") => (0.0, 0.0), // Local models are free
            tier if tier.contains("groq") => (0.0, 0.0),  // Groq free tier
            tier if tier.contains("claude") => (3.0, 15.0), // Claude Sonnet pricing
            tier if tier.contains("deepseek") => (0.27, 1.1), // DeepSeek pricing
            _ => (1.0, 3.0),                              // Default estimate
        };

        let input_tokens = tokens.input as f64;
        let output_tokens = tokens.output as f64;

        (input_tokens / 1_000_000.0)
            .mul_add(input_cost, (output_tokens / 1_000_000.0) * output_cost)
    }
}

/// Collects and stores metrics for analysis
pub struct MetricsCollector {
    requests: Vec<RequestMetrics>,
}

impl MetricsCollector {
    /// Creates a new metrics collector
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    /// Records a request
    pub fn record(&mut self, metrics: RequestMetrics) {
        self.requests.push(metrics);
    }

    /// Gets all recorded requests
    pub fn requests(&self) -> &[RequestMetrics] {
        &self.requests
    }

    /// Gets requests from today
    pub fn requests_today(&self) -> Vec<&RequestMetrics> {
        let now = SystemTime::now();
        let day_start = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| {
                let secs = duration.as_secs();
                let day_secs = secs - (secs % 86400);
                SystemTime::UNIX_EPOCH + Duration::from_secs(day_secs)
            })
            .unwrap_or(SystemTime::UNIX_EPOCH);

        self.requests
            .iter()
            .filter(|req| req.timestamp >= day_start)
            .collect()
    }

    /// Gets requests from the past week
    pub fn requests_this_week(&self) -> Vec<&RequestMetrics> {
        let now = SystemTime::now();
        let week_ago = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| SystemTime::UNIX_EPOCH + duration - Duration::from_secs(7 * 86400))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        self.requests
            .iter()
            .filter(|req| req.timestamp >= week_ago)
            .collect()
    }

    /// Clears all metrics
    pub fn clear(&mut self) {
        self.requests.clear();
    }

    /// Returns the number of recorded requests
    pub fn len(&self) -> usize {
        self.requests.len()
    }

    /// Returns whether the collector is empty
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests basic metrics collection functionality.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::new();

        let metrics = RequestMetrics::new(RequestMetricsParams {
            query: "test query".to_owned(),
            tier_used: "local".to_owned(),
            latency_ms: 100,
            tokens_used: TokenUsage::default(),
            success: true,
            escalated: false,
        });

        collector.record(metrics);
        assert_eq!(collector.len(), 1);
    }

    /// Tests cost estimation for different model tiers.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_cost_estimation() {
        let tokens = TokenUsage {
            input: 1000,
            output: 500,
            cache_read: 0,
            cache_write: 0,
        };

        let local_cost = RequestMetrics::estimate_cost("local", &tokens);
        assert!((local_cost - 0.0).abs() < f64::EPSILON);

        let claude_cost = RequestMetrics::estimate_cost("claude", &tokens);
        assert!(claude_cost > f64::EPSILON);
    }

    /// Tests filtering requests from today.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_requests_today() {
        let mut collector = MetricsCollector::new();

        let metrics = RequestMetrics::new(RequestMetricsParams {
            query: "test".to_owned(),
            tier_used: "local".to_owned(),
            latency_ms: 100,
            tokens_used: TokenUsage::default(),
            success: true,
            escalated: false,
        });

        collector.record(metrics);

        let today = collector.requests_today();
        assert_eq!(today.len(), 1);
    }
}
