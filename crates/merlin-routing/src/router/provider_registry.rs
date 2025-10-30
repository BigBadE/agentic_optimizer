//! Provider registry for managing model provider instances.
//!
//! Separates provider instantiation from model selection, allowing
//! providers to be created once and reused throughout the application.

use super::models::{Model, TierCategory};
use merlin_core::{ModelProvider, Result, RoutingConfig, RoutingError};
use merlin_local::LocalModelProvider;
use merlin_providers::{GroqProvider, OpenRouterProvider};
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
    /// Configuration for API keys and settings
    config: Arc<RoutingConfig>,
}

impl ProviderRegistry {
    /// Create a new provider registry with the given configuration.
    ///
    /// # Errors
    /// Returns an error if API keys are missing for enabled tiers.
    pub fn new(config: RoutingConfig) -> Result<Self> {
        let config = Arc::new(config);
        let mut providers = HashMap::new();

        // Initialize all providers that are enabled
        if config.tiers.local_enabled {
            Self::register_local_providers(&mut providers);
        }

        if config.tiers.groq_enabled {
            Self::register_groq_providers(&mut providers, &config)?;
        }

        if config.tiers.premium_enabled {
            Self::register_premium_providers(&mut providers, &config)?;
        }

        Ok(Self { providers, config })
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
        let config = Arc::new(RoutingConfig::default());
        let mut providers = HashMap::new();

        // Register the mock provider for all models
        for model in Model::all() {
            providers.insert(model, Arc::clone(provider));
        }

        Ok(Self { providers, config })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_registry_with_defaults() {
        // This test will fail without API keys set, which is expected
        let config = RoutingConfig::default();

        // Try to create registry - will fail without API keys
        let result = ProviderRegistry::new(config);

        // Without API keys, groq and premium providers can't be created
        // This is expected behavior
        let registry = match result {
            Ok(registry) => registry,
            Err(_) => {
                // Expected when API keys aren't available
                return;
            }
        };

        // If we have API keys, verify registry was created
        assert!(!registry.registered_models().is_empty());
    }

    #[test]
    fn test_provider_registry_local_only() {
        let mut config = RoutingConfig::default();
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;

        let registry = match ProviderRegistry::new(config) {
            Ok(registry) => registry,
            Err(_) => {
                // Local providers not available
                return;
            }
        };

        let models = registry.registered_models();
        assert!(!models.is_empty());

        // All registered models should be local
        for model in models {
            assert_eq!(model.tier_category(), TierCategory::Local);
        }
    }

    #[test]
    fn test_get_provider_for_unregistered_model() {
        let mut config = RoutingConfig::default();
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;

        let registry = match ProviderRegistry::new(config) {
            Ok(registry) => registry,
            Err(_) => {
                // Local providers not available
                return;
            }
        };

        // Try to get a Groq model when Groq is disabled
        let result = registry.get_provider(Model::Llama318BInstant);
        result.unwrap_err();
    }
}
