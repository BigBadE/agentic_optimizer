//! Unified integration test framework.
//!
//! This crate provides a single unified testing system that can verify:
//! - TypeScript execution and tool calls
//! - File operations
//! - UI state and rendering
//! - Task execution and dependencies
//!
//! All tests use the same fixture format with optional verification layers.
#![cfg_attr(
    test,
    allow(
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        reason = "Allow for tests"
    )
)]

mod event_source;
mod execution_tracker;
mod execution_verifier;
mod file_verifier;
mod fixture;
mod fixture_loader;
mod mock_provider;
mod prompt_verifier;
mod runner;
mod ui_verifier;
mod verification_result;
mod verifier;
mod workspace_setup;

pub use fixture::{
    EventType, ExecutionVerify, FileVerify, FinalVerify, LlmResponseEvent, PromptVerify,
    SetupConfig, StateVerify, TestEvent, TestFixture, TriggerConfig, UiVerify, UserInputEvent,
    VerifyConfig,
};
pub use mock_provider::MockProvider;
pub use runner::UnifiedTestRunner;
pub use verification_result::VerificationResult;
pub use verifier::UnifiedVerifier;
