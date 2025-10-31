//! Model definitions and registry.
//!
//! Centralizes all model definitions and provides type-safe model handling.
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

/// All supported models in the system.
///
/// This enum provides type-safe model handling and makes it easy to track
/// which models are supported and how they should be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Model {
    // Local models (Ollama)
    /// Qwen 2.5 Coder 7B model
    Qwen25Coder7B,
    /// Qwen 2.5 Coder 32B model
    Qwen25Coder32B,
    /// `DeepSeek` Coder V2 model
    DeepSeekCoderV2,

    // Groq models
    /// Llama 3.1 8B Instant (fastest Groq model)
    #[default]
    Llama318BInstant,
    /// Llama 3.1 70B Versatile (most capable free Groq model)
    Llama3170BVersatile,
    /// Llama 3.3 70B Versatile (newer Llama model)
    Llama3370BVersatile,
    /// Qwen 2.5 Coder 32B via Groq
    GroqQwen25Coder32B,

    // Premium models (OpenRouter)
    /// Claude 3.5 Haiku (fast, affordable premium)
    Claude35Haiku,
    /// Claude 3.5 Sonnet (most capable)
    Claude35Sonnet,
    /// `DeepSeek` V3 (high quality, low cost)
    DeepSeekV3,
}

impl Model {
    /// Get the provider name for this model.
    #[must_use]
    pub const fn provider_name(&self) -> &'static str {
        match self {
            Self::Qwen25Coder7B | Self::Qwen25Coder32B | Self::DeepSeekCoderV2 => "local",
            Self::Llama318BInstant
            | Self::Llama3170BVersatile
            | Self::Llama3370BVersatile
            | Self::GroqQwen25Coder32B => "groq",
            Self::Claude35Haiku | Self::Claude35Sonnet | Self::DeepSeekV3 => "openrouter",
        }
    }

    /// Get the model identifier string used by the provider.
    #[must_use]
    pub const fn model_id(&self) -> &'static str {
        match self {
            Self::Qwen25Coder7B => "qwen2.5-coder:7b",
            Self::Qwen25Coder32B => "qwen2.5-coder:32b",
            Self::DeepSeekCoderV2 => "deepseek-coder-v2",
            Self::Llama318BInstant => "llama-3.1-8b-instant",
            Self::Llama3170BVersatile => "llama-3.1-70b-versatile",
            Self::Llama3370BVersatile => "llama-3.3-70b-versatile",
            Self::GroqQwen25Coder32B => "qwen2.5-32b-coder-preview",
            Self::Claude35Haiku => "anthropic/claude-3-5-haiku-20241022",
            Self::Claude35Sonnet => "anthropic/claude-3-5-sonnet-20241022",
            Self::DeepSeekV3 => "deepseek/deepseek-chat",
        }
    }

    /// Get the tier category for this model.
    #[must_use]
    pub const fn tier_category(&self) -> TierCategory {
        match self {
            Self::Qwen25Coder7B | Self::Qwen25Coder32B | Self::DeepSeekCoderV2 => {
                TierCategory::Local
            }
            Self::Llama318BInstant
            | Self::Llama3170BVersatile
            | Self::Llama3370BVersatile
            | Self::GroqQwen25Coder32B => TierCategory::Groq,
            Self::Claude35Haiku | Self::Claude35Sonnet | Self::DeepSeekV3 => TierCategory::Premium,
        }
    }

    /// Estimate cost per 1M tokens (input + output combined, rough average).
    #[must_use]
    pub const fn cost_per_million_tokens(&self) -> f64 {
        match self {
            // Local and Groq models are free
            Self::Qwen25Coder7B
            | Self::Qwen25Coder32B
            | Self::DeepSeekCoderV2
            | Self::Llama318BInstant
            | Self::Llama3170BVersatile
            | Self::Llama3370BVersatile
            | Self::GroqQwen25Coder32B => 0.0,
            // Premium models (rough averages)
            Self::Claude35Haiku => 1.0,   // ~$0.25 input + $1.25 output
            Self::Claude35Sonnet => 15.0, // ~$3 input + $15 output
            Self::DeepSeekV3 => 0.2,      // Very cheap premium model
        }
    }

    /// Get relative quality score (1-10).
    #[must_use]
    pub const fn quality_score(&self) -> u8 {
        match self {
            Self::Qwen25Coder7B | Self::Llama318BInstant => 4,
            Self::DeepSeekCoderV2 => 5,
            Self::Qwen25Coder32B | Self::Llama3170BVersatile | Self::GroqQwen25Coder32B => 6,
            Self::Llama3370BVersatile => 7,
            Self::Claude35Haiku => 8,
            Self::Claude35Sonnet => 10,
            Self::DeepSeekV3 => 9,
        }
    }

    /// Get all models in a tier category.
    #[must_use]
    pub fn models_in_category(category: TierCategory) -> Vec<Self> {
        Self::all()
            .into_iter()
            .filter(|model| model.tier_category() == category)
            .collect()
    }

    /// Get all supported models.
    #[must_use]
    pub const fn all() -> [Self; 10] {
        [
            Self::Qwen25Coder7B,
            Self::Qwen25Coder32B,
            Self::DeepSeekCoderV2,
            Self::Llama318BInstant,
            Self::Llama3170BVersatile,
            Self::Llama3370BVersatile,
            Self::GroqQwen25Coder32B,
            Self::Claude35Haiku,
            Self::Claude35Sonnet,
            Self::DeepSeekV3,
        ]
    }
}

impl Display for Model {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Qwen25Coder7B => write!(f, "Qwen 2.5 Coder 7B"),
            Self::Qwen25Coder32B => write!(f, "Qwen 2.5 Coder 32B"),
            Self::DeepSeekCoderV2 => write!(f, "DeepSeek Coder V2"),
            Self::Llama318BInstant => write!(f, "Llama 3.1 8B Instant"),
            Self::Llama3170BVersatile => write!(f, "Llama 3.1 70B Versatile"),
            Self::Llama3370BVersatile => write!(f, "Llama 3.3 70B Versatile"),
            Self::GroqQwen25Coder32B => write!(f, "Qwen 2.5 32B Coder (Groq)"),
            Self::Claude35Haiku => write!(f, "Claude 3.5 Haiku"),
            Self::Claude35Sonnet => write!(f, "Claude 3.5 Sonnet"),
            Self::DeepSeekV3 => write!(f, "DeepSeek V3"),
        }
    }
}

/// Tier category for grouping models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TierCategory {
    /// Local models running on user's machine
    Local,
    /// Fast cloud models via Groq
    Groq,
    /// Premium cloud models
    Premium,
}

impl Display for TierCategory {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Local => write!(f, "Local"),
            Self::Groq => write!(f, "Groq"),
            Self::Premium => write!(f, "Premium"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that model properties are correctly defined.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_model_properties() {
        assert_eq!(Model::Llama318BInstant.provider_name(), "groq");
        assert_eq!(Model::Llama318BInstant.model_id(), "llama-3.1-8b-instant");
        assert_eq!(Model::Llama318BInstant.tier_category(), TierCategory::Groq);
    }

    /// Tests that all models have valid properties.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_all_models_have_valid_properties() {
        for model in Model::all() {
            assert!(!model.provider_name().is_empty());
            assert!(!model.model_id().is_empty());
            assert!(model.quality_score() >= 1 && model.quality_score() <= 10);
            assert!(model.cost_per_million_tokens() >= 0.0);
        }
    }

    /// Tests that models are correctly grouped by tier category.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_models_in_category() {
        let groq_models = Model::models_in_category(TierCategory::Groq);
        assert!(groq_models.contains(&Model::Llama318BInstant));
        assert!(groq_models.contains(&Model::Llama3170BVersatile));

        let premium_models = Model::models_in_category(TierCategory::Premium);
        assert!(premium_models.contains(&Model::Claude35Sonnet));
    }
}
