//! Interactive mode functionality - TUI mode

use merlin_agent::RoutingOrchestrator;
use merlin_deps::anyhow::Result;

use crate::ui::TuiApp;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use crate::utils::{cleanup_old_tasks, get_merlin_folder};

/// Initialize logging for TUI session
///
/// # Errors
/// Returns error if file operations fail
fn init_tui_logging(merlin_dir: &Path, project: &Path, local_only: bool) -> Result<fs::File> {
    let debug_log = merlin_dir.join("debug.log");

    // Open existing debug.log (already created by handle_interactive)
    let mut log_file = fs::OpenOptions::new().append(true).open(&debug_log)?;

    writeln!(
        log_file,
        "=== Session started at {:?} ===",
        SystemTime::now()
    )?;
    writeln!(log_file, "Project: {}", project.display())?;
    writeln!(
        log_file,
        "Mode: {}",
        if local_only {
            "Local Only"
        } else {
            "Multi-Model"
        }
    )?;

    Ok(log_file)
}

/// Run fully self-contained TUI interactive session
///
/// # Errors
/// Returns an error if filesystem, TUI, or async operations fail.
pub async fn run_tui_interactive(
    orchestrator: RoutingOrchestrator,
    project: PathBuf,
    local_only: bool,
) -> Result<()> {
    let merlin_dir = get_merlin_folder(&project)?;
    fs::create_dir_all(&merlin_dir)?;

    let mut log_file = init_tui_logging(&merlin_dir, &project, local_only)?;

    cleanup_old_tasks(&merlin_dir)?;

    let tasks_dir = merlin_dir.join("tasks");
    fs::create_dir_all(&tasks_dir)?;

    let log_clone = log_file.try_clone()?;
    let mut tui_app = TuiApp::new_with_storage(
        tasks_dir.clone(),
        Some(Arc::new(orchestrator)),
        Some(log_clone),
    )?;

    TuiApp::enable_raw_mode()?;

    tui_app.load_tasks_async().await;
    if let Err(err) = tui_app.load_threads() {
        merlin_deps::tracing::warn!("Failed to load threads: {err}");
    }

    let disk_task_files = fs::read_dir(&tasks_dir).map_or(0, |read_dir| {
        read_dir
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext_str| ext_str == "gz")
            })
            .count()
    });
    let parsed_tasks = tui_app.loaded_task_count();
    writeln!(
        log_file,
        "Found {} .gz file(s), parsed {} task(s) from {}",
        disk_task_files,
        parsed_tasks,
        tasks_dir.display(),
    )?;

    // Run the event loop until quit
    tui_app.run_event_loop().await?;

    tui_app.disable_raw_mode()?;
    writeln!(log_file, "=== Session ended ===")?;

    Ok(())
}
