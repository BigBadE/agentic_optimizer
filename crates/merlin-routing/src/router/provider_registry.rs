//! Provider registry for managing model provider instances.
//!
//! Separates provider instantiation from model selection, allowing
//! providers to be created once and reused throughout the application.

use super::models::{Model, TierCategory};
use merlin_core::{ModelProvider, ProviderType, Result, RoutingConfig, RoutingError};
use merlin_local::LocalModelProvider;
use merlin_providers::{ClaudeCodeProvider, GroqProvider, OpenRouterProvider};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

/// Registry that maps models to their provider instances.
///
/// Providers are instantiated once during initialization and reused
/// for all requests, avoiding the overhead of repeated provider creation.
#[derive(Clone)]
pub struct ProviderRegistry {
    /// Map from model to provider instance
    providers: HashMap<Model, Arc<dyn ModelProvider>>,
    /// Difficulty-based provider overrides (1-10 -> provider)
    difficulty_overrides: HashMap<u8, Arc<dyn ModelProvider>>,
    /// Configuration for API keys and settings
    config: RoutingConfig,
}

impl ProviderRegistry {
    /// Create a new provider registry with the given configuration.
    ///
    /// # Errors
    /// Returns an error if API keys are missing for enabled tiers.
    pub fn new(config: RoutingConfig) -> Result<Self> {
        let mut providers = HashMap::new();
        let mut difficulty_overrides = HashMap::new();

        // Setup difficulty-based overrides first
        Self::register_difficulty_overrides(&mut difficulty_overrides, &config)?;

        // Initialize tier-based providers based on enabled flags
        // These are used for model-based routing, independent of difficulty overrides
        if config.tiers.local_enabled {
            Self::register_local_providers(&mut providers);
        }

        if config.tiers.groq_enabled {
            Self::register_groq_providers(&mut providers, &config)?;
        }

        if config.tiers.premium_enabled {
            Self::register_premium_providers(&mut providers, &config)?;
        }

        Ok(Self {
            providers,
            difficulty_overrides,
            config,
        })
    }

    /// Register all local model providers.
    fn register_local_providers(providers: &mut HashMap<Model, Arc<dyn ModelProvider>>) {
        for model in Model::all() {
            if model.tier_category() == TierCategory::Local {
                let provider = LocalModelProvider::new(model.model_id().to_owned());
                providers.insert(model, Arc::new(provider));
            }
        }
    }

    /// Register all Groq model providers.
    ///
    /// # Errors
    /// Returns an error if Groq API key is missing or provider creation fails
    fn register_groq_providers(
        providers: &mut HashMap<Model, Arc<dyn ModelProvider>>,
        config: &RoutingConfig,
    ) -> Result<()> {
        // Get Groq API key
        let api_key = config
            .get_api_key("groq")
            .or_else(|| env::var("GROQ_API_KEY").ok())
            .ok_or_else(|| {
                RoutingError::Other("GROQ_API_KEY not found in config or environment".to_owned())
            })?;

        // Create provider for each Groq model
        for model in Model::all() {
            if model.tier_category() == TierCategory::Groq {
                let provider = GroqProvider::with_api_key_direct(api_key.clone())
                    .map_err(|error| RoutingError::Other(error.to_string()))?
                    .with_model(model.model_id().to_owned());
                providers.insert(model, Arc::new(provider));
            }
        }

        Ok(())
    }

    /// Register all premium model providers.
    ///
    /// # Errors
    /// Returns an error if `OpenRouter` API key is missing or provider creation fails
    fn register_premium_providers(
        providers: &mut HashMap<Model, Arc<dyn ModelProvider>>,
        config: &RoutingConfig,
    ) -> Result<()> {
        // Get OpenRouter API key
        let api_key = config
            .get_api_key("openrouter")
            .or_else(|| env::var("OPENROUTER_API_KEY").ok())
            .ok_or_else(|| {
                RoutingError::Other(
                    "OPENROUTER_API_KEY not found in config or environment".to_owned(),
                )
            })?;

        // Create provider for each premium model
        for model in Model::all() {
            if model.tier_category() == TierCategory::Premium {
                let provider = OpenRouterProvider::new(api_key.clone())?
                    .with_model(model.model_id().to_owned());
                providers.insert(model, Arc::new(provider));
            }
        }

        Ok(())
    }

    /// Register difficulty-based provider overrides from config.
    ///
    /// # Errors
    /// Returns an error if provider creation fails
    fn register_difficulty_overrides(
        overrides: &mut HashMap<u8, Arc<dyn ModelProvider>>,
        config: &RoutingConfig,
    ) -> Result<()> {
        // Register low difficulty provider (1-3)
        if let Some(provider_type) = &config.tiers.provider_low {
            let provider = Self::create_provider_for_type(provider_type, config)?;
            for difficulty in 1..=3 {
                overrides.insert(difficulty, Arc::clone(&provider));
            }
        }

        // Register mid difficulty provider (4-6)
        if let Some(provider_type) = &config.tiers.provider_mid {
            let provider = Self::create_provider_for_type(provider_type, config)?;
            for difficulty in 4..=6 {
                overrides.insert(difficulty, Arc::clone(&provider));
            }
        }

        // Register high difficulty provider (7-10)
        if let Some(provider_type) = &config.tiers.provider_high {
            let provider = Self::create_provider_for_type(provider_type, config)?;
            for difficulty in 7..=10 {
                overrides.insert(difficulty, Arc::clone(&provider));
            }
        }

        Ok(())
    }

    /// Create a provider instance for the given provider type.
    ///
    /// # Errors
    /// Returns an error if provider creation fails
    fn create_provider_for_type(
        provider_type: &ProviderType,
        config: &RoutingConfig,
    ) -> Result<Arc<dyn ModelProvider>> {
        match provider_type {
            ProviderType::Local => {
                let model = config.tiers.local_model.clone();
                Ok(Arc::new(LocalModelProvider::new(model)))
            }
            ProviderType::Groq => {
                let api_key = config
                    .get_api_key("groq")
                    .or_else(|| env::var("GROQ_API_KEY").ok())
                    .ok_or_else(|| {
                        RoutingError::Other(
                            "GROQ_API_KEY not found in config or environment".to_owned(),
                        )
                    })?;
                let model = config.tiers.groq_model.clone();
                let provider = GroqProvider::with_api_key_direct(api_key)
                    .map_err(|error| RoutingError::Other(error.to_string()))?
                    .with_model(model);
                Ok(Arc::new(provider))
            }
            ProviderType::OpenRouter => {
                let api_key = config
                    .get_api_key("openrouter")
                    .or_else(|| env::var("OPENROUTER_API_KEY").ok())
                    .ok_or_else(|| {
                        RoutingError::Other(
                            "OPENROUTER_API_KEY not found in config or environment".to_owned(),
                        )
                    })?;
                let provider = OpenRouterProvider::new(api_key)?;
                Ok(Arc::new(provider))
            }
            ProviderType::ClaudeCode => {
                let provider = ClaudeCodeProvider::new()
                    .map_err(|error| RoutingError::Other(error.to_string()))?;
                Ok(Arc::new(provider))
            }
        }
    }

    /// Get the provider instance for a given model.
    ///
    /// # Errors
    /// Returns an error if no provider is registered for the model.
    pub fn get_provider(&self, model: Model) -> Result<Arc<dyn ModelProvider>> {
        self.providers.get(&model).cloned().ok_or_else(|| {
            RoutingError::Other(format!(
                "No provider registered for model: {model}. \
                     Make sure the corresponding tier is enabled in configuration."
            ))
        })
    }

    /// Get the provider for a specific difficulty level, using overrides if configured.
    ///
    /// # Errors
    /// Returns an error if no provider is available for the difficulty.
    pub fn get_provider_for_difficulty(&self, difficulty: u8) -> Result<Arc<dyn ModelProvider>> {
        // Check for difficulty-based override first
        if let Some(provider) = self.difficulty_overrides.get(&difficulty) {
            return Ok(Arc::clone(provider));
        }

        // Fall back to model-based selection (existing behavior)
        Err(RoutingError::Other(format!(
            "No provider override for difficulty {difficulty}. \
             Use model-based routing instead."
        )))
    }

    /// Get the provider for a task, checking difficulty overrides first, then model.
    ///
    /// This is the unified method that should be used by executors.
    ///
    /// # Errors
    /// Returns an error if no provider is available.
    pub fn get_provider_for_task(
        &self,
        difficulty: u8,
        model: Model,
    ) -> Result<Arc<dyn ModelProvider>> {
        // Check for difficulty-based override first
        if let Ok(provider) = self.get_provider_for_difficulty(difficulty) {
            return Ok(provider);
        }

        // Fall back to model-based provider
        self.get_provider(model)
    }

    /// Check if a provider is available for the given model.
    pub async fn is_available(&self, model: Model) -> bool {
        if let Ok(provider) = self.get_provider(model) {
            provider.is_available().await
        } else {
            false
        }
    }

    /// Get all registered models.
    #[must_use]
    pub fn registered_models(&self) -> Vec<Model> {
        self.providers.keys().copied().collect()
    }

    /// Get the configuration.
    #[must_use]
    pub fn config(&self) -> &RoutingConfig {
        &self.config
    }

    /// Register a custom provider for a specific model (useful for testing).
    ///
    /// This allows injecting mock providers or overriding default providers.
    pub fn register_provider(&mut self, model: Model, provider: Arc<dyn ModelProvider>) {
        self.providers.insert(model, provider);
    }

    /// Create a registry with a single mock provider for all models (testing).
    ///
    /// # Errors
    /// Returns an error if configuration is invalid.
    pub fn with_mock_provider(provider: &Arc<dyn ModelProvider>) -> Result<Self> {
        let config = RoutingConfig::default();
        let mut providers = HashMap::new();

        // Register the mock provider for all models
        for model in Model::all() {
            providers.insert(model, Arc::clone(provider));
        }

        Ok(Self {
            providers,
            difficulty_overrides: HashMap::new(),
            config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests provider registry creation with default configuration.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_provider_registry_with_defaults() {
        // This test will fail without API keys set, which is expected
        let config = RoutingConfig::default();

        // Try to create registry - will fail without API keys
        // Without API keys, groq and premium providers can't be created
        // This is expected behavior
        let Ok(registry) = ProviderRegistry::new(config) else {
            // Expected when API keys aren't available
            return;
        };

        // If we have API keys, verify registry was created
        assert!(!registry.registered_models().is_empty());
    }

    /// Tests provider registry with only local providers enabled.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_provider_registry_local_only() {
        let mut config = RoutingConfig::default();
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;

        let Ok(registry) = ProviderRegistry::new(config) else {
            // Local providers not available
            return;
        };

        let models = registry.registered_models();
        assert!(!models.is_empty());

        // All registered models should be local
        for model in models {
            assert_eq!(model.tier_category(), TierCategory::Local);
        }
    }

    /// Tests error when requesting provider for disabled model.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_get_provider_for_unregistered_model() {
        let mut config = RoutingConfig::default();
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;

        let Ok(registry) = ProviderRegistry::new(config) else {
            // Local providers not available
            return;
        };

        // Try to get a Groq model when Groq is disabled
        let result = registry.get_provider(Model::Llama318BInstant);
        assert!(
            result.is_err(),
            "Expected error when getting disabled provider"
        );
    }
}
