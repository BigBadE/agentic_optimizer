//! Test case definition and loading.

use crate::metrics::{ExpectedFile, Priority};
use anyhow::{Context as _, Result};
use serde::{Deserialize, Deserializer};
use std::fs::read_to_string;
use std::path::Path;
use toml::from_str;

/// Test case definition
#[derive(Debug, Clone, Deserialize)]
pub struct TestCase {
    /// Test case name
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Query to search for
    pub query: String,
    /// Project root directory
    pub project_root: String,
    /// Expected relevant files
    #[serde(default)]
    pub expected: Vec<ExpectedFile>,
    /// Files that should NOT appear
    #[serde(default)]
    pub excluded: Vec<ExcludedFile>,
}

/// Expected file entry from TOML
#[derive(Debug, Clone, Deserialize)]
struct ExpectedFileToml {
    path: String,
    priority: String,
    reason: String,
}

/// Excluded file entry from TOML
#[derive(Debug, Clone, Deserialize)]
pub struct ExcludedFile {
    /// File path
    pub path: String,
    /// Reason for exclusion
    pub reason: String,
}

impl<'de> Deserialize<'de> for ExpectedFile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let toml_file = ExpectedFileToml::deserialize(deserializer)?;
        let priority = match toml_file.priority.to_lowercase().as_str() {
            "critical" => Priority::Critical,
            "high" => Priority::High,
            "low" => Priority::Low,
            _ => Priority::Medium,
        };

        Ok(Self {
            path: toml_file.path,
            priority,
            reason: toml_file.reason,
        })
    }
}

impl TestCase {
    /// Load test case from TOML file
    ///
    /// # Errors
    /// Returns error if file cannot be read or TOML cannot be parsed
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = read_to_string(path)
            .with_context(|| format!("Failed to read test case file: {}", path.display()))?;

        let test_case: Self = from_str(&content)
            .with_context(|| format!("Failed to parse test case TOML: {}", path.display()))?;

        Ok(test_case)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml::from_str;

    #[test]
    #[allow(
        clippy::unwrap_used,
        clippy::missing_panics_doc,
        reason = "Test code can use unwrap"
    )]
    fn test_priority_parsing() {
        let toml_content = r#"
            name = "Test"
            query = "test query"
            project_root = "test"

            [[expected]]
            path = "file1.rs"
            priority = "critical"
            reason = "test"

            [[expected]]
            path = "file2.rs"
            priority = "high"
            reason = "test"
        "#;

        let test_case: TestCase = from_str(toml_content).unwrap();
        assert_eq!(test_case.expected.len(), 2);
        assert_eq!(test_case.expected[0].priority, Priority::Critical);
        assert_eq!(test_case.expected[1].priority, Priority::High);
    }
}
