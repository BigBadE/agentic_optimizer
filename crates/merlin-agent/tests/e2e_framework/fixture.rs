//! Enhanced fixture format with comprehensive verification support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Tool call verification data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpectedToolCall {
    /// Tool name (e.g., "bash", "writeFile", "readFile")
    pub tool: String,
    /// Pattern to match in tool arguments
    pub args_pattern: Option<String>,
    /// Expected result pattern
    pub result_pattern: Option<String>,
}

/// Response verification data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResponseVerification {
    /// Pattern that must be in the response
    pub contains: Vec<String>,
    /// Pattern that must NOT be in the response
    pub not_contains: Vec<String>,
    /// Minimum response length
    pub min_length: Option<usize>,
}

/// Mock response with comprehensive tracking
#[derive(Debug, Clone, Serialize)]
pub struct MockResponse {
    /// Pattern to match in the query (substring match)
    pub pattern: String,
    /// Response to return when pattern matches
    #[serde(skip)]
    pub response: String,
    /// Expected tool calls in this response
    pub expected_tool_calls: Vec<ExpectedToolCall>,
    /// Whether this response should be used exactly once
    pub use_once: bool,
    /// Whether this response should fail (for error testing)
    pub should_fail: bool,
    /// Error message if `should_fail` is true
    pub error_message: Option<String>,
}

/// Helper struct for deserialization
#[derive(Debug, Clone, Deserialize)]
struct MockResponseRaw {
    pattern: String,
    #[serde(deserialize_with = "deserialize_string_or_array")]
    response: String,
    #[serde(default)]
    expected_tool_calls: Vec<ExpectedToolCall>,
    #[serde(default)]
    use_once: bool,
    #[serde(default)]
    should_fail: bool,
    #[serde(default)]
    error_message: Option<String>,
}

/// Deserialize either a string or an array of strings into a single string
fn deserialize_string_or_array<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    use serde_json::Value;

    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(s),
        Value::Array(arr) => {
            let lines: std::result::Result<Vec<String>, _> = arr
                .into_iter()
                .map(|v| {
                    v.as_str()
                        .ok_or_else(|| Error::custom("Array must contain only strings"))
                        .map(String::from)
                })
                .collect();
            lines.map(|lines| lines.join("\n"))
        }
        _ => Err(Error::custom("Expected string or array of strings")),
    }
}

impl<'de> Deserialize<'de> for MockResponse {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = MockResponseRaw::deserialize(deserializer)?;
        Ok(MockResponse {
            pattern: raw.pattern,
            response: raw.response,
            expected_tool_calls: raw.expected_tool_calls,
            use_once: raw.use_once,
            should_fail: raw.should_fail,
            error_message: raw.error_message,
        })
    }
}

/// File verification data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileVerification {
    /// Path to the file
    pub path: String,
    /// Expected content patterns
    pub contains: Vec<String>,
    /// Content that should NOT be present
    pub not_contains: Vec<String>,
    /// Exact content (if specified, `contains`/`not_contains` are ignored)
    pub exact_content: Option<String>,
    /// Whether file must exist
    pub must_exist: bool,
    /// Whether file must NOT exist (for deletion tests)
    pub must_not_exist: bool,
}

/// Expected outcomes with comprehensive verification
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpectedOutcomes {
    /// Whether all tasks should complete successfully
    pub all_tasks_completed: bool,
    /// Files to verify
    pub files: Vec<FileVerification>,
    /// Whether validation should pass
    pub validation_passed: bool,
    /// Expected response verification
    pub response: Option<ResponseVerification>,
    /// Expected number of tool calls (minimum)
    pub min_tool_calls: Option<usize>,
    /// Expected number of tool calls (maximum)
    pub max_tool_calls: Option<usize>,
    /// Expected number of provider calls (minimum)
    pub min_provider_calls: Option<usize>,
    /// Expected number of provider calls (maximum)
    pub max_provider_calls: Option<usize>,
}

/// Task list verification structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpectedTaskList {
    /// Total number of tasks expected
    pub total_tasks: usize,
    /// Expected task descriptions (substring match)
    pub task_descriptions: Vec<String>,
    /// Expected dependency chains (task IDs that each task depends on)
    pub dependency_chain: Vec<Vec<u32>>,
}

/// Comprehensive E2E test fixture
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct E2EFixture {
    /// Name of the test scenario
    pub name: String,
    /// Description of what this tests
    pub description: String,
    /// Initial user query
    pub initial_query: String,
    /// Mock responses for agent queries
    pub mock_responses: Vec<MockResponse>,
    /// Expected task list structure (optional, for task decomposition tests)
    pub expected_task_list: Option<ExpectedTaskList>,
    /// Expected outcomes
    pub expected_outcomes: ExpectedOutcomes,
    /// Setup files to create before test
    pub setup_files: HashMap<String, String>,
    /// Environment variables to set
    pub env_vars: HashMap<String, String>,
    /// Tags for test categorization
    pub tags: Vec<String>,
}

impl E2EFixture {
    /// Load a fixture from a JSON file.
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read fixture {}: {e}", path.as_ref().display()))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse fixture {}: {e}", path.as_ref().display()))
    }

    /// Discover all fixture files in a directory.
    ///
    /// # Errors
    /// Returns error if directory cannot be read.
    pub fn discover_fixtures(dir: impl AsRef<Path>) -> Result<Vec<Self>, String> {
        let mut fixtures = Vec::new();
        let entries = fs::read_dir(dir.as_ref())
            .map_err(|e| format!("Failed to read directory {}: {e}", dir.as_ref().display()))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match Self::load(&path) {
                    Ok(fixture) => fixtures.push(fixture),
                    Err(e) => {
                        tracing::warn!("Failed to load fixture {:?}: {}", path, e);
                    }
                }
            }
        }

        fixtures.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(fixtures)
    }

    /// Validate fixture structure.
    ///
    /// # Errors
    /// Returns error if fixture structure is invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Fixture has empty name".to_owned());
        }

        if self.initial_query.is_empty() {
            return Err("Fixture has empty initial query".to_owned());
        }

        if self.mock_responses.is_empty() {
            return Err("Fixture has no mock responses".to_owned());
        }

        // Validate task list structure if present
        if let Some(ref task_list) = self.expected_task_list {
            if task_list.task_descriptions.len() != task_list.total_tasks {
                return Err(format!(
                    "Task count mismatch: {} descriptions, {} total",
                    task_list.task_descriptions.len(),
                    task_list.total_tasks
                ));
            }

            if task_list.dependency_chain.len() != task_list.total_tasks {
                return Err(format!(
                    "Dependency chain length mismatch: {} chains, {} total tasks",
                    task_list.dependency_chain.len(),
                    task_list.total_tasks
                ));
            }

            // Validate dependencies are valid task IDs
            for (i, deps) in task_list.dependency_chain.iter().enumerate() {
                for dep in deps {
                    if *dep == 0 || *dep > task_list.total_tasks as u32 {
                        return Err(format!(
                            "Task {} has invalid dependency {} (must be 1-{})",
                            i + 1,
                            dep,
                            task_list.total_tasks
                        ));
                    }
                }
            }
        }

        // Validate file verifications
        for file_verify in &self.expected_outcomes.files {
            if file_verify.must_exist && file_verify.must_not_exist {
                return Err(format!(
                    "File {} cannot have both must_exist and must_not_exist",
                    file_verify.path
                ));
            }
        }

        // Validate tool call counts
        if let (Some(min), Some(max)) = (
            self.expected_outcomes.min_tool_calls,
            self.expected_outcomes.max_tool_calls,
        ) && min > max
        {
            return Err(format!("min_tool_calls ({min}) > max_tool_calls ({max})"));
        }

        // Validate provider call counts
        if let (Some(min), Some(max)) = (
            self.expected_outcomes.min_provider_calls,
            self.expected_outcomes.max_provider_calls,
        ) && min > max
        {
            return Err(format!(
                "min_provider_calls ({min}) > max_provider_calls ({max})"
            ));
        }

        Ok(())
    }
}
