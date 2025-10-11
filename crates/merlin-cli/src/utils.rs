//! Utility functions for CLI operations

use anyhow::Result;
use merlin_core::{Response, TokenUsage};
use merlin_routing::{MessageLevel, UiChannel, UiEvent};
use std::env;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;

const MAX_TASKS: usize = 50;

/// Get the Merlin folder path, respecting `MERLIN_FOLDER` environment variable
///
/// If `MERLIN_FOLDER` is set, use it. Otherwise default to `project/.merlin`
pub fn get_merlin_folder(project_root: &Path) -> PathBuf {
    env::var("MERLIN_FOLDER").map_or_else(|_| project_root.join(".merlin"), PathBuf::from)
}

/// Calculate estimated cost based on token usage.
pub fn calculate_cost(usage: &TokenUsage) -> f64 {
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

/// Display response metrics
pub fn display_response_metrics(response: &Response) {
    tracing::info!("\n{sep}\n", sep = "=".repeat(80));
    tracing::info!("{text}", text = response.text);
    tracing::info!("{sep}", sep = "=".repeat(80));

    tracing::info!("\nMetrics:");
    tracing::info!("  Provider: {provider}", provider = response.provider);
    tracing::info!(
        "  Confidence: {confidence:.2}",
        confidence = response.confidence
    );
    tracing::info!("  Latency: {latency}ms", latency = response.latency_ms);
    tracing::info!("  Tokens:");
    tracing::info!("    Input: {input}", input = response.tokens_used.input);
    tracing::info!("    Output: {output}", output = response.tokens_used.output);
    tracing::info!(
        "    Cache Read: {cache_read}",
        cache_read = response.tokens_used.cache_read
    );
    tracing::info!(
        "    Cache Write: {cache_write}",
        cache_write = response.tokens_used.cache_write
    );
    tracing::info!("    Total: {total}", total = response.tokens_used.total());

    let actual_cost = calculate_cost(&response.tokens_used);
    tracing::info!("  Cost: ${actual_cost:.4}");
}

/// Clean up old task files to prevent disk space waste
///
/// # Errors
/// Returns an error if the tasks directory cannot be read.
pub fn cleanup_old_tasks(merlin_dir: &Path) -> Result<()> {
    let tasks_dir = merlin_dir.join("tasks");
    if !tasks_dir.exists() {
        return Ok(());
    }

    // Get all task files sorted by modification time
    let mut task_files: Vec<_> = fs::read_dir(&tasks_dir)?
        .filter_map(StdResult::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "gz")
        })
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            meta.modified().ok().map(|time| (entry.path(), time))
        })
        .collect();

    // Sort by modification time (newest first)
    task_files.sort_by(|left, right| right.1.cmp(&left.1));

    // Keep only the 50 most recent, delete the rest
    for (path, _) in task_files.iter().skip(MAX_TASKS) {
        if let Err(error) = fs::remove_file(path) {
            tracing::warn!("failed to remove old task file {:?}: {}", path, error);
        }
    }

    Ok(())
}

/// Write to the log file; if it fails, emit a UI warning.
pub fn try_write_log(ui: &UiChannel, writer: &mut fs::File, message: &str) {
    if let Err(error) = writeln!(writer, "{message}") {
        let () = ui.send(UiEvent::SystemMessage {
            level: MessageLevel::Warning,
            message: format!("Failed to write to log: {error}"),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread::sleep;
    use std::time::Duration;
    use tempfile::TempDir;

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

    #[test]
    fn test_cleanup_old_tasks_no_directory() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let merlin_dir = temp.path().join(".merlin");
        let result = cleanup_old_tasks(&merlin_dir);
        assert!(
            result.is_ok(),
            "Cleanup should succeed when directory doesn't exist"
        );
    }

    #[test]
    fn test_cleanup_old_tasks_under_limit() {
        const NUM_TASKS: usize = 5;
        let temp = TempDir::new().expect("Failed to create temp dir");
        let tasks_dir = temp.path().join(".merlin").join("tasks");
        fs::create_dir_all(&tasks_dir).expect("Failed to create tasks dir");

        for task_num in 0..NUM_TASKS {
            let task_file = tasks_dir.join(format!("task_{task_num}.gz"));
            fs::write(&task_file, b"test").expect("Failed to write task file");
        }

        let result = cleanup_old_tasks(temp.path().join(".merlin").as_path());
        assert!(result.is_ok(), "Cleanup should succeed");

        let remaining = fs::read_dir(&tasks_dir)
            .expect("Failed to read tasks dir")
            .count();
        assert_eq!(
            remaining, NUM_TASKS,
            "All tasks should remain when under limit"
        );
    }

    #[test]
    fn test_cleanup_old_tasks_over_limit() {
        const OVER_LIMIT: usize = MAX_TASKS + 10;

        let temp = TempDir::new().expect("Failed to create temp dir");
        let tasks_dir = temp.path().join(".merlin").join("tasks");
        fs::create_dir_all(&tasks_dir).expect("Failed to create tasks dir");

        for task_num in 0..OVER_LIMIT {
            let task_file = tasks_dir.join(format!("task_{task_num}.gz"));
            fs::write(&task_file, b"test").expect("Failed to write task file");
            sleep(Duration::from_millis(10));
        }

        let result = cleanup_old_tasks(temp.path().join(".merlin").as_path());
        assert!(result.is_ok(), "Cleanup should succeed");

        let remaining = fs::read_dir(&tasks_dir)
            .expect("Failed to read tasks dir")
            .count();
        assert_eq!(remaining, MAX_TASKS, "Should keep exactly MAX_TASKS tasks");
    }

    #[test]
    fn test_cleanup_old_tasks_ignores_non_gz() {
        const NUM_GZ_TASKS: usize = 3;
        const NUM_OTHER_FILES: usize = 2;
        const EXPECTED_TOTAL: usize = NUM_GZ_TASKS + NUM_OTHER_FILES;

        let temp = TempDir::new().expect("Failed to create temp dir");
        let tasks_dir = temp.path().join(".merlin").join("tasks");
        fs::create_dir_all(&tasks_dir).expect("Failed to create tasks dir");

        for task_num in 0..NUM_GZ_TASKS {
            let task_file = tasks_dir.join(format!("task_{task_num}.gz"));
            fs::write(&task_file, b"test").expect("Failed to write task file");
        }

        for other_num in 0..NUM_OTHER_FILES {
            let other_file = tasks_dir.join(format!("other_{other_num}.txt"));
            fs::write(&other_file, b"not gz").expect("Failed to write other file");
        }

        let result = cleanup_old_tasks(temp.path().join(".merlin").as_path());
        assert!(result.is_ok(), "Cleanup should succeed");

        let total_files = fs::read_dir(&tasks_dir)
            .expect("Failed to read tasks dir")
            .count();
        assert_eq!(total_files, EXPECTED_TOTAL, "Should preserve non-gz files");
    }

    #[test]
    fn test_cleanup_old_tasks_with_mixed_extensions() {
        const NUM_GZ_FILES: usize = 30;
        const NUM_JSON_FILES: usize = 15;

        let temp = TempDir::new().expect("Failed to create temp dir");
        let tasks_dir = temp.path().join(".merlin").join("tasks");
        fs::create_dir_all(&tasks_dir).expect("Failed to create tasks dir");

        for file_num in 0..NUM_GZ_FILES {
            let file_path = tasks_dir.join(format!("task_{file_num}.gz"));
            fs::write(&file_path, b"gz data").expect("Failed to write gz file");
            sleep(Duration::from_millis(5));
        }

        for file_num in 0..NUM_JSON_FILES {
            let file_path = tasks_dir.join(format!("data_{file_num}.json"));
            fs::write(&file_path, b"{}").expect("Failed to write json file");
        }

        let result = cleanup_old_tasks(temp.path().join(".merlin").as_path());
        assert!(result.is_ok(), "Cleanup should succeed");

        let gz_count = fs::read_dir(&tasks_dir)
            .expect("Failed to read dir")
            .filter_map(StdResult::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == "gz")
            })
            .count();

        assert_eq!(
            gz_count, NUM_GZ_FILES,
            "Should keep all gz files under limit"
        );

        let json_count = fs::read_dir(&tasks_dir)
            .expect("Failed to read dir")
            .filter_map(StdResult::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == "json")
            })
            .count();

        assert_eq!(json_count, NUM_JSON_FILES, "Should preserve all json files");
    }

    #[test]
    fn test_display_response_metrics() {
        let response = Response {
            text: "Test response".to_owned(),
            confidence: 0.95,
            tokens_used: TokenUsage {
                input: 100,
                output: 50,
                cache_read: 0,
                cache_write: 0,
            },
            provider: "test-provider".to_owned(),
            latency_ms: 250,
        };

        display_response_metrics(&response);
    }
}
