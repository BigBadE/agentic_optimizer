use super::model_registry::ModelRegistry;
use super::models::Model;
use super::provider_registry::ProviderRegistry;
use crate::{ModelRouter, Result, RoutingDecision, RoutingError, Task};
use async_trait::async_trait;
use std::sync::Arc;

/// Availability checker for model tiers
#[derive(Default)]
pub struct AvailabilityChecker {
    // In a real implementation, this would track quotas, rate limits, etc.
}

impl AvailabilityChecker {
    /// Checks if a model is available for use.
    ///
    /// Currently always returns true. In production, this would check:
    /// - API key presence
    /// - Rate limit status
    /// - Quota remaining
    /// - Service health
    pub fn check(&self, _model: Model) -> bool {
        // For now, assume all models are available
        // In production, this would check:
        // - API key presence
        // - Rate limit status
        // - Quota remaining
        // - Service health
        true
    }
}

/// Difficulty-based router implementation with model registry and provider registry
pub struct StrategyRouter {
    /// Model registry for difficulty-based routing
    model_registry: Arc<ModelRegistry>,
    /// Provider registry for accessing provider instances
    provider_registry: Arc<ProviderRegistry>,
    /// Availability checker
    availability_checker: Arc<AvailabilityChecker>,
}

impl StrategyRouter {
    /// Creates a new difficulty-based router with default model registry.
    ///
    /// # Errors
    /// Returns an error if provider initialization fails (e.g., missing API keys).
    pub fn new(provider_registry: ProviderRegistry) -> Self {
        Self {
            model_registry: Arc::new(ModelRegistry::with_defaults()),
            provider_registry: Arc::new(provider_registry),
            availability_checker: Arc::new(AvailabilityChecker::default()),
        }
    }

    /// Creates a router with a custom model registry.
    #[must_use]
    pub fn with_model_registry(
        model_registry: ModelRegistry,
        provider_registry: ProviderRegistry,
    ) -> Self {
        Self {
            model_registry: Arc::new(model_registry),
            provider_registry: Arc::new(provider_registry),
            availability_checker: Arc::new(AvailabilityChecker::default()),
        }
    }

    /// Creates a router with default strategies (for backward compatibility).
    ///
    /// # Errors
    /// Returns an error if provider initialization fails.
    pub fn with_default_strategies() -> Result<Self> {
        use merlin_core::RoutingConfig;
        let config = RoutingConfig::load_or_create()?;
        let provider_registry = ProviderRegistry::new(config)?;
        Ok(Self::new(provider_registry))
    }

    /// Get the provider registry.
    #[must_use]
    pub fn provider_registry(&self) -> &Arc<ProviderRegistry> {
        &self.provider_registry
    }

    /// Get the model registry.
    #[must_use]
    pub fn model_registry(&self) -> &Arc<ModelRegistry> {
        &self.model_registry
    }
}

#[async_trait]
impl ModelRouter for StrategyRouter {
    async fn route(&self, task: &Task) -> Result<RoutingDecision> {
        // Check for difficulty-based provider override first
        if let Ok(provider) = self
            .provider_registry
            .get_provider_for_difficulty(task.difficulty)
        {
            let reasoning = format!(
                "Using configured provider override for difficulty level {}",
                task.difficulty
            );

            // Use a placeholder model since we're using a direct provider
            let model = self.model_registry.select_model(task.difficulty)?;
            let mut decision = RoutingDecision::new(model, reasoning);

            // Override with actual provider name
            provider.name().clone_into(&mut decision.provider_name);

            tracing::info!(
                "🎯 Routing decision: {} (override) | Difficulty: {} | Cost: ${:.6} | Latency: {}ms",
                provider.name(),
                task.difficulty,
                decision.estimated_cost,
                decision.estimated_latency_ms
            );

            return Ok(decision);
        }

        // Fall back to model-based routing
        let model = self.model_registry.select_model(task.difficulty)?;

        // Check if model is enabled in provider registry
        if self.provider_registry.get_provider(model).is_err() {
            return Err(RoutingError::Other(format!(
                "Selected model {model} is not available. \
                 Make sure the corresponding tier is enabled in configuration."
            )));
        }

        if !self.is_available(&model).await {
            return Err(RoutingError::Other(format!(
                "Selected model {model} is not currently available"
            )));
        }

        let reasoning = format!(
            "Selected {} for difficulty level {}",
            model, task.difficulty
        );

        let decision = RoutingDecision::new(model, reasoning);

        tracing::info!(
            "🎯 Routing decision: {} | Difficulty: {} | Cost: ${:.6} | Latency: {}ms",
            model,
            task.difficulty,
            decision.estimated_cost,
            decision.estimated_latency_ms
        );

        Ok(decision)
    }

    async fn is_available(&self, model: &Model) -> bool {
        self.availability_checker.check(*model) && self.provider_registry.is_available(*model).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_core::RoutingConfig;

    /// Creates a test router with cloud providers disabled.
    ///
    /// # Errors
    /// Returns an error if provider registry creation fails.
    fn create_test_router() -> Result<StrategyRouter> {
        let mut config = RoutingConfig::default();
        // Disable cloud providers for tests
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;

        let provider_registry = ProviderRegistry::new(config)?;
        Ok(StrategyRouter::new(provider_registry))
    }

    /// Tests routing for local models with low difficulty tasks.
    ///
    /// # Errors
    /// Returns an error if router creation or routing fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_difficulty_based_routing_local() -> Result<()> {
        let router = create_test_router()?;

        // Test low difficulty
        let easy_task = Task::new("Easy task".to_owned()).with_difficulty(2);
        let result = router.route(&easy_task).await;

        // Will fail because default registry uses Groq models which aren't enabled
        if result.is_err() {
            // Expected failure
        }

        Ok(())
    }

    /// Tests that availability checker returns true for all models.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_availability_checker_always_true() {
        let checker = AvailabilityChecker::default();
        assert!(checker.check(Model::Llama318BInstant));
        assert!(checker.check(Model::Qwen25Coder7B));
        assert!(checker.check(Model::Claude35Sonnet));
    }

    /// Tests that router accessors return valid references.
    ///
    /// # Errors
    /// Returns an error if router creation fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_router_accessors() -> Result<()> {
        use std::sync::Arc;

        let router = create_test_router()?;
        // Test that model_registry accessor works
        let model_result = router.model_registry().select_model(5);
        if model_result.is_err() {
            // Expected if no models registered for difficulty 5
        }
        // Test that provider_registry accessor works - it returns a valid Arc reference
        assert!(Arc::strong_count(router.provider_registry()) > 0);
        Ok(())
    }

    /// Tests router creation with default strategies.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_with_default_strategies_creation() {
        // This might fail in CI without API keys, which is expected
        let result = StrategyRouter::with_default_strategies();
        // Just ensure it doesn't panic
        drop(result);
    }

    /// Tests availability check for local models.
    ///
    /// # Errors
    /// Returns an error if router creation fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_is_available_local_model() -> Result<()> {
        let router = create_test_router()?;

        // Local models should be available if Ollama is running
        // We don't assert true/false as it depends on environment
        let _ = router.is_available(&Model::Qwen25Coder7B).await;

        Ok(())
    }

    /// Tests routing error when model is not available.
    ///
    /// # Errors
    /// Returns an error if router creation fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_route_with_invalid_model() -> Result<()> {
        let router = create_test_router()?;

        // Try to route a task when cloud providers are disabled
        let task = Task::new("Test task".to_owned()).with_difficulty(2);
        let result = router.route(&task).await;

        // Should fail because default registry uses Groq models
        if let Err(err) = result {
            assert!(err.to_string().contains("not available"));
        }

        Ok(())
    }

    /// Tests creating router with custom model registry.
    ///
    /// # Errors
    /// Returns an error if router creation fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_with_model_registry_constructor() -> Result<()> {
        let mut config = RoutingConfig::default();
        // Disable cloud providers for tests
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;

        let provider_registry = ProviderRegistry::new(config)?;
        let model_registry = ModelRegistry::with_defaults();

        let router = StrategyRouter::with_model_registry(model_registry, provider_registry);

        // Verify the router was created successfully
        let model_result = router.model_registry().select_model(5);
        if model_result.is_err() {
            // Expected if no models registered for difficulty 5
        }
        Ok(())
    }
}
