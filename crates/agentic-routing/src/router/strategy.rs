use async_trait::async_trait;
use crate::{ModelTier, Result, Task};

/// Trait for individual routing strategies
#[async_trait]
pub trait RoutingStrategy: Send + Sync {
    /// Check if this strategy applies to the task
    fn applies_to(&self, task: &Task) -> bool;
    
    /// Select model tier for this task
    async fn select_tier(&self, task: &Task) -> Result<ModelTier>;
    
    /// Priority of this strategy (higher = evaluated first)
    fn priority(&self) -> u8;
    
    /// Name of this strategy for debugging
    fn name(&self) -> &'static str;
}
