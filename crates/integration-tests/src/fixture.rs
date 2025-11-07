//! Unified test fixture format.
//!
//! This module defines the complete fixture format for unified integration tests.
//! All tests use the same format with optional verification layers.

use crate::verify::{FinalVerify, VerifyConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete test fixture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFixture {
    /// Test name
    pub name: String,
    /// Test description
    pub description: String,
    /// Test tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Setup configuration
    #[serde(default)]
    pub setup: SetupConfig,
    /// Event sequence
    pub events: Vec<TestEvent>,
    /// Final verification
    #[serde(default)]
    pub final_verify: FinalVerify,
}

/// Setup configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupConfig {
    /// Pre-made workspace name (from test-workspaces directory)
    /// If specified, uses that workspace read-only
    /// If not specified, creates temp workspace with files
    pub workspace: Option<String>,
    /// Files to create before test (only if workspace is not specified)
    #[serde(default)]
    pub files: HashMap<String, String>,
    /// Environment variables to set
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    /// Terminal size (width, height)
    pub terminal_size: Option<(u16, u16)>,
}

/// Test event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TestEvent {
    /// User input event
    UserInput(UserInputEvent),
    /// Key press event
    KeyPress(KeyPressEvent),
    /// LLM response event
    LlmResponse(Box<LlmResponseEvent>),
    /// Wait event
    Wait(WaitEvent),
    /// Verification event (for mid-execution state verification)
    Verify(VerifyEvent),
}

/// User input event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInputEvent {
    /// Optional event ID for explicit tracking and verification
    #[serde(default)]
    pub id: Option<String>,
    /// Event data
    pub data: UserInputData,
    /// Optional verification
    #[serde(default)]
    pub verify: VerifyConfig,
}

/// User input data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserInputData {
    /// Text to input
    pub text: String,
    /// Whether to submit
    #[serde(default)]
    pub submit: bool,
}

/// Key press event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPressEvent {
    /// Optional event ID for explicit tracking and verification
    #[serde(default)]
    pub id: Option<String>,
    /// Event data
    pub data: KeyPressData,
    /// Optional verification
    #[serde(default)]
    pub verify: VerifyConfig,
}

/// Key press data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyPressData {
    /// Key to press
    pub key: String,
    /// Optional key modifiers (ctrl, shift, alt)
    #[serde(default)]
    pub modifiers: Vec<String>,
}

/// LLM response event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponseEvent {
    /// Optional event ID for explicit tracking and verification
    #[serde(default)]
    pub id: Option<String>,
    /// Trigger configuration
    pub trigger: TriggerConfig,
    /// Response configuration
    pub response: ResponseConfig,
    /// Optional verification before event execution
    #[serde(default)]
    pub verify_before: VerifyConfig,
    /// Optional verification after event execution
    #[serde(default)]
    pub verify_after: VerifyConfig,
    /// Optional verification (legacy, maps to `verify_after`)
    #[serde(default)]
    pub verify: VerifyConfig,
    /// Capture the prompt sent to the LLM (for verification)
    #[serde(skip)]
    pub captured_prompt: Option<String>,
}

/// Trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggerConfig {
    /// Pattern to match
    pub pattern: String,
    /// Match type
    pub match_type: MatchType,
}

/// Match type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    /// Exact string match
    Exact,
    /// Contains substring
    Contains,
    /// Regex match
    Regex,
}

/// Response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseConfig {
    /// TypeScript code lines
    pub typescript: Vec<String>,
}

/// Wait event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitEvent {
    /// Event data
    pub data: WaitData,
}

/// Wait data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitData {
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// What to wait for
    pub wait_for: Option<String>,
}

/// Verify event (for mid-execution verification)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyEvent {
    /// Optional event ID
    #[serde(default)]
    pub id: Option<String>,
    /// Description of what is being verified
    #[serde(default)]
    pub description: Option<String>,
    /// Verification configuration
    pub verify: VerifyConfig,
}

/// Event type enum for pattern matching
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// User input
    UserInput,
    /// Key press
    KeyPress,
    /// LLM response
    LlmResponse,
    /// Wait
    Wait,
    /// Verify
    Verify,
}

impl TestEvent {
    /// Get event type
    #[must_use]
    pub fn event_type(&self) -> EventType {
        match self {
            Self::UserInput(_) => EventType::UserInput,
            Self::KeyPress(_) => EventType::KeyPress,
            Self::LlmResponse(_) => EventType::LlmResponse,
            Self::Wait(_) => EventType::Wait,
            Self::Verify(_) => EventType::Verify,
        }
    }

    /// Get event ID if present
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::UserInput(event) => event.id.as_deref(),
            Self::KeyPress(event) => event.id.as_deref(),
            Self::LlmResponse(event) => event.id.as_deref(),
            Self::Wait(_) => None,
            Self::Verify(event) => event.id.as_deref(),
        }
    }

    /// Get verification config
    #[must_use]
    pub fn verify_config(&self) -> &VerifyConfig {
        /// Empty verification config for events without verification
        const EMPTY_VERIFY: VerifyConfig = VerifyConfig {
            execution: None,
            files: None,
            ui: None,
            state: None,
            prompt: None,
            context: None,
            validation: None,
        };

        match self {
            Self::UserInput(event) => &event.verify,
            Self::KeyPress(event) => &event.verify,
            Self::LlmResponse(event) => &event.verify,
            Self::Wait(_) => {
                // Wait events don't have verification
                &EMPTY_VERIFY
            }
            Self::Verify(event) => &event.verify,
        }
    }
}
