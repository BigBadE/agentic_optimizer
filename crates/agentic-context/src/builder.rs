use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use agentic_core::{Context, FileContext, Query, Result};
use core::result::Result as CoreResult;

/// Default system prompt used when constructing the base context.
const DEFAULT_SYSTEM_PROMPT: &str = "You are a helpful coding assistant. You help users understand and modify their codebase.\n\nWhen making changes:\n1. Be precise and accurate\n2. Explain your reasoning\n3. Provide complete, working code\n4. Follow the existing code style\n\nYou have access to the user's codebase context below.";

/// Directories ignored during project scan.
const IGNORED_DIRS: &[&str] = &["target", "node_modules", "dist", "build", ".git", ".idea", ".vscode"];

/// Builds a `Context` by scanning files under a project root.
pub struct ContextBuilder {
    /// Root directory of the project to scan
    project_root: PathBuf,
    /// Maximum number of files to include in context
    max_files: usize,
    /// Maximum file size in bytes to include
    max_file_size: usize,
}

impl ContextBuilder {
    /// Create a new builder with defaults.
    #[must_use]
    #[allow(clippy::missing_const_for_fn, reason = "PathBuf construction in const fn is not desired here")]
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            max_files: 50,
            max_file_size: 100_000,
        }
    }

    /// Override the maximum number of files included in context.
    #[must_use]
    #[allow(clippy::missing_const_for_fn, reason = "builder-style API; const not necessary")]
    pub fn with_max_files(mut self, max_files: usize) -> Self {
        self.max_files = max_files;
        self
    }

    /// Build a `Context` for the provided query.
    ///
    /// # Errors
    /// Returns an error if file scanning or reading fails.
    pub fn build_context(&self, query: &Query) -> Result<Context> {
        let mut files = if query.files_context.is_empty() {
            self.collect_all_files()
        } else {
            let mut collected = Vec::new();
            for file_path in &query.files_context {
                if let Ok(file_context) = FileContext::from_path(file_path) {
                    collected.push(file_context);
                }
            }
            if collected.is_empty() {
                self.collect_all_files()
            } else {
                collected
            }
        };

        files.truncate(self.max_files);

        Ok(Context::new(DEFAULT_SYSTEM_PROMPT).with_files(files))
    }

    /// Collect a list of readable code files under the project root.
    fn collect_all_files(&self) -> Vec<FileContext> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.project_root)
            .into_iter()
            .filter_entry(|entry_var| !Self::is_ignored(entry_var))
            .filter_map(CoreResult::ok)
        {
            if entry.file_type().is_dir() {
                continue;
            }

            if !Self::is_code_file(entry.path()) {
                continue;
            }

            if let Ok(metadata) = entry.metadata()
                && metadata.len() > self.max_file_size as u64
            {
                continue;
            }

            if let Ok(file_context) = FileContext::from_path(&entry.path().to_path_buf()) {
                files.push(file_context);
            }

            if files.len() >= self.max_files {
                break;
            }
        }

        files
    }

    /// Determine whether a directory entry should be ignored.
    fn is_ignored(entry: &walkdir::DirEntry) -> bool {
        let file_name = entry.file_name().to_string_lossy();

        if file_name.starts_with('.') {
            return true;
        }

        if entry.file_type().is_dir() && IGNORED_DIRS.contains(&file_name.as_ref()) {
            return true;
        }

        false
    }

    /// Determine whether a path looks like a code/documentation file worth indexing.
    fn is_code_file(path: &Path) -> bool {
        path.extension().is_some_and(|extension| {
            let ext = extension.to_string_lossy();
            matches!(ext.as_ref(), "rs" | "toml" | "md" | "txt" | "json" | "yaml" | "yml")
        })
    }
}
