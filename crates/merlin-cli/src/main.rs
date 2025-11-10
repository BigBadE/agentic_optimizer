//! Merlin CLI - Interactive AI coding assistant command-line interface

use anyhow::{Context as _, Result};
use cli::Cli;
use tokio::task::LocalSet;

mod cli;
mod config;
mod handlers;
mod interactive;
mod ui;
mod utils;

/// Main entry point for Merlin CLI
///
/// # Errors
/// Returns error if CLI parsing or handler execution fails
///
/// # Panics
/// Panics if tokio runtime initialization fails
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse().context("Failed to parse command-line arguments")?;

    // Wrap entire execution in LocalSet to support !Send TypeScript runtime
    LocalSet::new()
        .run_until(async {
            handlers::handle_interactive(cli.project, cli.validation, cli.local, cli.context_dump)
                .await
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Error;
    use filetime::{FileTime, set_file_mtime};
    use merlin_core::TokenUsage;
    use std::fs;
    use std::result::Result as StdResult;
    use tempfile::TempDir;

    /// Calculate estimated cost based on token usage.
    fn calculate_cost(usage: &TokenUsage) -> f64 {
        const INPUT_COST: f64 = 3.0 / 1_000_000.0;
        const OUTPUT_COST: f64 = 15.0 / 1_000_000.0;
        const CACHE_READ_COST: f64 = 0.3 / 1_000_000.0;
        const CACHE_WRITE_COST: f64 = 3.75 / 1_000_000.0;

        (usage.cache_write as f64).mul_add(
            CACHE_WRITE_COST,
            (usage.cache_read as f64).mul_add(
                CACHE_READ_COST,
                (usage.output as f64).mul_add(OUTPUT_COST, usage.input as f64 * INPUT_COST),
            ),
        )
    }

    /// Tests basic cost calculation without cache.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_calculate_cost_basic() {
        const EXPECTED_COST: f64 = 0.0105;
        const TOLERANCE: f64 = 0.0001;

        let usage = TokenUsage {
            input: 1000,
            output: 500,
            cache_read: 0,
            cache_write: 0,
        };
        let cost = calculate_cost(&usage);
        assert!(
            (cost - EXPECTED_COST).abs() < TOLERANCE,
            "Expected cost ~{EXPECTED_COST}, got {cost}"
        );
    }

    /// Tests cost calculation with cache tokens.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_calculate_cost_with_cache() {
        const EXPECTED_COST: f64 = 0.01485;
        const TOLERANCE: f64 = 0.00001;

        let usage = TokenUsage {
            input: 1000,
            output: 500,
            cache_read: 2000,
            cache_write: 1000,
        };
        let cost = calculate_cost(&usage);
        assert!(
            (cost - EXPECTED_COST).abs() < TOLERANCE,
            "Expected cost with cache ~{EXPECTED_COST}, got {cost}"
        );
    }

    /// Tests cost calculation with zero tokens.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_calculate_cost_zero_tokens() {
        const TOLERANCE: f64 = 0.0001;

        let usage = TokenUsage {
            input: 0,
            output: 0,
            cache_read: 0,
            cache_write: 0,
        };
        let cost = calculate_cost(&usage);
        assert!(
            cost.abs() < TOLERANCE,
            "Zero tokens should result in zero cost, got {cost}"
        );
    }

    /// Tests cost calculation with large token values.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_calculate_cost_large_values() {
        const LARGE_INPUT: u64 = 1_000_000;
        const LARGE_OUTPUT: u64 = 500_000;
        const EXPECTED_COST: f64 = 10.5;
        const TOLERANCE: f64 = 0.01;

        let usage = TokenUsage {
            input: LARGE_INPUT,
            output: LARGE_OUTPUT,
            cache_read: 0,
            cache_write: 0,
        };
        let cost = calculate_cost(&usage);
        assert!(
            (cost - EXPECTED_COST).abs() < TOLERANCE,
            "Expected large cost ~{EXPECTED_COST}, got {cost}"
        );
    }

    /// Tests cleanup when .merlin directory doesn't exist.
    ///
    /// # Errors
    /// Returns error if temp directory creation or cleanup fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_cleanup_old_tasks_no_directory() -> Result<(), Error> {
        let temp = TempDir::new()?;
        let merlin_dir = temp.path().join(".merlin");
        let result = utils::cleanup_old_tasks(&merlin_dir);
        assert!(
            result.is_ok(),
            "Cleanup should succeed when directory doesn't exist"
        );
        Ok(())
    }

    /// Tests cleanup when task count is under limit.
    ///
    /// # Errors
    /// Returns error if temp directory creation or file operations fail.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_cleanup_old_tasks_under_limit() -> Result<(), Error> {
        const NUM_TASKS: usize = 5;
        let temp = TempDir::new()?;
        let tasks_dir = temp.path().join(".merlin").join("tasks");
        assert!(
            fs::create_dir_all(&tasks_dir).is_ok(),
            "Failed to create tasks dir"
        );

        for task_num in 0..NUM_TASKS {
            let task_file = tasks_dir.join(format!("task_{task_num}.gz"));
            assert!(
                fs::write(&task_file, b"test").is_ok(),
                "Failed to write task file"
            );
        }

        let result = utils::cleanup_old_tasks(temp.path().join(".merlin").as_path());
        assert!(result.is_ok(), "Cleanup should succeed");

        let remaining_result = fs::read_dir(&tasks_dir);
        assert!(remaining_result.is_ok(), "Failed to read tasks dir");
        let remaining = remaining_result.map(Iterator::count).unwrap_or(0);
        assert_eq!(
            remaining, NUM_TASKS,
            "All tasks should remain when under limit"
        );
        Ok(())
    }

    /// Tests cleanup when task count exceeds limit.
    ///
    /// # Errors
    /// Returns error if temp directory creation or file operations fail.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_cleanup_old_tasks_over_limit() -> Result<(), Error> {
        const MAX_TASKS: usize = 50;
        const OVER_LIMIT: usize = MAX_TASKS + 10;

        let temp = TempDir::new()?;
        let tasks_dir = temp.path().join(".merlin").join("tasks");
        assert!(
            fs::create_dir_all(&tasks_dir).is_ok(),
            "Failed to create tasks dir"
        );

        // Create files with explicitly set timestamps (no sleep needed)
        for task_num in 0..OVER_LIMIT {
            let task_file = tasks_dir.join(format!("task_{task_num}.gz"));
            assert!(
                fs::write(&task_file, b"test").is_ok(),
                "Failed to write task file"
            );

            // Set mtime to task_num seconds ago (older files have lower numbers)
            let mtime = FileTime::from_unix_time(1_000_000 + task_num as i64, 0);
            assert!(
                set_file_mtime(&task_file, mtime).is_ok(),
                "Failed to set mtime"
            );
        }

        let result = utils::cleanup_old_tasks(temp.path().join(".merlin").as_path());
        assert!(result.is_ok(), "Cleanup should succeed");

        let remaining_result = fs::read_dir(&tasks_dir);
        assert!(remaining_result.is_ok(), "Failed to read tasks dir");
        let remaining = remaining_result.map(Iterator::count).unwrap_or(0);
        assert_eq!(remaining, MAX_TASKS, "Should keep exactly MAX_TASKS tasks");
        Ok(())
    }

    /// Tests cleanup ignores non-gz files.
    ///
    /// # Errors
    /// Returns error if temp directory creation or file operations fail.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_cleanup_old_tasks_ignores_non_gz() -> Result<(), Error> {
        const NUM_GZ_TASKS: usize = 3;
        const NUM_OTHER_FILES: usize = 2;
        const EXPECTED_TOTAL: usize = NUM_GZ_TASKS + NUM_OTHER_FILES;

        let temp = TempDir::new()?;
        let tasks_dir = temp.path().join(".merlin").join("tasks");
        assert!(
            fs::create_dir_all(&tasks_dir).is_ok(),
            "Failed to create tasks dir"
        );

        for task_num in 0..NUM_GZ_TASKS {
            let task_file = tasks_dir.join(format!("task_{task_num}.gz"));
            assert!(
                fs::write(&task_file, b"test").is_ok(),
                "Failed to write task file"
            );
        }

        for other_num in 0..NUM_OTHER_FILES {
            let other_file = tasks_dir.join(format!("other_{other_num}.txt"));
            assert!(
                fs::write(&other_file, b"not gz").is_ok(),
                "Failed to write other file"
            );
        }

        let result = utils::cleanup_old_tasks(temp.path().join(".merlin").as_path());
        assert!(result.is_ok(), "Cleanup should succeed");

        let total_files_result = fs::read_dir(&tasks_dir);
        assert!(total_files_result.is_ok(), "Failed to read tasks dir");
        let total_files = total_files_result.map(Iterator::count).unwrap_or(0);
        assert_eq!(total_files, EXPECTED_TOTAL, "Should preserve non-gz files");
        Ok(())
    }

    /// Tests cleanup with mixed file extensions.
    ///
    /// # Errors
    /// Returns error if temp directory creation or file operations fail.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_cleanup_old_tasks_with_mixed_extensions() -> Result<(), Error> {
        const NUM_GZ_FILES: usize = 30;
        const NUM_JSON_FILES: usize = 15;

        let temp = TempDir::new()?;
        let tasks_dir = temp.path().join(".merlin").join("tasks");
        assert!(
            fs::create_dir_all(&tasks_dir).is_ok(),
            "Failed to create tasks dir"
        );

        // Create gz files with explicit timestamps
        for file_num in 0..NUM_GZ_FILES {
            let file_path = tasks_dir.join(format!("task_{file_num}.gz"));
            assert!(
                fs::write(&file_path, b"gz data").is_ok(),
                "Failed to write gz file"
            );

            // Set mtime to file_num seconds in the future
            let mtime = FileTime::from_unix_time(1_000_000 + file_num as i64, 0);
            assert!(
                set_file_mtime(&file_path, mtime).is_ok(),
                "Failed to set mtime"
            );
        }

        for file_num in 0..NUM_JSON_FILES {
            let file_path = tasks_dir.join(format!("data_{file_num}.json"));
            assert!(
                fs::write(&file_path, b"{}").is_ok(),
                "Failed to write json file"
            );
        }

        let result = utils::cleanup_old_tasks(temp.path().join(".merlin").as_path());
        assert!(result.is_ok(), "Cleanup should succeed");

        let gz_count_result = fs::read_dir(&tasks_dir);
        assert!(gz_count_result.is_ok(), "Failed to read dir");
        let gz_count = gz_count_result.ok().map_or(0, |dir| {
            dir.filter_map(StdResult::ok)
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext == "gz")
                })
                .count()
        });

        assert_eq!(
            gz_count, NUM_GZ_FILES,
            "Should keep all gz files under limit"
        );

        let json_count_result = fs::read_dir(&tasks_dir);
        assert!(json_count_result.is_ok(), "Failed to read dir");
        let json_count = json_count_result.ok().map_or(0, |dir| {
            dir.filter_map(StdResult::ok)
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext == "json")
                })
                .count()
        });

        assert_eq!(json_count, NUM_JSON_FILES, "Should preserve all json files");
        Ok(())
    }
}
