//! Fixture loading and discovery utilities

use merlin_core::{Result, RoutingError};
use merlin_deps::serde_json::from_str;
use std::fs;
use std::path::{Path, PathBuf};

use super::fixture::TestFixture;

/// Load a test fixture from a JSON file
///
/// # Errors
/// Returns error if file reading or parsing fails
pub fn load_fixture(path: &Path) -> Result<TestFixture> {
    let content = fs::read_to_string(path)
        .map_err(|err| RoutingError::Other(format!("Failed to read fixture: {err}")))?;
    from_str(&content).map_err(|err| RoutingError::Other(format!("Failed to parse fixture: {err}")))
}

/// Discover all fixtures in directory
///
/// # Errors
/// Returns error if directory reading fails
pub fn discover_fixtures(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut fixtures = Vec::new();

    if !dir.exists() {
        return Ok(fixtures);
    }

    let entries = fs::read_dir(dir)
        .map_err(|err| RoutingError::Other(format!("Failed to read directory: {err}")))?;

    for entry in entries {
        let entry =
            entry.map_err(|err| RoutingError::Other(format!("Failed to read entry: {err}")))?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            fixtures.push(path);
        } else if path.is_dir() {
            // Recurse into subdirectories
            let mut sub_fixtures = discover_fixtures(&path)?;
            fixtures.append(&mut sub_fixtures);
        }
    }

    Ok(fixtures)
}
