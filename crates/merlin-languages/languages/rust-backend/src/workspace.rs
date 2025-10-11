//! Workspace loading utilities using rust-analyzer's cargo loader.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Context as _;
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
    pub fn new(project_root: &Path) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            config: LoadConfig::default(),
        }
    }

    /// Create a new workspace loader with custom configuration.
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
        self.report_cache_status();
        self.log_loading_path();

        let cargo_config = Self::build_cargo_config();
        let load_config = Self::build_load_config();
        let (host, virtual_fs) = self.initialize_rust_analyzer(&cargo_config, &load_config)?;

        let (file_id_map, file_metadata) = Self::index_files(&virtual_fs);
        self.maybe_save_cache(file_metadata);

        Ok((host, virtual_fs, file_id_map))
    }

    fn report_cache_status(&self) {
        tracing::debug!("Checking cache...");
        if self.config.use_cache
            && let Ok(cache) = WorkspaceCache::load(&self.project_root)
            && cache.is_valid(&self.project_root).unwrap_or(false)
        {
            tracing::info!(
                "Using cached workspace state (timestamp: {:?})",
                cache.timestamp
            );
        }
        tracing::debug!("Cache checked");
    }

    fn log_loading_path(&self) {
        tracing::info!(
            "Loading Cargo workspace from: {}",
            self.project_root.display()
        );
    }

    fn build_cargo_config() -> CargoConfig {
        CargoConfig {
            sysroot: Some(RustLibSource::Discover),
            ..CargoConfig::default()
        }
    }

    fn build_load_config() -> LoadCargoConfig {
        LoadCargoConfig {
            load_out_dirs_from_check: true,
            with_proc_macro_server: ProcMacroServerChoice::Sysroot,
            prefill_caches: false,
        }
    }

    /// Initialize rust-analyzer with the given configuration
    ///
    /// # Errors
    /// Returns an error when the cargo workspace cannot be loaded or transformed into an analysis database.
    fn initialize_rust_analyzer(
        &self,
        cargo_config: &CargoConfig,
        load_config: &LoadCargoConfig,
    ) -> Result<(AnalysisHost, Vfs)> {
        tracing::info!("Initializing rust-analyzer...");
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
        tracing::info!("Rust-analyzer initialized");
        Ok((AnalysisHost::with_database(analysis_db), virtual_fs))
    }

    fn index_files(virtual_fs: &Vfs) -> (FileIdMap, FileMetadata) {
        tracing::debug!("Building file index...");
        let (file_id_map, file_metadata) = build_file_index(virtual_fs);
        tracing::info!("Indexed {} Rust files", file_id_map.len());
        (file_id_map, file_metadata)
    }

    fn maybe_save_cache(&self, file_metadata: FileMetadata) {
        if self.config.use_cache {
            tracing::debug!("Saving cache...");
            let cache = WorkspaceCache::new(self.project_root.clone(), file_metadata);
            if let Err(error) = cache.save(&self.project_root) {
                tracing::warn!("Failed to save workspace cache: {}", error);
            } else {
                tracing::debug!("Cache saved");
            }
        }
    }
}

fn build_file_index(vfs: &Vfs) -> (FileIdMap, FileMetadata) {
    let mut file_id_map = HashMap::default();
    let mut file_metadata = FileMetadata::default();

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
