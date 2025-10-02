use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use agentic_core::{Context, FileContext, Query, Result};

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a helpful coding assistant. You help users understand and modify their codebase.

When making changes:
1. Be precise and accurate
2. Explain your reasoning
3. Provide complete, working code
4. Follow the existing code style

You have access to the user's codebase context below."#;

pub struct ContextBuilder {
    project_root: PathBuf,
    max_files: usize,
    max_file_size: usize,
}

impl ContextBuilder {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            max_files: 50,
            max_file_size: 100_000,
        }
    }

    pub fn with_max_files(mut self, max_files: usize) -> Self {
        self.max_files = max_files;
        self
    }

    pub fn build_context(&self, query: &Query) -> Result<Context> {
        let mut files = Vec::new();

        if query.files_context.is_empty() {
            files = self.collect_all_files()?;
        } else {
            for file_path in &query.files_context {
                if let Ok(file_context) = FileContext::from_path(file_path) {
                    files.push(file_context);
                }
            }

            if files.is_empty() {
                files = self.collect_all_files()?;
            }
        }

        files.truncate(self.max_files);

        Ok(Context::new(DEFAULT_SYSTEM_PROMPT).with_files(files))
    }

    fn collect_all_files(&self) -> Result<Vec<FileContext>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.project_root)
            .into_iter()
            .filter_entry(|e| !self.is_ignored(e))
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            if !self.is_code_file(entry.path()) {
                continue;
            }

            if let Ok(metadata) = entry.metadata() {
                if metadata.len() > self.max_file_size as u64 {
                    continue;
                }
            }

            if let Ok(file_context) = FileContext::from_path(&entry.path().to_path_buf()) {
                files.push(file_context);
            }

            if files.len() >= self.max_files {
                break;
            }
        }

        Ok(files)
    }

    fn is_ignored(&self, entry: &walkdir::DirEntry) -> bool {
        let file_name = entry.file_name().to_string_lossy();

        if file_name.starts_with('.') {
            return true;
        }

        const IGNORED_DIRS: &[&str] = &[
            "target",
            "node_modules",
            "dist",
            "build",
            ".git",
            ".idea",
            ".vscode",
        ];

        if entry.file_type().is_dir() && IGNORED_DIRS.contains(&file_name.as_ref()) {
            return true;
        }

        false
    }

    fn is_code_file(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy();
            matches!(
                ext.as_ref(),
                "rs" | "toml" | "md" | "txt" | "json" | "yaml" | "yml"
            )
        } else {
            false
        }
    }
}
