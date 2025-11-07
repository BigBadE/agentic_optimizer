//! Unified test fixture format.
//!
//! This module defines the complete fixture format for unified integration tests.
//! All tests use the same format with optional verification layers.

use crate::mock_provider::{self, ResponseStrategy};
use crate::verify::{FinalVerify, VerifyConfig};
use merlin_core::{ContextType, ExecutionResult, PromptType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;

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
    /// Retry responses for this event (executed on validation failures)
    #[serde(default)]
    pub retry_responses: Vec<RetryResponse>,
}

/// Retry response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryResponse {
    /// Routing match configuration for this retry
    pub routing_match: RoutingMatchConfig,
    /// Response configuration
    pub response: ResponseConfig,
    /// Optional verification after this retry
    #[serde(default)]
    pub verify: VerifyConfig,
}

/// Strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StrategyConfig {
    /// Single response (one-time use)
    Once {
        /// Routing match configuration (optional - if None, matches any)
        #[serde(default)]
        routing_match: Option<RoutingMatchConfig>,
        /// Response configuration
        response: ResponseConfig,
    },
    /// Sequence of responses
    Sequence {
        /// Routing match configuration (optional - if None, matches any)
        #[serde(default)]
        routing_match: Option<RoutingMatchConfig>,
        /// List of responses
        responses: Vec<ResponseConfig>,
    },
    /// Repeating response
    Repeating {
        /// Routing match configuration (optional - if None, matches any)
        #[serde(default)]
        routing_match: Option<RoutingMatchConfig>,
        /// Response configuration
        response: ResponseConfig,
    },
}

/// Routing match configuration for routing-based matching
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingMatchConfig {
    /// Expected context type (optional - None matches any)
    #[serde(default)]
    pub context_type: Option<String>,
    /// Expected prompt type (optional - None matches any)
    #[serde(default)]
    pub prompt_type: Option<String>,
    /// Expected difficulty range [min, max] (optional - None matches any)
    #[serde(default)]
    pub difficulty_range: Option<(u8, u8)>,
    /// Expected retry attempt (optional - None matches any)
    #[serde(default)]
    pub retry_attempt: Option<u8>,
    /// Expected previous result (`soft_error` or `hard_error`, optional - None matches any)
    #[serde(default)]
    pub previous_result: Option<String>,
}

impl RoutingMatchConfig {
    /// Convert to mock provider `RoutingMatcher`
    #[must_use]
    pub fn to_routing_matcher(&self) -> mock_provider::RoutingMatcher {
        let context_type =
            self.context_type
                .as_ref()
                .and_then(|context_str| match context_str.as_str() {
                    "task_decomposition" => Some(ContextType::TaskDecomposition),
                    "step_execution" => Some(ContextType::StepExecution),
                    "validation" => Some(ContextType::Validation),
                    "error_recovery" => Some(ContextType::ErrorRecovery),
                    "conversation" => Some(ContextType::Conversation),
                    _ => None,
                });

        let prompt_type =
            self.prompt_type
                .as_ref()
                .and_then(|prompt_str| match prompt_str.as_str() {
                    "design" => Some(PromptType::Design),
                    "debug" => Some(PromptType::Debug),
                    "validation" => Some(PromptType::Validation),
                    "planning" => Some(PromptType::Planning),
                    _ => None,
                });

        let previous_result = self
            .previous_result
            .as_ref()
            .and_then(|result_str| match result_str.as_str() {
                "soft_error" => Some(ExecutionResult::SoftError),
                "hard_error" => Some(ExecutionResult::HardError),
                _ => None,
            });

        mock_provider::RoutingMatcher {
            context_type,
            prompt_type,
            difficulty_range: self.difficulty_range,
            retry_attempt: self.retry_attempt,
            previous_result,
        }
    }
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
    /// Convert to `ResponseStrategy`
    #[must_use]
    pub fn to_response_strategy(&self) -> ResponseStrategy {
        match self {
            Self::Once {
                routing_match,
                response,
            } => {
                let routing_matcher = routing_match
                    .as_ref()
                    .map(RoutingMatchConfig::to_routing_matcher);
                let typescript = response.typescript.join("\n");
                ResponseStrategy::Once {
                    routing_match: routing_matcher,
                    typescript,
                }
            }
            Self::Sequence {
                routing_match,
                responses,
            } => {
                let routing_matcher = routing_match
                    .as_ref()
                    .map(RoutingMatchConfig::to_routing_matcher);
                let typescript_responses = responses
                    .iter()
                    .map(|response| response.typescript.join("\n"))
                    .collect();

                ResponseStrategy::Sequence {
                    routing_match: routing_matcher,
                    responses: typescript_responses,
                    index: AtomicUsize::new(0),
                }
            }
            Self::Repeating {
                routing_match,
                response,
            } => {
                let routing_matcher = routing_match
                    .as_ref()
                    .map(RoutingMatchConfig::to_routing_matcher);
                let typescript = response.typescript.join("\n");
                ResponseStrategy::Repeating {
                    routing_match: routing_matcher,
                    typescript,
                }
            }
        }
    }
}
