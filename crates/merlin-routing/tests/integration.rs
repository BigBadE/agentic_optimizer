//! Integration tests organized by component

#[path = "common/mod.rs"]
pub mod common;

// Orchestrator tests
#[path = "integration/orchestrator/integration_tests.rs"]
mod integration_tests;

// Cache tests
#[path = "integration/cache/phase5_integration_tests.rs"]
mod phase5_integration_tests;

// Persistence tests
#[path = "integration/persistence/persistence_tests.rs"]
mod persistence_tests;

// Context tests
#[path = "integration/context/context_fetcher_e2e_tests.rs"]
mod context_fetcher_e2e_tests;
#[path = "integration/context/conversation_context_tests.rs"]
mod conversation_context_tests;

// Executor tests
#[path = "integration/executor/executor_tests.rs"]
mod executor_tests;
#[path = "integration/executor/self_determining_tests.rs"]
mod self_determining_tests;
