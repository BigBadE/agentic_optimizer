//! Unified test fixture format.
//!
//! This module defines the complete fixture format for unified integration tests.
//! All tests use the same format with optional verification layers.

use crate::mock_provider::{self, ResponseStrategy};
use crate::verify::{FinalVerify, VerifyConfig};
use merlin_core::Result;
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
    /// Response strategy
    pub strategy: StrategyConfig,
    /// Optional verification before event execution
    #[serde(default)]
    pub verify_before: VerifyConfig,
    /// Optional verification after event execution
    #[serde(default)]
    pub verify_after: VerifyConfig,
    /// Optional verification (maps to `verify_after`)
    #[serde(default)]
    pub verify: VerifyConfig,
}

/// Strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StrategyConfig {
    /// Single response (one-time use)
    Once {
        /// Trigger configuration
        trigger: TriggerConfig,
        /// Response configuration
        response: ResponseConfig,
    },
    /// Sequence of responses
    Sequence {
        /// Trigger configuration
        trigger: TriggerConfig,
        /// List of responses
        responses: Vec<ResponseConfig>,
    },
    /// Repeating response
    Repeating {
        /// Trigger configuration
        trigger: TriggerConfig,
        /// Response configuration
        response: ResponseConfig,
    },
}

/// Trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggerConfig {
    /// Pattern to match
    pub pattern: String,
    /// Match type
    pub match_type: MatchType,
    /// What to match against (default: combined for backward compat)
    #[serde(default = "default_match_against")]
    pub match_against: MatchAgainst,
}

/// Default match against for backward compatibility
fn default_match_against() -> MatchAgainst {
    MatchAgainst::Combined
}

/// Match against
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchAgainst {
    /// Match against query text only
    Query,
    /// Match against system prompt only
    System,
    /// Match against combined system + query
    Combined,
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

impl StrategyConfig {
    /// Convert to ResponseStrategy
    ///
    /// # Errors
    /// Returns error if conversion fails
    pub fn to_response_strategy(&self) -> Result<ResponseStrategy> {
        match self {
            Self::Once { trigger, response } => {
                let trigger_cfg = trigger.to_trigger_config()?;
                let typescript = response.typescript.join("\n");
                Ok(ResponseStrategy::Once {
                    trigger: trigger_cfg,
                    typescript,
                })
            }
            Self::Sequence { trigger, responses } => {
                let trigger_cfg = trigger.to_trigger_config()?;
                let typescript_responses =
                    responses.iter().map(|r| r.typescript.join("\n")).collect();
                Ok(ResponseStrategy::new_sequence(trigger_cfg, typescript_responses))
            }
            Self::Repeating { trigger, response } => {
                let trigger_cfg = trigger.to_trigger_config()?;
                let typescript = response.typescript.join("\n");
                Ok(ResponseStrategy::Repeating {
                    trigger: trigger_cfg,
                    typescript,
                })
            }
        }
    }
}

impl TriggerConfig {
    /// Convert to mock provider TriggerConfig
    ///
    /// # Errors
    /// Returns error if conversion fails
    fn to_trigger_config(&self) -> Result<mock_provider::TriggerConfig> {
        let match_type = match self.match_type {
            MatchType::Exact => mock_provider::MatchType::Exact,
            MatchType::Contains => mock_provider::MatchType::Contains,
            MatchType::Regex => mock_provider::MatchType::Regex,
        };

        let match_against = match self.match_against {
            MatchAgainst::Query => mock_provider::MatchAgainst::Query,
            MatchAgainst::System => mock_provider::MatchAgainst::System,
            MatchAgainst::Combined => mock_provider::MatchAgainst::Combined,
        };

        mock_provider::TriggerConfig::new(self.pattern.clone(), match_type, match_against)
    }
}
