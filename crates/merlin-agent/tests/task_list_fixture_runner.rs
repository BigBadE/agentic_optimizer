//! Task list fixture-based testing utilities.
//!
//! Enables end-to-end testing of task decomposition and execution using
//! JSON fixture files with predefined agent responses.
#![cfg_attr(
    test,
    allow(
        dead_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::absolute_paths,
        clippy::min_ident_chars,
        clippy::use_self,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::uninlined_format_args,
        reason = "Allow for tests"
    )
)]

use merlin_agent::RoutingOrchestrator;
use merlin_core::{ModelProvider, Result, RoutingConfig};
use merlin_providers::MockProvider;
use merlin_routing::ProviderRegistry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Mock response pattern matching.
#[derive(Debug, Clone, Serialize)]
pub struct MockResponse {
    /// Pattern to match in the query (substring match)
    pub pattern: String,
    /// Response to return when pattern matches (either a string or array of lines)
    #[serde(skip)]
    pub response: String,
}

/// Helper struct for deserialization that supports both string and array formats
#[derive(Debug, Clone, Deserialize)]
struct MockResponseRaw {
    pattern: String,
    #[serde(deserialize_with = "deserialize_string_or_array")]
    response: String,
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
        })
    }
}

/// Expected task list structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpectedTaskList {
    /// Total number of tasks expected
    pub total_tasks: usize,
    /// Expected task descriptions
    pub task_descriptions: Vec<String>,
    /// Expected dependency chains (task IDs that each task depends on)
    pub dependency_chain: Vec<Vec<u32>>,
}

/// Expected test outcomes.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpectedOutcomes {
    /// Whether all tasks should complete successfully
    pub all_tasks_completed: bool,
    /// Files expected to be created
    pub files_created: Vec<String>,
    /// Whether tests should pass
    pub tests_passed: bool,
}

/// Task list test fixture.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskListFixture {
    /// Name of the test scenario
    pub name: String,
    /// Description of what this tests
    pub description: String,
    /// Initial user query
    pub initial_query: String,
    /// Mock responses for agent queries
    pub mock_responses: Vec<MockResponse>,
    /// Expected task list structure
    pub expected_task_list: ExpectedTaskList,
    /// Expected outcomes
    pub expected_outcomes: ExpectedOutcomes,
}

impl TaskListFixture {
    /// Load a fixture from a JSON file.
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|e| {
            merlin_core::RoutingError::Other(format!("Failed to read fixture: {e}"))
        })?;

        serde_json::from_str(&content)
            .map_err(|e| merlin_core::RoutingError::Other(format!("Failed to parse fixture: {e}")))
    }

    /// Create a mock provider from this fixture's responses.
    #[must_use]
    pub fn create_mock_provider(&self) -> MockProvider {
        let mut provider = MockProvider::new("test");

        for mock_response in &self.mock_responses {
            provider = provider.with_response(&mock_response.pattern, &mock_response.response);
        }

        provider
    }

    /// Create a test orchestrator with this fixture's mock provider.
    ///
    /// # Errors
    /// Returns error if orchestrator creation fails.
    pub fn create_test_orchestrator(&self) -> Result<RoutingOrchestrator> {
        let config = RoutingConfig::default();
        let _mock_provider = Arc::new(self.create_mock_provider()) as Arc<dyn ModelProvider>;

        // Create a provider registry with just the mock provider
        let _provider_registry = ProviderRegistry::new(config.clone())?;

        // TODO: Need to expose a way to register custom providers in ProviderRegistry
        // For now, this is a placeholder showing the intended usage

        RoutingOrchestrator::new(config)
    }

    /// Verify that actual task list matches expected structure.
    ///
    /// # Errors
    /// Returns error if verification fails.
    pub fn verify_task_list(&self, actual_tasks: &[TaskDescription]) -> Result<()> {
        // Verify task count
        if actual_tasks.len() != self.expected_task_list.total_tasks {
            return Err(merlin_core::RoutingError::Other(format!(
                "Expected {} tasks, got {}",
                self.expected_task_list.total_tasks,
                actual_tasks.len()
            )));
        }

        // Verify task descriptions
        for (i, expected_desc) in self.expected_task_list.task_descriptions.iter().enumerate() {
            if !actual_tasks[i].description.contains(expected_desc) {
                return Err(merlin_core::RoutingError::Other(format!(
                    "Task {} description mismatch. Expected substring: '{}', got: '{}'",
                    i, expected_desc, actual_tasks[i].description
                )));
            }
        }

        // Verify dependency chains
        for (i, expected_deps) in self.expected_task_list.dependency_chain.iter().enumerate() {
            let actual_deps = &actual_tasks[i].dependencies;

            if actual_deps.len() != expected_deps.len() {
                return Err(merlin_core::RoutingError::Other(format!(
                    "Task {} dependency count mismatch. Expected {} dependencies, got {}",
                    i,
                    expected_deps.len(),
                    actual_deps.len()
                )));
            }

            for expected_dep in expected_deps {
                if !actual_deps.contains(expected_dep) {
                    return Err(merlin_core::RoutingError::Other(format!(
                        "Task {} missing expected dependency {}. Has: {:?}, Expected to contain: {:?}",
                        i, expected_dep, actual_deps, expected_deps
                    )));
                }
            }
        }

        Ok(())
    }

    /// Convert expected task list to `TaskDescription` instances for verification.
    #[must_use]
    pub fn create_task_descriptions(&self) -> Vec<TaskDescription> {
        self.expected_task_list
            .task_descriptions
            .iter()
            .enumerate()
            .map(|(i, desc)| TaskDescription {
                id: (i + 1) as u32,
                description: desc.clone(),
                dependencies: self.expected_task_list.dependency_chain[i].clone(),
            })
            .collect()
    }

    /// Discover all fixture files in a directory.
    ///
    /// # Errors
    /// Returns error if directory cannot be read.
    pub fn discover_fixtures(dir: impl AsRef<Path>) -> Result<Vec<Self>> {
        let mut fixtures = Vec::new();
        let entries = fs::read_dir(dir.as_ref()).map_err(|e| {
            merlin_core::RoutingError::Other(format!("Failed to read directory: {e}"))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                merlin_core::RoutingError::Other(format!("Failed to read entry: {e}"))
            })?;
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
}

/// Simplified task description for verification.
#[derive(Debug, Clone)]
pub struct TaskDescription {
    /// Task ID
    pub id: u32,
    /// Task description
    pub description: String,
    /// Dependencies
    pub dependencies: Vec<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_simple_implementation_fixture() {
        let fixture_path = "tests/fixtures/task_lists/simple_implementation.json";
        let fixture = TaskListFixture::load(fixture_path).expect("Failed to load fixture");

        assert_eq!(fixture.name, "Simple Implementation Task");
        assert_eq!(fixture.expected_task_list.total_tasks, 3);
        assert_eq!(fixture.mock_responses.len(), 4);
    }

    #[test]
    fn test_create_mock_provider_from_fixture() {
        let fixture_path = "tests/fixtures/task_lists/simple_implementation.json";
        let fixture = TaskListFixture::load(fixture_path).expect("Failed to load fixture");

        let provider = fixture.create_mock_provider();

        // Verify call history tracking works
        assert_eq!(provider.call_count(), 0);
    }

    #[test]
    fn test_verify_task_list_success() {
        let fixture = TaskListFixture {
            name: "Test".to_owned(),
            description: "Test".to_owned(),
            initial_query: "Test".to_owned(),
            mock_responses: vec![],
            expected_task_list: ExpectedTaskList {
                total_tasks: 2,
                task_descriptions: vec!["Create file".to_owned(), "Write tests".to_owned()],
                dependency_chain: vec![vec![], vec![1]],
            },
            expected_outcomes: ExpectedOutcomes {
                all_tasks_completed: true,
                files_created: vec![],
                tests_passed: true,
            },
        };

        let actual_tasks = vec![
            TaskDescription {
                id: 1,
                description: "Create file with function".to_owned(),
                dependencies: vec![],
            },
            TaskDescription {
                id: 2,
                description: "Write tests for feature".to_owned(),
                dependencies: vec![1],
            },
        ];

        fixture
            .verify_task_list(&actual_tasks)
            .expect("Verification should succeed");
    }

    #[test]
    fn test_verify_task_list_count_mismatch() {
        let fixture = TaskListFixture {
            name: "Test".to_owned(),
            description: "Test".to_owned(),
            initial_query: "Test".to_owned(),
            mock_responses: vec![],
            expected_task_list: ExpectedTaskList {
                total_tasks: 2,
                task_descriptions: vec!["Create".to_owned(), "Test".to_owned()],
                dependency_chain: vec![vec![], vec![1]],
            },
            expected_outcomes: ExpectedOutcomes {
                all_tasks_completed: true,
                files_created: vec![],
                tests_passed: true,
            },
        };

        let actual_tasks = vec![TaskDescription {
            id: 1,
            description: "Create".to_owned(),
            dependencies: vec![],
        }];

        fixture.verify_task_list(&actual_tasks).unwrap_err();
    }
}
