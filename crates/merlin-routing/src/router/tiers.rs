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

    fn create_test_router() -> Result<StrategyRouter> {
        let mut config = RoutingConfig::default();
        // Disable cloud providers for tests
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;

        let provider_registry = ProviderRegistry::new(config)?;
        Ok(StrategyRouter::new(provider_registry))
    }

    #[tokio::test]
    async fn test_difficulty_based_routing_local() -> Result<()> {
        let router = create_test_router()?;

        // Test low difficulty
        let easy_task = Task::new("Easy task".to_owned()).with_difficulty(2);
        let result = router.route(&easy_task).await;

        // Will fail because default registry uses Groq models which aren't enabled
        result.unwrap_err();

        Ok(())
    }

    #[tokio::test]
    async fn test_custom_model_registry() -> Result<()> {
        use merlin_core::ModelProvider;
        use merlin_providers::MockProvider;
        use std::sync::Arc;

        // Create mock provider for testing
        let mock_provider: Arc<dyn ModelProvider> =
            Arc::new(MockProvider::new("test").with_default_response("Mock response for testing"));

        let provider_registry = ProviderRegistry::with_mock_provider(&mock_provider)?;
        let mut model_registry = ModelRegistry::new();

        // Register local models only
        model_registry.register_range(1..=10, Model::Qwen25Coder7B);

        let router = StrategyRouter::with_model_registry(model_registry, provider_registry);

        let task = Task::new("Test task".to_owned()).with_difficulty(5);
        let decision = router.route(&task).await?;

        assert_eq!(decision.model, Model::Qwen25Coder7B);

        Ok(())
    }

    #[test]
    fn test_availability_checker_always_true() {
        let checker = AvailabilityChecker::default();
        assert!(checker.check(Model::Llama318BInstant));
        assert!(checker.check(Model::Qwen25Coder7B));
        assert!(checker.check(Model::Claude35Sonnet));
    }

    #[test]
    fn test_router_accessors() -> Result<()> {
        use std::sync::Arc;

        let router = create_test_router()?;
        // Test that model_registry accessor works
        router.model_registry().select_model(5).unwrap();
        // Test that provider_registry accessor works - it returns a valid Arc reference
        assert!(Arc::strong_count(router.provider_registry()) > 0);
        Ok(())
    }

    #[test]
    fn test_with_default_strategies_creation() {
        // This might fail in CI without API keys, which is expected
        let result = StrategyRouter::with_default_strategies();
        // Just ensure it doesn't panic
        drop(result);
    }

    #[tokio::test]
    async fn test_is_available_local_model() -> Result<()> {
        let router = create_test_router()?;

        // Local models should be available if Ollama is running
        // We don't assert true/false as it depends on environment
        let _ = router.is_available(&Model::Qwen25Coder7B).await;

        Ok(())
    }

    #[tokio::test]
    async fn test_route_with_invalid_model() -> Result<()> {
        let router = create_test_router()?;

        // Try to route a task when cloud providers are disabled
        let task = Task::new("Test task".to_owned()).with_difficulty(2);
        let result = router.route(&task).await;

        // Should fail because default registry uses Groq models
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not available"));

        Ok(())
    }

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
        router.model_registry().select_model(5).unwrap();
        Ok(())
    }
}
