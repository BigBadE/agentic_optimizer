//! End-to-end tests for progress callback functionality to prevent UI freezing
#![cfg_attr(
    test,
    allow(
        dead_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

use merlin_context::ContextFetcher;
use merlin_core::Query;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::fs;

/// Create a test project with enough files to trigger embedding
async fn create_test_project() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::with_prefix("progress_e2e").expect("Failed to create temp dir");
    let project_root = temp_dir.path().to_path_buf();

    fs::create_dir_all(project_root.join("src")).await.unwrap();

    // Create multiple files to ensure embedding happens
    for i in 0..10 {
        fs::write(
            project_root.join(format!("src/file{i}.rs")),
            format!(
                r#"
/// Module documentation for file {i}
pub fn function_{i}() {{
    println!("Function {{}}", {i});
}}

pub struct Data{i} {{
    pub value: i32,
}}

impl Data{i} {{
    pub fn new(value: i32) -> Self {{
        Self {{ value }}
    }}
    
    pub fn process(&self) -> i32 {{
        self.value * 2
    }}
}}
"#
            ),
        )
        .await
        .unwrap();
    }

    (temp_dir, project_root)
}

#[derive(Debug, Clone)]
struct ProgressEvent {
    stage: String,
    current: u64,
    total: Option<u64>,
    timestamp: Instant,
}

/// Test that progress callback is called during context building and doesn't freeze
#[tokio::test]
#[ignore = "Requires embedding model to be available"]
async fn test_progress_callback_no_freeze() {
    let (_temp, project_root) = create_test_project().await;

    let progress_events = Arc::new(Mutex::new(Vec::<ProgressEvent>::new()));
    let events_clone = Arc::clone(&progress_events);

    let callback = Arc::new(move |stage: &str, current: u64, total: Option<u64>| {
        let event = ProgressEvent {
            stage: stage.to_owned(),
            current,
            total,
            timestamp: Instant::now(),
        };
        events_clone.lock().unwrap().push(event);
    });

    let mut fetcher = ContextFetcher::new(project_root).with_progress_callback(callback);

    let query = Query::new("Find the function that processes data");

    let start = Instant::now();
    let result = fetcher.build_context_for_query(&query).await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Context building should succeed");

    let events = progress_events.lock().unwrap();

    // Verify we got progress events
    assert!(!events.is_empty(), "Should have progress events");

    // Check for expected stages
    let stages: Vec<String> = events.iter().map(|e| e.stage.clone()).collect();
    // Progress stages: {stages:?}

    // Verify we have key stages
    let has_building_context = stages.iter().any(|s| s.contains("Building Context"));
    let has_embedding = stages
        .iter()
        .any(|s| s.contains("Embedding") || s.contains("Reading"));

    assert!(
        has_building_context || has_embedding,
        "Should have context building or embedding stage"
    );

    // Verify no long gaps between progress updates (would indicate freezing)
    for window in events.windows(2) {
        let gap = window[1].timestamp.duration_since(window[0].timestamp);
        let first_stage = &window[0].stage;
        let second_stage = &window[1].stage;
        assert!(
            gap < Duration::from_secs(30),
            "Gap between progress updates should be less than 30s, got {gap:?} between '{first_stage}' and '{second_stage}'"
        );
    }

    // Verify the operation completed in reasonable time
    assert!(
        duration < Duration::from_secs(120),
        "Context building should complete within 120s, took {duration:?}"
    );

    let event_count = events.len();
    drop(events);
    // Test completed successfully with {event_count} progress events
    assert!(event_count > 0, "Should have progress events");
}

/// Test that cache saving reports progress and doesn't freeze
#[tokio::test]
#[ignore = "Requires embedding model to be available"]
async fn test_cache_saving_progress() {
    let (_temp, project_root) = create_test_project().await;

    let progress_events = Arc::new(Mutex::new(Vec::<ProgressEvent>::new()));
    let events_clone = Arc::clone(&progress_events);

    let callback = Arc::new(move |stage: &str, current: u64, total: Option<u64>| {
        let event = ProgressEvent {
            stage: stage.to_owned(),
            current,
            total,
            timestamp: Instant::now(),
        };
        events_clone.lock().unwrap().push(event);
    });

    let mut fetcher = ContextFetcher::new(project_root).with_progress_callback(callback);

    let query = Query::new("Find all public functions");

    // First run - should build cache
    let result1 = fetcher.build_context_for_query(&query).await;
    assert!(result1.is_ok(), "First context building should succeed");

    let events = progress_events.lock().unwrap();

    // Check for cache saving stage
    let has_saving = events.iter().any(|e| e.stage.contains("Saving cache"));
    let has_complete = events.iter().any(|e| e.stage == "Complete");

    // If embedding was triggered, we should see cache saving
    if events.iter().any(|e| e.stage.contains("Embedding")) {
        assert!(
            has_saving,
            "Should have 'Saving cache' stage when embedding occurs"
        );
    }

    // Complete should come after saving
    if has_saving && has_complete {
        let saving_idx = events.iter().position(|e| e.stage.contains("Saving cache"));
        let complete_idx = events.iter().position(|e| e.stage == "Complete");

        if let (Some(save_idx), Some(comp_idx)) = (saving_idx, complete_idx) {
            assert!(
                comp_idx > save_idx,
                "Complete should come after Saving cache"
            );
        }
    }

    let event_count = events.len();
    drop(events);
    // Cache saving test completed with {event_count} events
    assert!(event_count > 0, "Should have progress events");
}

/// Test that subsequent runs with cached data don't freeze
#[tokio::test]
#[ignore = "Requires embedding model to be available"]
async fn test_cached_run_no_freeze() {
    let (_temp, project_root) = create_test_project().await;

    let progress_events = Arc::new(Mutex::new(Vec::<ProgressEvent>::new()));
    let events_clone = Arc::clone(&progress_events);

    let callback = Arc::new(move |stage: &str, current: u64, total: Option<u64>| {
        let event = ProgressEvent {
            stage: stage.to_owned(),
            current,
            total,
            timestamp: Instant::now(),
        };
        events_clone.lock().unwrap().push(event);
    });

    let mut fetcher = ContextFetcher::new(project_root.clone()).with_progress_callback(callback);

    let query = Query::new("Find struct definitions");

    // First run
    let start1 = Instant::now();
    let result1 = fetcher.build_context_for_query(&query).await;
    let _duration1 = start1.elapsed();
    assert!(result1.is_ok(), "First run should succeed");

    // Clear events
    progress_events.lock().unwrap().clear();

    // Second run with same fetcher (should use cache)
    let start2 = Instant::now();
    let result2 = fetcher.build_context_for_query(&query).await;
    let _duration2 = start2.elapsed();
    assert!(result2.is_ok(), "Second run should succeed");

    let events = progress_events.lock().unwrap();

    // Second run should be faster or similar (cached)

    // Verify no freezing in second run
    for window in events.windows(2) {
        let gap = window[1].timestamp.duration_since(window[0].timestamp);
        assert!(
            gap < Duration::from_secs(30),
            "Gap in cached run should be less than 30s, got {gap:?}"
        );
    }

    drop(events);
    // Cached run test completed successfully
}
