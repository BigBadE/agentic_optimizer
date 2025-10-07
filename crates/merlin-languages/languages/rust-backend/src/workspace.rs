//! Workspace loading utilities using rust-analyzer's cargo loader.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::Context as _;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use ra_ap_ide::{AnalysisHost, FileId};
use ra_ap_load_cargo::{LoadCargoConfig, ProcMacroServerChoice, load_workspace_at};
use ra_ap_project_model::{CargoConfig, RustLibSource};
use ra_ap_vfs::Vfs;

use crate::cache::WorkspaceCache;
use merlin_core::{Error, Result};

/// File ID mapping from paths to rust-analyzer file IDs
pub type FileIdMap = HashMap<PathBuf, FileId>;

/// Loaded workspace components
pub type LoadedWorkspace = (AnalysisHost, Vfs, FileIdMap);

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
    if let Ok(metadata) = fs::metadata(path)
        && let Ok(modified) = metadata.modified()
    {
        metadata_map.insert(path.clone(), modified);
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
    pub fn load(&self) -> Result<LoadedWorkspace> {
        let progress_group = self.config.show_progress.then(MultiProgress::new);
        let progress_group = progress_group.as_ref();

        self.report_cache_status(progress_group);
        self.log_loading_path();

        let cargo_config = self.build_cargo_config();
        let load_config = self.build_load_config();
        let (host, virtual_fs) =
            self.initialize_rust_analyzer(progress_group, &cargo_config, &load_config)?;

        let (file_id_map, file_metadata) = Self::index_files(progress_group, &virtual_fs);
        self.maybe_save_cache(progress_group, file_metadata);

        Ok((host, virtual_fs, file_id_map))
    }

    fn report_cache_status(&self, progress_group: Option<&MultiProgress>) {
        let progress_cache = progress_group.map(|group| create_spinner(group, "Checking cache..."));
        if self.config.use_cache
            && let Ok(cache) = WorkspaceCache::load(&self.project_root)
            && cache.is_valid(&self.project_root).unwrap_or(false)
        {
            tracing::info!(
                "Using cached rust-analyzer state ({} files)",
                cache.file_count
            );
        }
        finish_spinner(
            progress_cache.as_ref(),
            "\x1b[32m\u{2713}\x1b[0m Cache checked",
        );
    }

    fn log_loading_path(&self) {
        tracing::info!(
            "Loading Cargo workspace from: {}",
            self.project_root.display()
        );
    }

    fn build_cargo_config(&self) -> CargoConfig {
        CargoConfig {
            sysroot: if self.config.workspace_only {
                None
            } else {
                Some(RustLibSource::Discover)
            },
            ..Default::default()
        }
    }

    fn build_load_config(&self) -> LoadCargoConfig {
        LoadCargoConfig {
            load_out_dirs_from_check: !self.config.workspace_only,
            with_proc_macro_server: ProcMacroServerChoice::None,
            prefill_caches: false,
        }
    }

    /// Initialize rust-analyzer by loading the workspace.
    ///
    /// # Errors
    /// Returns an error when the cargo workspace cannot be loaded or transformed into an analysis database.
    fn initialize_rust_analyzer(
        &self,
        progress_group: Option<&MultiProgress>,
        cargo_config: &CargoConfig,
        load_config: &LoadCargoConfig,
    ) -> Result<(AnalysisHost, Vfs)> {
        let progress_ra_init =
            progress_group.map(|group| create_spinner(group, "Initializing rust-analyzer..."));
        let on_progress = |message: String| tracing::debug!("Workspace loading: {}", message);
        let (analysis_db, virtual_fs, _) = load_workspace_at(
            self.project_root.as_path(),
            cargo_config,
            load_config,
            &on_progress,
        )
        .with_context(|| {
            format!(
                "Failed to load workspace at {}",
                self.project_root.display()
            )
        })
        .map_err(|error| Error::Other(error.to_string()))?;
        finish_spinner(
            progress_ra_init.as_ref(),
            "\x1b[32m\u{2713}\x1b[0m Rust-analyzer initialized",
        );
        Ok((AnalysisHost::with_database(analysis_db), virtual_fs))
    }

    fn index_files(
        progress_group: Option<&MultiProgress>,
        virtual_fs: &Vfs,
    ) -> (FileIdMap, FileMetadata) {
        let progress_index =
            progress_group.map(|group| create_spinner(group, "Building file index..."));
        let (file_id_map, file_metadata) = build_file_index(virtual_fs);
        finish_spinner(
            progress_index.as_ref(),
            &format!(
                "\x1b[32m\u{2713}\x1b[0m Indexed {} Rust files",
                file_id_map.len()
            ),
        );
        tracing::info!("Loaded {} Rust files", file_id_map.len());
        (file_id_map, file_metadata)
    }

    fn maybe_save_cache(
        &self,
        progress_group: Option<&MultiProgress>,
        file_metadata: FileMetadata,
    ) {
        let progress_save = progress_group.map(|group| create_spinner(group, "Saving cache..."));
        if self.config.use_cache {
            let cache = WorkspaceCache::new(self.project_root.clone(), file_metadata);
            if let Err(error) = cache.save(&self.project_root) {
                tracing::warn!("Failed to save cache: {}", error);
            }
        }
        finish_spinner(
            progress_save.as_ref(),
            "\x1b[32m\u{2713}\x1b[0m Cache saved",
        );
    }
}

fn create_spinner(group: &MultiProgress, message: &str) -> ProgressBar {
    let spinner = group.add(ProgressBar::new_spinner());
    spinner.set_style(ProgressStyle::default_spinner());
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner
}

fn finish_spinner(progress: Option<&ProgressBar>, message: &str) {
    if let Some(progress_bar) = progress {
        progress_bar.finish_with_message(message.to_string());
    }
}

fn build_file_index(vfs: &Vfs) -> (FileIdMap, FileMetadata) {
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

    (file_id_map, file_metadata)
}
