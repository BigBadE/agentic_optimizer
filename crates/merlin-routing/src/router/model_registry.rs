//! Model registry for difficulty-based routing.
//!
//! Maps difficulty levels (1-10) to appropriate models.
use super::models::Model;
use crate::{Result, RoutingError};
use std::collections::HashMap;
use std::ops::RangeInclusive;

/// Difficulty level from 1 (easiest) to 10 (hardest)
pub type DifficultyLevel = u8;

/// Registry for models organized by difficulty level.
///
/// Allows registering models for specific difficulty ranges and
/// selecting the appropriate model based on task difficulty.
#[derive(Clone, Debug)]
pub struct ModelRegistry {
    /// Models registered for each difficulty level (no locking needed - immutable after init)
    models: HashMap<DifficultyLevel, Model>,
}

impl ModelRegistry {
    /// Creates a new empty model registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Creates a model registry with default mappings based on cost optimization.
    ///
    /// Default mappings:
    /// - Difficulty 1-2: Llama 3.1 8B Instant (Groq, fast and cheap)
    /// - Difficulty 3-4: Qwen 2.5 32B Coder (Groq, balanced)
    /// - Difficulty 5-6: Llama 3.1 70B Versatile (Groq, more capable)
    /// - Difficulty 7-8: Claude 3.5 Haiku (Premium, high quality)
    /// - Difficulty 9-10: Claude 3.5 Sonnet (Premium, best quality)
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Difficulty 1-2: Simple tasks - use smallest Groq model
        registry.register_range(1..=2, Model::Llama318BInstant);

        // Difficulty 3-4: Medium tasks - use Groq 32B coder
        registry.register_range(3..=4, Model::GroqQwen25Coder32B);

        // Difficulty 5-6: Moderately complex - use Groq 70B
        registry.register_range(5..=6, Model::Llama3170BVersatile);

        // Difficulty 7-8: Complex tasks - use premium Haiku
        registry.register_range(7..=8, Model::Claude35Haiku);

        // Difficulty 9-10: Very complex tasks - use premium Sonnet
        registry.register_range(9..=10, Model::Claude35Sonnet);

        registry
    }

    /// Registers a model for a range of difficulty levels.
    ///
    /// # Panics
    /// Panics if any difficulty level is not in range 1-10.
    pub fn register_range(&mut self, range: RangeInclusive<DifficultyLevel>, model: Model) {
        for difficulty in range {
            assert!(
                (1..=10).contains(&difficulty),
                "Difficulty level must be between 1 and 10, got {difficulty}"
            );
            self.models.insert(difficulty, model);
        }
    }

    /// Registers a model for a specific difficulty level.
    ///
    /// # Errors
    /// Returns error if difficulty level is not in range 1-10.
    pub fn register(&mut self, difficulty: DifficultyLevel, model: Model) -> Result<()> {
        if !(1..=10).contains(&difficulty) {
            return Err(RoutingError::Other(format!(
                "Difficulty level must be between 1 and 10, got {difficulty}"
            )));
        }

        self.models.insert(difficulty, model);
        Ok(())
    }

    /// Selects the appropriate model for a given difficulty level.
    ///
    /// If no exact match is found, returns the nearest higher difficulty model.
    /// Falls back to the highest registered model if difficulty exceeds all registrations.
    ///
    /// # Errors
    /// Returns error if:
    /// - Difficulty level is not in range 1-10
    /// - No models are registered
    pub fn select_model(&self, difficulty: DifficultyLevel) -> Result<Model> {
        if !(1..=10).contains(&difficulty) {
            return Err(RoutingError::Other(format!(
                "Difficulty level must be between 1 and 10, got {difficulty}"
            )));
        }

        if self.models.is_empty() {
            return Err(RoutingError::Other(
                "No models registered in ModelRegistry".to_owned(),
            ));
        }

        // Try exact match first
        if let Some(model) = self.models.get(&difficulty) {
            return Ok(*model);
        }

        // Find nearest higher difficulty
        let mut candidates: Vec<_> = self
            .models
            .iter()
            .filter(|(level, _)| **level >= difficulty)
            .collect();

        if !candidates.is_empty() {
            candidates.sort_by_key(|(level, _)| **level);
            return Ok(*candidates[0].1);
        }

        // Fall back to highest registered model
        let max_level = self.models.keys().max().copied().ok_or_else(|| {
            RoutingError::Other("No models registered in ModelRegistry".to_owned())
        })?;

        self.models
            .get(&max_level)
            .copied()
            .ok_or_else(|| RoutingError::Other("Failed to retrieve model".to_owned()))
    }

    /// Lists all registered difficulty levels.
    #[must_use]
    pub fn registered_levels(&self) -> Vec<DifficultyLevel> {
        let mut levels: Vec<_> = self.models.keys().copied().collect();
        levels.sort_unstable();
        levels
    }

    /// Clears all registered models.
    pub fn clear(&mut self) {
        self.models.clear();
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_registry_defaults() {
        let registry = ModelRegistry::with_defaults();
        let levels = registry.registered_levels();

        assert_eq!(levels.len(), 10);
        assert_eq!(levels[0], 1);
        assert_eq!(levels[9], 10);
    }

    #[test]
    fn test_exact_match() -> Result<()> {
        let registry = ModelRegistry::with_defaults();
        let model = registry.select_model(5)?;
        assert_eq!(model, Model::Llama3170BVersatile);
        Ok(())
    }

    #[test]
    fn test_nearest_higher() -> Result<()> {
        let mut registry = ModelRegistry::new();
        registry.register(2, Model::Llama318BInstant)?;
        registry.register(5, Model::Llama3170BVersatile)?;
        registry.register(8, Model::Claude35Haiku)?;

        // Difficulty 3 should select Llama3170BVersatile (nearest higher)
        let model = registry.select_model(3)?;
        assert_eq!(model, Model::Llama3170BVersatile);
        Ok(())
    }

    #[test]
    fn test_fallback_to_highest() -> Result<()> {
        let mut registry = ModelRegistry::new();
        registry.register(5, Model::Llama3170BVersatile)?;

        // Difficulty 10 should fall back to Llama3170BVersatile (highest registered)
        let model = registry.select_model(10)?;
        assert_eq!(model, Model::Llama3170BVersatile);
        Ok(())
    }

    #[test]
    fn test_invalid_difficulty() {
        let registry = ModelRegistry::with_defaults();

        registry.select_model(0).unwrap_err();
        registry.select_model(11).unwrap_err();
    }

    #[test]
    fn test_empty_registry() {
        let registry = ModelRegistry::new();
        let result = registry.select_model(5);
        result.unwrap_err();
    }

    #[test]
    fn test_register_and_retrieve() -> Result<()> {
        let mut registry = ModelRegistry::new();
        registry.register(7, Model::Claude35Sonnet)?;

        let model = registry.select_model(7)?;
        assert_eq!(model, Model::Claude35Sonnet);
        Ok(())
    }

    #[test]
    fn test_clear() {
        let mut registry = ModelRegistry::with_defaults();
        assert_eq!(registry.registered_levels().len(), 10);

        registry.clear();
        assert_eq!(registry.registered_levels().len(), 0);
    }

    #[test]
    fn test_register_range() -> Result<()> {
        let mut registry = ModelRegistry::new();
        registry.register_range(1..=5, Model::Llama318BInstant);
        registry.register_range(6..=10, Model::Claude35Sonnet);

        assert_eq!(registry.select_model(1)?, Model::Llama318BInstant);
        assert_eq!(registry.select_model(3)?, Model::Llama318BInstant);
        assert_eq!(registry.select_model(5)?, Model::Llama318BInstant);
        assert_eq!(registry.select_model(6)?, Model::Claude35Sonnet);
        assert_eq!(registry.select_model(10)?, Model::Claude35Sonnet);
        Ok(())
    }

    #[test]
    #[should_panic(expected = "Difficulty level must be between 1 and 10, got 0")]
    fn test_register_range_panics_on_invalid() {
        let mut registry = ModelRegistry::new();
        registry.register_range(0..=5, Model::Llama318BInstant);
    }
}
