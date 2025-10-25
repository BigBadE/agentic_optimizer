//! Unified integration test framework.
//!
//! This crate provides a single unified testing system that can verify:
//! - TypeScript execution and tool calls
//! - File operations
//! - UI state and rendering
//! - Task execution and dependencies
//!
//! All tests use the same fixture format with optional verification layers.

mod fixture;
mod runner;
mod verifier;

pub use fixture::{
    EventType, ExecutionVerify, FileVerify, FinalVerify, LlmResponseEvent, SetupConfig,
    StateVerify, TestEvent, TestFixture, TriggerConfig, UiVerify, UserInputEvent, VerifyConfig,
};
pub use runner::{PatternMockProvider, UnifiedTestRunner};
pub use verifier::{UnifiedVerifier, VerificationResult};
