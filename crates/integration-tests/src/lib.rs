//! Unified integration test framework.
//!
//! This crate provides a single unified testing system that can verify:
//! - TypeScript execution and tool calls
//! - File operations
//! - UI state and rendering
//! - Task execution and dependencies
//!
//! All tests use the same fixture format with optional verification layers.

mod event_source;
mod execution_tracker;
mod execution_verifier;
mod file_verifier;
mod fixture;
mod fixture_loader;
mod mock_provider;
mod prompt_verifier;
mod runner;
mod timing;
mod tui_test_helpers;
mod ui_verifier;
mod verification_result;
mod verifier;
mod verify;
mod workspace_setup;

pub use fixture::{
    EventType, LlmResponseEvent, SetupConfig, TestEvent, TestFixture, TriggerConfig, UserInputEvent,
};
pub use mock_provider::MockProvider;
pub use runner::UnifiedTestRunner;
pub use timing::{TimingData, TimingLayer};
pub use verification_result::VerificationResult;
pub use verifier::UnifiedVerifier;
pub use verify::{
    ContextVerify, ExecutionVerify, FileVerify, FinalVerify, PromptVerify, StateVerify, UiVerify,
    ValidationVerify, VerifyConfig, WorkUnitVerify,
};
