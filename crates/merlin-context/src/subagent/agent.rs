//! Trait definition for context planning agents.

use merlin_core::Result;
use crate::query::{QueryIntent, ContextPlan};

/// Trait for agents that generate context plans
pub trait ContextAgent: Send + Sync {
    /// Generate a context plan for the given query intent
    ///
    /// # Errors
    /// Returns an error if the agent fails to generate a plan
    fn generate_plan_sync(&self, intent: &QueryIntent, query_text: &str) -> Result<ContextPlan>;
    
    /// Check if the agent is available (e.g., Ollama is running)
    ///
    /// # Errors
    /// Returns an error if the availability check fails
    fn is_available_sync(&self) -> Result<bool>;
    
    /// Get the name of this agent
    fn name(&self) -> &str;
}

