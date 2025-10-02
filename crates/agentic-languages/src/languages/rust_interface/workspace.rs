//! Workspace loading utilities using rust-analyzer's cargo loader.

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use anyhow::Context as _;
use ra_ap_ide::AnalysisHost;
use ra_ap_load_cargo::{LoadCargoConfig, load_workspace_at};
use ra_ap_project_model::{CargoConfig, RustLibSource};
use ra_ap_vfs::Vfs;

use agentic_core::Result;

/// Helper to load a Cargo workspace into rust-analyzer structures.
pub struct WorkspaceLoader {
    /// Root directory of the Cargo workspace
    project_root: PathBuf,
}

impl WorkspaceLoader {
    /// Create a new workspace loader.
    #[must_use]
    pub fn new(project_root: &Path) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
        }
    }

    /// Load the workspace, returning analysis host, virtual filesystem, and a file id map.
    ///
    /// # Errors
    /// Returns an error if the workspace cannot be loaded.
    pub fn load(&self) -> Result<(AnalysisHost, Vfs, HashMap<PathBuf, ra_ap_ide::FileId>)> {
        tracing::info!("Loading Cargo workspace from: {}", self.project_root.display());

        let cargo_config = CargoConfig {
            sysroot: Some(RustLibSource::Discover),
            ..Default::default()
        };

        let load_config = LoadCargoConfig {
            load_out_dirs_from_check: true,
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
        .map_err(|error| agentic_core::Error::Other(error.to_string()))?;

        let host = AnalysisHost::with_database(db);
        
        let mut file_id_map = HashMap::new();
        
        for (file_id, path) in vfs.iter() {
            if let Some(abs_path) = path.as_path() {
                let path_buf: PathBuf = abs_path.to_path_buf().into();
                if path_buf.to_string_lossy().ends_with(".rs") {
                    file_id_map.insert(path_buf, file_id);
                }
            }
        }

        tracing::info!("Loaded {} Rust files", file_id_map.len());

        Ok((host, vfs, file_id_map))
    }
}
