//! Comprehensive tests for TUI task persistence
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::tests_outside_test_module,
    clippy::missing_panics_doc,
    clippy::similar_names,
    clippy::min_ident_chars,
    reason = "Tests allow these"
)]

use merlin_routing::TaskId;
use merlin_routing::user_interface::output_tree::OutputTree;
use merlin_routing::user_interface::persistence::TaskPersistence;
use merlin_routing::user_interface::task_manager::{TaskDisplay, TaskStatus};
use std::collections::HashMap;
use std::fs;
use std::io::Result as IoResult;
use std::time::Instant;
use tempfile::TempDir;

type LoadResult = IoResult<HashMap<TaskId, TaskDisplay>>;

fn create_test_task(description: &str, status: TaskStatus) -> TaskDisplay {
    let mut output_tree = OutputTree::default();
    output_tree.add_text(format!("Test output for {description}"));

    let end_time = (status == TaskStatus::Completed).then(Instant::now);

    TaskDisplay {
        description: description.to_string(),
        status,
        progress: None,
        output_lines: Vec::new(),
        start_time: Instant::now(),
        end_time,
        parent_id: None,
        output_tree,
        steps: Vec::new(),
    }
}

#[test]
fn test_persistence_creation() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    assert_eq!(persistence.get_tasks_dir(), temp_dir.path());
}

#[test]
fn test_save_task() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    let task_id = TaskId::default();
    let task = create_test_task("Test task", TaskStatus::Running);

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    let result = persistence.save_task(task_id, &task);
    assert!(result.is_ok(), "Should save task successfully");

    // Check file exists
    let task_files: Vec<_> = fs::read_dir(persistence.get_tasks_dir())
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    assert_eq!(task_files.len(), 1, "Should have one task file");
}

#[tokio::test]
async fn test_load_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    let tasks = persistence.load_all_tasks().await.unwrap();
    assert!(
        tasks.is_empty(),
        "Should load empty task map from non-existent directory"
    );
}

#[tokio::test]
async fn test_save_and_load_task() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    let task_id = TaskId::default();
    let task = create_test_task("Saveable task", TaskStatus::Completed);

    // Save
    persistence.save_task(task_id, &task).unwrap();

    // Load
    let loaded_tasks = persistence.load_all_tasks().await.unwrap();

    assert_eq!(loaded_tasks.len(), 1, "Should load one task");
    assert!(
        loaded_tasks.contains_key(&task_id),
        "Should contain saved task ID"
    );

    let loaded_task = &loaded_tasks[&task_id];
    assert_eq!(loaded_task.description, "Saveable task");
    assert_eq!(loaded_task.status, TaskStatus::Completed);
}

#[tokio::test]
async fn test_save_multiple_tasks() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    let task1_id = TaskId::default();
    let task1 = create_test_task("Task 1", TaskStatus::Completed);

    let task2_id = TaskId::default();
    let task2 = create_test_task("Task 2", TaskStatus::Running);

    let task3_id = TaskId::default();
    let task3 = create_test_task("Task 3", TaskStatus::Failed);

    persistence.save_task(task1_id, &task1).unwrap();
    persistence.save_task(task2_id, &task2).unwrap();
    persistence.save_task(task3_id, &task3).unwrap();

    let loaded_tasks = persistence.load_all_tasks().await.unwrap();

    assert_eq!(loaded_tasks.len(), 3, "Should load three tasks");
    assert!(loaded_tasks.contains_key(&task1_id));
    assert!(loaded_tasks.contains_key(&task2_id));
    assert!(loaded_tasks.contains_key(&task3_id));
}

#[test]
fn test_delete_task_file() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    let task_id = TaskId::default();
    let task = create_test_task("Deletable task", TaskStatus::Running);

    // Save
    persistence.save_task(task_id, &task).unwrap();

    // Verify exists
    let files_before = fs::read_dir(persistence.get_tasks_dir()).unwrap().count();
    assert_eq!(files_before, 1);

    // Delete
    let result = persistence.delete_task_file(task_id);
    assert!(result.is_ok(), "Should delete task file successfully");

    // Verify deleted
    let files_after = fs::read_dir(persistence.get_tasks_dir()).unwrap().count();
    assert_eq!(files_after, 0);
}

#[tokio::test]
async fn test_task_status_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    // Test each status
    let statuses = vec![
        (TaskStatus::Running, "Running task"),
        (TaskStatus::Completed, "Completed task"),
        (TaskStatus::Failed, "Failed task"),
    ];

    for (status, desc) in statuses {
        let task_id = TaskId::default();
        let task = create_test_task(desc, status);

        persistence.save_task(task_id, &task).unwrap();

        let loaded = persistence.load_all_tasks().await.unwrap();
        let loaded_task = &loaded[&task_id];

        assert_eq!(
            loaded_task.status, status,
            "Status should persist correctly"
        );
    }
}

#[tokio::test]
async fn test_output_text_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    let task_id = TaskId::default();
    let mut task = create_test_task("Output task", TaskStatus::Running);

    // Add multiple output lines
    task.output_tree.add_text("Line 1".to_string());
    task.output_tree.add_text("Line 2".to_string());
    task.output_tree.add_text("Line 3".to_string());

    persistence.save_task(task_id, &task).unwrap();

    let loaded = persistence.load_all_tasks().await.unwrap();
    let loaded_task = &loaded[&task_id];

    let output_text = loaded_task.output_tree.to_text();
    assert!(
        output_text.contains("Line 1"),
        "Output should contain line 1"
    );
    assert!(
        output_text.contains("Line 2"),
        "Output should contain line 2"
    );
    assert!(
        output_text.contains("Line 3"),
        "Output should contain line 3"
    );
}

#[tokio::test]
async fn test_parent_id_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    let parent_id = TaskId::default();
    let child_id = TaskId::default();

    let mut child_task = create_test_task("Child task", TaskStatus::Running);
    child_task.parent_id = Some(parent_id);

    persistence.save_task(child_id, &child_task).unwrap();

    let loaded = persistence.load_all_tasks().await.unwrap();
    let loaded_child = &loaded[&child_id];

    assert_eq!(
        loaded_child.parent_id,
        Some(parent_id),
        "Parent ID should persist"
    );
}

#[test]
fn test_delete_nonexistent_task() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    let fake_id = TaskId::default();
    let result = persistence.delete_task_file(fake_id);

    assert!(
        result.is_err(),
        "Should error when deleting nonexistent task"
    );
}

#[tokio::test]
async fn test_compression_effectiveness() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    let task_id = TaskId::default();
    let mut task = create_test_task("Large output task", TaskStatus::Running);

    // Add lots of text
    for i in 0..100 {
        task.output_tree.add_text(format!(
            "Line {i}: This is a repeated line with similar content"
        ));
    }

    persistence.save_task(task_id, &task).unwrap();

    // Check file is compressed (should be much smaller than raw JSON)
    let task_files: Vec<_> = fs::read_dir(persistence.get_tasks_dir())
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    assert_eq!(task_files.len(), 1);
    let file_size = fs::metadata(task_files[0].path()).unwrap().len();

    // Compressed size should be reasonable (exact size depends on compression)
    assert!(
        file_size < 10000,
        "Compressed file should be reasonably small"
    );
}

#[tokio::test]
async fn test_special_characters_in_description() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    let task_id = TaskId::default();
    let special_desc = "Task with \"quotes\" and 'apostrophes' and /slashes\\ and émojis 🚀";
    let task = create_test_task(special_desc, TaskStatus::Running);

    persistence.save_task(task_id, &task).unwrap();

    let loaded = persistence.load_all_tasks().await.unwrap();
    let loaded_task = &loaded[&task_id];

    assert_eq!(
        loaded_task.description, special_desc,
        "Special characters should persist"
    );
}

#[tokio::test]
async fn test_concurrent_loads() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    // Save some tasks
    for i in 0..5 {
        let task_id = TaskId::default();
        let task = create_test_task(&format!("Task {i}"), TaskStatus::Running);
        persistence.save_task(task_id, &task).unwrap();
    }

    // Load concurrently
    let persistence1 = TaskPersistence::new(temp_dir.path().to_path_buf());
    let persistence2 = TaskPersistence::new(temp_dir.path().to_path_buf());

    let (result1, result2): (LoadResult, LoadResult) =
        tokio::join!(persistence1.load_all_tasks(), persistence2.load_all_tasks());

    let tasks1 = result1.unwrap();
    let tasks2 = result2.unwrap();

    assert_eq!(tasks1.len(), 5);
    assert_eq!(tasks2.len(), 5);
}

#[tokio::test]
async fn test_unicode_in_output() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    let task_id = TaskId::default();
    let mut task = create_test_task("Unicode task", TaskStatus::Running);

    task.output_tree.add_text("Hello 世界 🌍".to_string());
    task.output_tree.add_text("Привет мир".to_string());
    task.output_tree.add_text("مرحبا بالعالم".to_string());

    persistence.save_task(task_id, &task).unwrap();

    let loaded = persistence.load_all_tasks().await.unwrap();
    let loaded_task = &loaded[&task_id];

    let output = loaded_task.output_tree.to_text();
    assert!(output.contains("世界"));
    assert!(output.contains("Привет"));
    assert!(output.contains("مرحبا"));
}

#[tokio::test]
async fn test_end_time_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let persistence = TaskPersistence::new(temp_dir.path().to_path_buf());

    fs::create_dir_all(persistence.get_tasks_dir()).unwrap();

    let task_id = TaskId::default();
    let task_completed = create_test_task("Completed", TaskStatus::Completed);
    let task_running = create_test_task("Running", TaskStatus::Running);

    persistence.save_task(task_id, &task_completed).unwrap();
    let loaded = persistence.load_all_tasks().await.unwrap();
    assert!(
        loaded[&task_id].end_time.is_some(),
        "Completed task should have end time"
    );

    let task2_id = TaskId::default();
    persistence.save_task(task2_id, &task_running).unwrap();
    let loaded2 = persistence.load_all_tasks().await.unwrap();
    assert!(
        loaded2[&task2_id].end_time.is_none(),
        "Running task should not have end time"
    );
}
