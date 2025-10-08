use async_trait::async_trait;

use crate::{Context, Query, Response, Result};

/// Trait for AI model providers that can generate responses to queries.
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Returns the unique identifier for this provider.
    fn name(&self) -> &'static str;

    /// Checks whether this provider is currently available and ready to process requests.
    async fn is_available(&self) -> bool;

    /// Generates a response to the given query using the provided context.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is unavailable, the request fails,
    /// or the response cannot be parsed.
    async fn generate(&self, query: &Query, context: &Context) -> Result<Response>;

    /// Estimates the cost in USD for processing the given context.
    fn estimate_cost(&self, context: &Context) -> f64;
}
