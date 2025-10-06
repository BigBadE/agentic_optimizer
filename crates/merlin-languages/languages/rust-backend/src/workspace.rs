//! Workspace loading utilities using rust-analyzer's cargo loader.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::Context as _;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use ra_ap_ide::AnalysisHost;
use ra_ap_load_cargo::{LoadCargoConfig, load_workspace_at};
use ra_ap_project_model::{CargoConfig, RustLibSource};
use ra_ap_vfs::Vfs;

use merlin_core::{Error, Result};
use crate::cache::WorkspaceCache;

/// Configuration for workspace loading
#[derive(Debug, Clone)]
pub struct LoadConfig {
    /// Whether to load only workspace members (not dependencies)
    pub workspace_only: bool,
    /// Whether to show progress indicators
    pub show_progress: bool,
    /// Whether to use cached state if available
    pub use_cache: bool,
}

type FileMetadata = HashMap<PathBuf, SystemTime>;

fn insert_file_metadata(path: &PathBuf, metadata_map: &mut FileMetadata) {
    if let Ok(metadata) = fs::metadata(path) {
        if let Ok(modified) = metadata.modified() {
            metadata_map.insert(path.clone(), modified);
        }
    }
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            workspace_only: true,
            show_progress: true,
            use_cache: true,
        }
    }
}

/// Helper to load a Cargo workspace into rust-analyzer structures.
pub struct WorkspaceLoader {
    /// Root directory of the Cargo workspace
    project_root: PathBuf,
    /// Loading configuration
    config: LoadConfig,
}

impl WorkspaceLoader {
    /// Create a new workspace loader.
    #[must_use]
    pub fn new(project_root: &Path) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            config: LoadConfig::default(),
        }
    }

    /// Create a new workspace loader with custom configuration.
    #[must_use]
    pub fn with_config(project_root: &Path, config: LoadConfig) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            config,
        }
    }

    /// Load the workspace, returning analysis host, virtual filesystem, and a file id map.
    ///
    /// # Errors
    /// Returns an error if the workspace cannot be loaded.
    pub fn load(&self) -> Result<(AnalysisHost, Vfs, HashMap<PathBuf, ra_ap_ide::FileId>)> {
        let multi = self.config.show_progress.then(MultiProgress::new);

        let pb1 = multi.as_ref().map(|multi_progress| {
            let pb = multi_progress.add(ProgressBar::new_spinner());
            pb.set_style(ProgressStyle::default_spinner());
            pb.set_message("Checking cache...");
            pb.enable_steady_tick(Duration::from_millis(100));
            pb
        });

        if self.config.use_cache && {
            if let Ok(cache) = WorkspaceCache::load(&self.project_root) {
                if cache.is_valid(&self.project_root).unwrap_or(false) {
                    tracing::info!("Using cached rust-analyzer state ({} files)", cache.file_count);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } {
            if let Some(pb) = pb1 {
                pb.finish_with_message("\x1b[32m\u{2713}\x1b[0m Cache checked");
            }
        }

        let pb2 = multi.as_ref().map(|multi_progress| {
            let pb = multi_progress.add(ProgressBar::new_spinner());
            pb.set_style(ProgressStyle::default_spinner());
            pb.set_message("Initializing rust-analyzer...");
            pb.enable_steady_tick(Duration::from_millis(100));
            pb
        });

        tracing::info!("Loading Cargo workspace from: {}", self.project_root.display());

        let cargo_config = CargoConfig {
            sysroot: if self.config.workspace_only {
                None
            } else {
                Some(RustLibSource::Discover)
            },
            ..Default::default()
        };

        let load_config = LoadCargoConfig {
            load_out_dirs_from_check: !self.config.workspace_only,
            with_proc_macro_server: ra_ap_load_cargo::ProcMacroServerChoice::None,
            prefill_caches: false,
        };

        let progress = |message: String| {
            tracing::debug!("Workspace loading: {}", message);
        };

        let (db, vfs, _) = load_workspace_at(
            self.project_root.as_path(),
            &cargo_config,
            &load_config,
            &progress,
        )
        .with_context(|| format!("Failed to load workspace at {}", self.project_root.display()))
        .map_err(|error| Error::Other(error.to_string()))?;

        if let Some(pb) = pb2 {
            pb.finish_with_message("\x1b[32m\u{2713}\x1b[0m Rust-analyzer initialized");
        }

        let pb3 = multi.as_ref().map(|multi_progress| {
            let pb = multi_progress.add(ProgressBar::new_spinner());
            pb.set_style(ProgressStyle::default_spinner());
            pb.set_message("Building file index...");
            pb.enable_steady_tick(Duration::from_millis(100));
            pb
        });

        let host = AnalysisHost::with_database(db);

        let mut file_id_map = HashMap::new();
        let mut file_metadata = FileMetadata::new();
        
        for (file_id, path) in vfs.iter() {
            if let Some(abs_path) = path.as_path() {
                let path_buf: PathBuf = abs_path.to_path_buf().into();
                if path_buf.to_string_lossy().ends_with(".rs") {
                    file_id_map.insert(path_buf.clone(), file_id);
                    insert_file_metadata(&path_buf, &mut file_metadata);
                }
            }
        }

        if let Some(pb) = pb3 {
            pb.finish_with_message(format!("\x1b[32m\u{2713}\x1b[0m Indexed {} Rust files", file_id_map.len()));
        }

        tracing::info!("Loaded {} Rust files", file_id_map.len());

        let pb4 = multi.as_ref().map(|multi_progress| {
            let pb = multi_progress.add(ProgressBar::new_spinner());
            pb.set_style(ProgressStyle::default_spinner());
            pb.set_message("Saving cache...");
            pb.enable_steady_tick(Duration::from_millis(100));
            pb
        });

        if self.config.use_cache {
            let cache = WorkspaceCache::new(self.project_root.clone(), file_metadata);
            if let Err(error) = cache.save(&self.project_root) {
                tracing::warn!("Failed to save cache: {}", error);
            }
        }

        if let Some(pb) = pb4 {
            pb.finish_with_message("\x1b[32m\u{2713}\x1b[0m Cache saved");
        }

        Ok((host, vfs, file_id_map))
    }
}

