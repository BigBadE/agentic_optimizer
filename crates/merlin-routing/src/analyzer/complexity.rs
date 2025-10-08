use super::intent::{Action, Intent};
use crate::{Complexity, ContextRequirements};
use std::path::PathBuf;

/// Estimates task complexity based on multiple factors
#[derive(Default)]
pub struct ComplexityEstimator;

impl ComplexityEstimator {
    /// Estimate task complexity
    pub fn estimate(&self, intent: &Intent, request: &str) -> Complexity {
        let mut score = 0;

        score += Self::score_action(&intent.action);
        score += Self::score_scope(&intent.scope);
        score += Self::score_request_length(request);
        score += Self::score_keywords(request);

        if let Some(hint) = intent.complexity_hint {
            score = usize::midpoint(score, Self::complexity_to_score(hint));
        }

        Self::score_to_complexity(score)
    }

    /// Estimate context requirements for a task
    pub fn estimate_context_needs(&self, intent: &Intent, request: &str) -> ContextRequirements {
        let estimated_tokens = Self::estimate_token_count(request);
        let required_files = Self::extract_file_references(request);
        let requires_full_context = Self::needs_full_context(intent, request);

        ContextRequirements::default()
            .with_estimated_tokens(estimated_tokens)
            .with_files(required_files)
            .with_full_context(requires_full_context)
    }

    fn score_action(action: &Action) -> usize {
        match action {
            Action::Create | Action::Delete | Action::Document => 1,
            Action::Modify | Action::Fix | Action::Test => 2,
            Action::Analyze | Action::Optimize => 3,
            Action::Refactor => 4,
        }
    }

    fn score_scope(scope: &super::intent::Scope) -> usize {
        use super::intent::Scope;

        match scope {
            Scope::Function(_) => 1,
            Scope::File(_) => 2,
            Scope::Module(_) => 3,
            Scope::Multiple(files) => 2 + files.len().min(3),
            Scope::Project => 4,
        }
    }

    fn score_request_length(request: &str) -> usize {
        let word_count = request.split_whitespace().count();

        if word_count < 10 {
            0
        } else if word_count < 30 {
            1
        } else if word_count < 60 {
            2
        } else {
            3
        }
    }

    fn score_keywords(request: &str) -> usize {
        let request_lower = request.to_lowercase();
        let mut score = 0;

        let complex_keywords = [
            "architecture",
            "design",
            "refactor",
            "optimize",
            "performance",
            "concurrent",
            "async",
            "distributed",
            "algorithm",
            "complex",
        ];

        for keyword in &complex_keywords {
            if request_lower.contains(keyword) {
                score += 1;
            }
        }

        score.min(3)
    }

    fn complexity_to_score(complexity: Complexity) -> usize {
        match complexity {
            Complexity::Trivial => 0,
            Complexity::Simple => 2,
            Complexity::Medium => 5,
            Complexity::Complex => 8,
        }
    }

    fn score_to_complexity(score: usize) -> Complexity {
        match score {
            0..=2 => Complexity::Trivial,
            3..=5 => Complexity::Simple,
            6..=8 => Complexity::Medium,
            _ => Complexity::Complex,
        }
    }

    fn estimate_token_count(request: &str) -> usize {
        let base_tokens = request.len() / 4;

        let file_count = request.matches(".rs").count() + request.matches(".toml").count();
        let file_tokens = file_count * 500;

        let context_multiplier = if request.contains("entire") || request.contains("all") {
            3
        } else if request.contains("multiple") {
            2
        } else {
            1
        };

        (base_tokens + file_tokens) * context_multiplier
    }

    fn extract_file_references(request: &str) -> Vec<PathBuf> {
        request
            .split_whitespace()
            .filter(|word| word.contains(".rs") || word.contains(".toml"))
            .map(|word| {
                PathBuf::from(word.trim_matches(|letter: char| {
                    !letter.is_alphanumeric() && letter != '.' && letter != '/' && letter != '_'
                }))
            })
            .collect()
    }

    fn needs_full_context(intent: &Intent, request: &str) -> bool {
        use super::intent::Scope;

        matches!(intent.scope, Scope::Project | Scope::Multiple(_))
            || request.contains("entire")
            || request.contains("all files")
            || request.contains("codebase")
    }
}

#[cfg(test)]
mod tests {
    use super::super::intent::IntentExtractor;
    use super::*;

    #[test]
    fn test_trivial_complexity() {
        let estimator = ComplexityEstimator;
        let extractor = IntentExtractor;

        let intent = extractor.extract("Add a comment");
        let complexity = estimator.estimate(&intent, "Add a comment");

        assert!(matches!(
            complexity,
            Complexity::Trivial | Complexity::Simple
        ));
    }

    #[test]
    fn test_complex_refactor() {
        let estimator = ComplexityEstimator;
        let extractor = IntentExtractor;

        let intent = extractor.extract("Refactor the entire architecture to use async patterns");
        let complexity = estimator.estimate(
            &intent,
            "Refactor the entire architecture to use async patterns",
        );

        assert_eq!(complexity, Complexity::Complex);
    }

    #[test]
    fn test_context_estimation() {
        let estimator = ComplexityEstimator;
        let extractor = IntentExtractor;

        let intent = extractor.extract("Modify test.rs and main.rs");
        let context = estimator.estimate_context_needs(&intent, "Modify test.rs and main.rs");

        assert_eq!(context.required_files.len(), 2);
        assert!(context.estimated_tokens > 0);
    }

    #[test]
    fn test_full_context_detection() {
        let estimator = ComplexityEstimator;
        let extractor = IntentExtractor;

        let intent = extractor.extract("Analyze the entire codebase");
        let context = estimator.estimate_context_needs(&intent, "Analyze the entire codebase");

        assert!(context.requires_full_context);
    }
}
