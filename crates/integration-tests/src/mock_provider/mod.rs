//! Mock provider for testing with scoped response strategies.

mod provider;
mod routing_matcher;
mod strategy;

pub use provider::MockProvider;
pub use routing_matcher::RoutingMatcher;
pub use strategy::ResponseStrategy;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use merlin_core::{
        Context, ContextType, ModelProvider as _, PromptType, Query, Result, RoutingContext,
    };

    /// Test basic pattern matching
    ///
    /// # Errors
    /// Returns error if test fails
    ///
    /// # Panics
    /// Panics if assertions fail
    #[tokio::test]
    async fn test_routing_match() -> Result<()> {
        let routing_matcher = RoutingMatcher {
            context_type: Some(ContextType::Conversation),
            prompt_type: None,
            difficulty_range: None,
            retry_attempt: Some(0),
            previous_result: None,
        };

        let strategy = ResponseStrategy::Once {
            routing_match: Some(routing_matcher),
            typescript: "return 'matched';".to_owned(),
        };

        let mut strategies = HashMap::new();
        strategies.insert("test_event".to_owned(), vec![strategy]);

        let provider = MockProvider::new("test", strategies);
        provider.set_current_event(Some("test_event".to_owned()))?;

        let query = Query::new("hello world").with_routing_context(RoutingContext {
            context_type: ContextType::Conversation,
            prompt_type: PromptType::Design,
            estimated_difficulty: None,
            retry_attempt: 0,
            previous_result: None,
        });
        let context = Context::new("");

        let response = provider.generate(&query, &context).await?;
        assert!(response.text.contains("matched"));

        Ok(())
    }
}
