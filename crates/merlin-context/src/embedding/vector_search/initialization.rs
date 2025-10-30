//! Vector search manager initialization logic.

use merlin_deps::tracing::info;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::time::SystemTime;

use crate::embedding::vector_search::cache::{CachedEmbedding, VectorCache};
use crate::embedding::{BM25Index, VectorStore};
use crate::fs_utils::is_source_file;

/// Initialization helper
pub struct InitializationHelper;

impl InitializationHelper {
    /// Resolve cache path with environment override support
    ///
    /// Env variables:
    /// - `MERLIN_FOLDER`: directory for the entire Merlin state (e.g. `.merlin`). We store embeddings at `{MERLIN_FOLDER}/cache/vector/embeddings.bin`
    pub fn resolve_cache_path(project_root: &Path) -> PathBuf {
        if let Ok(folder) = env::var("MERLIN_FOLDER") {
            let path = PathBuf::from(folder)
                .join("cache")
                .join("vector")
                .join("embeddings.bin");
            info!("Using MERLIN_FOLDER: {}", path.display());
            return path;
        }

        project_root
            .join(".merlin")
            .join("cache")
            .join("vector")
            .join("embeddings.bin")
    }

    /// Collect all source files in the project
    pub fn collect_source_files(project_root: &Path) -> Vec<PathBuf> {
        use merlin_deps::ignore::WalkBuilder;

        let mut files = Vec::default();

        let walker = WalkBuilder::new(project_root)
            .max_depth(None)
            .hidden(true)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(false)
            .build();

        for entry in walker.filter_map(StdResult::ok) {
            let path = entry.path();

            if entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
                && is_source_file(path)
            {
                let normalized_path = path
                    .strip_prefix(project_root)
                    .map_or_else(|_| path.to_path_buf(), PathBuf::from);
                files.push(normalized_path);
            }
        }

        files
    }

    /// Load valid cache entries into the store
    pub fn load_valid_entries(
        valid: &[CachedEmbedding],
        store: &mut VectorStore,
        bm25: &mut BM25Index,
        file_times: &mut HashMap<PathBuf, SystemTime>,
        file_hashes: &mut HashMap<PathBuf, u64>,
    ) {
        for entry in valid {
            let chunk_path = format!(
                "{}:{}-{}",
                entry.path.display(),
                entry.start_line,
                entry.end_line
            );
            file_times.insert(entry.path.clone(), entry.modified);
            file_hashes.insert(entry.path.clone(), entry.content_hash);
            store.add(
                PathBuf::from(&chunk_path),
                entry.embedding.clone(),
                entry.preview.clone(),
            );

            // Rebuild BM25 index from preview (approximation)
            bm25.add_document(PathBuf::from(chunk_path), &entry.preview);
        }
    }

    /// Identify new files that need embedding
    pub fn identify_new_files(cache: &VectorCache, project_root: &Path) -> (Vec<PathBuf>, usize) {
        let all_files = Self::collect_source_files(project_root);
        let cached_paths: HashSet<_> = cache.embeddings.iter().map(|entry| &entry.path).collect();
        let new_files: Vec<_> = all_files
            .into_iter()
            .filter(|f| !cached_paths.contains(f))
            .collect();
        let new_count = new_files.len();
        (new_files, new_count)
    }
}
