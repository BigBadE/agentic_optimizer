use async_trait::async_trait;

use crate::core::{Context, Query, Response, Result};

#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn is_available(&self) -> bool;

    async fn generate(&self, query: &Query, context: &Context) -> Result<Response>;

    fn estimate_cost(&self, context: &Context) -> f64;
}
