//! Mock provider for testing with scoped response strategies.

mod provider;
mod router;
mod strategy;

pub use provider::MockProvider;
pub use router::MockRouter;
pub use strategy::{MatchAgainst, MatchType, ResponseStrategy, TriggerConfig};

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_core::{Context, ModelProvider, Query, Result};

    /// Test basic pattern matching
    ///
    /// # Errors
    /// Returns error if test fails
    ///
    /// # Panics
    /// Panics if assertions fail
    #[tokio::test]
    async fn test_exact_match() -> Result<()> {
        let mut provider = MockProvider::new("test");

        let trigger = TriggerConfig::new(
            "hello world".to_owned(),
            MatchType::Exact,
            MatchAgainst::Query,
        )?;

        let strategy = ResponseStrategy::Once {
            trigger,
            typescript: "return 'matched';".to_owned(),
        };

        provider.push_scope(None, vec![strategy])?;

        let query = Query::new("hello world");
        let context = Context::new("");

        let response = provider.generate(&query, &context).await?;
        assert!(response.text.contains("matched"));

        Ok(())
    }

    /// Test sequence strategy
    ///
    /// # Errors
    /// Returns error if test fails
    ///
    /// # Panics
    /// Panics if assertions fail
    #[tokio::test]
    async fn test_sequence_strategy() -> Result<()> {
        let mut provider = MockProvider::new("test");

        let trigger =
            TriggerConfig::new("retry".to_owned(), MatchType::Contains, MatchAgainst::Query)?;

        let strategy = ResponseStrategy::Sequence {
            trigger,
            responses: vec![
                "return 'first';".to_owned(),
                "return 'second';".to_owned(),
                "return 'third';".to_owned(),
            ],
        };

        provider.push_scope(None, vec![strategy])?;

        let query = Query::new("retry command");
        let context = Context::new("");

        // First call
        let response1 = provider.generate(&query, &context).await?;
        assert!(response1.text.contains("first"));

        // Second call
        let response2 = provider.generate(&query, &context).await?;
        assert!(response2.text.contains("second"));

        // Third call
        let response3 = provider.generate(&query, &context).await?;
        assert!(response3.text.contains("third"));

        // Fourth call should fail (exhausted)
        let result4 = provider.generate(&query, &context).await;
        assert!(result4.is_err());

        Ok(())
    }

    /// Test repeating strategy
    ///
    /// # Errors
    /// Returns error if test fails
    ///
    /// # Panics
    /// Panics if assertions fail
    #[tokio::test]
    async fn test_repeating_strategy() -> Result<()> {
        let mut provider = MockProvider::new("test");

        let trigger = TriggerConfig::new("ping".to_owned(), MatchType::Exact, MatchAgainst::Query)?;

        let strategy = ResponseStrategy::Repeating {
            trigger,
            typescript: "return 'pong';".to_owned(),
        };

        provider.push_scope(None, vec![strategy])?;

        let query = Query::new("ping");
        let context = Context::new("");

        // Should work infinitely
        for _ in 0..10 {
            let response = provider.generate(&query, &context).await?;
            assert!(response.text.contains("pong"));
        }

        Ok(())
    }

    /// Test scope shadowing
    ///
    /// # Errors
    /// Returns error if test fails
    ///
    /// # Panics
    /// Panics if assertions fail
    #[tokio::test]
    async fn test_scope_shadowing() -> Result<()> {
        let mut provider = MockProvider::new("test");

        // Outer scope with fallback
        let outer_trigger =
            TriggerConfig::new(".*".to_owned(), MatchType::Regex, MatchAgainst::Query)?;
        let outer_strategy = ResponseStrategy::Repeating {
            trigger: outer_trigger,
            typescript: "return 'outer';".to_owned(),
        };
        provider.push_scope(None, vec![outer_strategy])?;

        // Inner scope with specific pattern
        let inner_trigger = TriggerConfig::new(
            "specific".to_owned(),
            MatchType::Contains,
            MatchAgainst::Query,
        )?;
        let inner_strategy = ResponseStrategy::Once {
            trigger: inner_trigger,
            typescript: "return 'inner';".to_owned(),
        };
        provider.push_scope(None, vec![inner_strategy])?;

        let query_specific = Query::new("specific query");
        let query_generic = Query::new("generic query");
        let context = Context::new("");

        // Inner scope should match first
        let response1 = provider.generate(&query_specific, &context).await?;
        assert!(response1.text.contains("inner"));

        // After inner exhausted, should fall back to outer
        let response2 = provider.generate(&query_specific, &context).await?;
        assert!(response2.text.contains("outer"));

        // Generic query should use outer
        let response3 = provider.generate(&query_generic, &context).await?;
        assert!(response3.text.contains("outer"));

        Ok(())
    }
}
