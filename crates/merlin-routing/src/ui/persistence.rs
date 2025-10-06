use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::{Instant, SystemTime};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use crate::TaskId;
use super::task_manager::{TaskDisplay, TaskStatus};
use super::output_tree::OutputTree;

/// Serializable task representation for disk storage
#[derive(Serialize, Deserialize)]
struct SerializableTask {
    id: TaskId,
    description: String,
    status: String,
    output_text: String,
    start_time: SystemTime,
    end_time: Option<SystemTime>,
    parent_id: Option<TaskId>,
}

/// Handles task persistence to disk
pub struct TaskPersistence {
    tasks_dir: PathBuf,
}

impl TaskPersistence {
    /// Creates a new TaskPersistence instance
    pub fn new(tasks_dir: PathBuf) -> Self {
        Self { tasks_dir }
    }

    /// Gets the tasks directory
    pub fn get_tasks_dir(&self) -> &PathBuf {
        &self.tasks_dir
    }

    /// Loads all tasks from disk
    pub async fn load_all_tasks(&self) -> io::Result<HashMap<TaskId, TaskDisplay>> {
        let dir = self.tasks_dir.clone();

        tokio::task::spawn_blocking(move || Self::load_tasks_sync(&dir))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    /// Saves a task to disk
    pub fn save_task(&self, task_id: TaskId, task: &TaskDisplay) -> io::Result<()> {
        let status_str = task_status_to_string(task.status);

        // Convert Instant to SystemTime by calculating elapsed time from task start
        let now_instant = std::time::Instant::now();
        let now_system = SystemTime::now();
        let elapsed = now_instant.duration_since(task.start_time);
        let start_time = now_system - elapsed;

        let end_time = task.end_time.map(|end_instant| {
            let end_elapsed = now_instant.duration_since(end_instant);
            now_system - end_elapsed
        });

        let serializable = SerializableTask {
            id: task_id,
            description: task.description.clone(),
            status: status_str.to_string(),
            output_text: task.output_tree.to_text(),
            start_time,
            end_time,
            parent_id: task.parent_id,
        };

        let filename = format!("{}.json.gz", extract_task_id_string(task_id));
        let path = self.tasks_dir.join(filename);

        write_compressed_task(&path, &serializable)
    }

    /// Deletes a task file from disk
    pub fn delete_task_file(&self, task_id: TaskId) -> io::Result<()> {
        let filename = format!("{}.json.gz", extract_task_id_string(task_id));
        let task_file = self.tasks_dir.join(filename);
        std::fs::remove_file(task_file)
    }

    // Private helpers

    fn load_tasks_sync(tasks_dir: &PathBuf) -> io::Result<HashMap<TaskId, TaskDisplay>> {
        let mut tasks = HashMap::new();

        if !tasks_dir.exists() {
            return Ok(tasks);
        }

        for entry in std::fs::read_dir(tasks_dir)? {
            let entry = entry?;
            let path = entry.path();

            if is_compressed_task_file(&path) {
                if let Some(task_display) = load_single_task(&path)? {
                    tasks.insert(task_display.0, task_display.1);
                }
            }
        }

        Ok(tasks)
    }
}

// Helper functions

/// Checks if a path is a compressed task file
fn is_compressed_task_file(path: &std::path::Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("gz")
}

/// Loads a single task from a file
fn load_single_task(
    path: &std::path::Path,
) -> io::Result<Option<(TaskId, TaskDisplay)>> {
    let file = std::fs::File::open(path)?;
    let mut decoder = GzDecoder::new(file);
    let mut json_str = String::new();

    decoder.read_to_string(&mut json_str)?;

    let serializable: SerializableTask = serde_json::from_str(&json_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let task_display = deserialize_task(serializable);
    Ok(Some(task_display))
}

/// Deserializes a task from its serializable form
fn deserialize_task(serializable: SerializableTask) -> (TaskId, TaskDisplay) {
    let mut output_tree = OutputTree::new();

    for line in serializable.output_text.lines() {
        if !line.is_empty() {
            output_tree.add_text(line.to_string());
        }
    }

    let status = match serializable.status.as_str() {
        "Running" => TaskStatus::Running,
        "Completed" => TaskStatus::Completed,
        "Failed" => TaskStatus::Failed,
        _ => TaskStatus::Running,
    };

    // Convert SystemTime to Instant by calculating offset from now
    let now_instant = Instant::now();
    let now_system = SystemTime::now();

    let start_time = match now_system.duration_since(serializable.start_time) {
        Ok(elapsed) => now_instant - elapsed,
        Err(_) => now_instant, // If start_time is in future, use now
    };

    let end_time = serializable.end_time.and_then(|end_sys| {
        now_system.duration_since(end_sys).ok().map(|elapsed| now_instant - elapsed)
    });

    let task_display = TaskDisplay {
        description: serializable.description,
        status,
        progress: None,
        output_lines: Vec::new(),
        start_time,
        end_time,
        parent_id: serializable.parent_id,
        output_tree,
        steps: Vec::new(),
    };

    (serializable.id, task_display)
}

/// Writes a compressed task to disk
fn write_compressed_task(
    path: &std::path::Path,
    serializable: &SerializableTask,
) -> io::Result<()> {
    let json = serde_json::to_string(serializable)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let file = std::fs::File::create(path)?;
    let mut encoder = GzEncoder::new(file, Compression::fast());
    encoder.write_all(json.as_bytes())?;
    encoder.finish()?;

    Ok(())
}

/// Converts task status to string
fn task_status_to_string(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Running => "Running",
        TaskStatus::Completed => "Completed",
        TaskStatus::Failed => "Failed",
    }
}

/// Extracts clean task ID string from TaskId debug format
fn extract_task_id_string(task_id: TaskId) -> String {
    let task_id_str = format!("{:?}", task_id);
    task_id_str
        .strip_prefix("TaskId(")
        .and_then(|s| s.strip_suffix(")"))
        .unwrap_or(&task_id_str)
        .to_string()
}
