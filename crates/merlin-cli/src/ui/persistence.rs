use super::task_manager::{TaskDisplay, TaskStatus};
use merlin_core::ThreadId;
use merlin_deps::flate2::{Compression, read::GzDecoder, write::GzEncoder};
use merlin_deps::serde_json::{from_str, to_string};
use merlin_routing::TaskId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self as filesystem, File};
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};
use tokio::fs as async_fs;

/// Serializable task representation for disk storage
#[derive(Serialize, Deserialize)]
struct SerializableTask {
    id: TaskId,
    description: String,
    status: String,
    output_text: String,
    output_lines: Vec<String>,
    created_at: SystemTime,
    timestamp: SystemTime,
    thread_id: Option<ThreadId>,
}

/// Handles task persistence to disk
pub struct TaskPersistence {
    tasks_dir: PathBuf,
}

impl TaskPersistence {
    /// Creates a new `TaskPersistence` instance
    pub fn new(tasks_dir: PathBuf) -> Self {
        Self { tasks_dir }
    }

    /// Loads all tasks from disk
    ///
    /// # Errors
    ///
    /// Returns an error if the task directory cannot be read or task files cannot be deserialized
    pub async fn load_all_tasks(&self) -> io::Result<HashMap<TaskId, TaskDisplay>> {
        let mut tasks = HashMap::default();

        // Check if directory exists using tokio async fs
        if !async_fs::try_exists(&self.tasks_dir).await.unwrap_or(false) {
            return Ok(tasks);
        }

        let mut entries = async_fs::read_dir(&self.tasks_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if !is_compressed_task_file(&path) {
                continue;
            }

            match load_single_task(&path).await {
                Ok(Some(task_display)) => {
                    tasks.insert(task_display.0, task_display.1);
                }
                Ok(None) => {}
                Err(error) => {
                    merlin_deps::tracing::warn!("Failed to load task file {:?}: {}", path, error);
                }
            }
        }

        Ok(tasks)
    }

    /// Saves a task to disk
    ///
    /// # Errors
    ///
    /// Returns an error if the task directory cannot be created or the task file cannot be written
    pub fn save_task(&self, task_id: TaskId, task: &TaskDisplay) -> io::Result<()> {
        // Ensure the tasks directory exists (only if not already present)
        if !self.tasks_dir.exists() {
            filesystem::create_dir_all(&self.tasks_dir)?;
        }

        let status_str = task_status_to_string(task.status);

        // Convert Instant to SystemTime by calculating elapsed time from task start
        let now_instant = Instant::now();
        let now_system = SystemTime::now();
        let elapsed = now_instant.duration_since(task.timestamp);
        let timestamp = now_system - elapsed;

        let serializable = SerializableTask {
            id: task_id,
            description: task.description.clone(),
            status: status_str.to_string(),
            output_text: task.output.clone(),
            output_lines: task.output_lines.clone(),
            created_at: task.created_at,
            timestamp,
            thread_id: task.thread_id,
        };

        let filename = format!("{}.json.gz", extract_task_id_string(task_id));
        let path = self.tasks_dir.join(filename);

        write_compressed_task(&path, &serializable)
    }

    /// Deletes a task file from disk
    ///
    /// # Errors
    ///
    /// Returns an error if the task file cannot be removed
    pub fn delete_task_file(&self, task_id: TaskId) -> io::Result<()> {
        let filename = format!("{}.json.gz", extract_task_id_string(task_id));
        let task_file = self.tasks_dir.join(filename);
        filesystem::remove_file(task_file)
    }
}

// Helper functions
/// Checks if a path is a compressed task file
fn is_compressed_task_file(path: &Path) -> bool {
    path.extension().and_then(OsStr::to_str) == Some("gz")
}

type LoadedTask = Option<(TaskId, TaskDisplay)>;

/// Loads a single task from a file
///
/// # Errors
/// Returns an error if the file cannot be opened, the gzip decoding fails, or
async fn load_single_task(path: &Path) -> io::Result<LoadedTask> {
    // Read file asynchronously
    let compressed_data = async_fs::read(path).await?;

    // Decompress (this is CPU-bound but fast, so keep it sync)
    let mut decoder = GzDecoder::new(&compressed_data[..]);
    let mut json_str = String::default();
    decoder.read_to_string(&mut json_str)?;

    let serializable: SerializableTask =
        from_str(&json_str).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    let task_display = deserialize_task(serializable);
    Ok(Some(task_display))
}

/// Deserializes a task from its serializable form
fn deserialize_task(serializable: SerializableTask) -> (TaskId, TaskDisplay) {
    let status = match serializable.status.as_str() {
        "Completed" => TaskStatus::Completed,
        "Failed" => TaskStatus::Failed,
        _ => TaskStatus::Running,
    };

    // Convert SystemTime to Instant by calculating offset from now
    let now_instant = Instant::now();
    let now_system = SystemTime::now();

    let timestamp = now_system
        .duration_since(serializable.timestamp)
        .map_or(now_instant, |elapsed| {
            now_instant.checked_sub(elapsed).unwrap_or(now_instant)
        });

    let task_display = TaskDisplay {
        description: serializable.description,
        status,
        progress: None,
        output_lines: serializable.output_lines,
        created_at: serializable.created_at,
        timestamp,
        thread_id: serializable.thread_id,
        output: serializable.output_text,
        steps: Vec::default(),
        current_step: None,
        retry_count: 0,
        work_unit: None, // No live WorkUnit for persisted tasks
    };

    (serializable.id, task_display)
}

/// Writes a compressed task to disk
///
/// # Errors
///
/// Returns an error if the JSON serialization fails or the file cannot be written
fn write_compressed_task(path: &Path, serializable: &SerializableTask) -> io::Result<()> {
    let json =
        to_string(serializable).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    let file = File::create(path)?;
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

/// Extracts clean task ID string from `TaskId` debug format
fn extract_task_id_string(task_id: TaskId) -> String {
    let task_id_str = format!("{task_id:?}");
    let Some(stripped) = task_id_str.strip_prefix("TaskId(") else {
        return task_id_str;
    };
    stripped.strip_suffix(")").unwrap_or(stripped).to_owned()
}
