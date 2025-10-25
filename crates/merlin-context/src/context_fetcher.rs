use regex::Regex;
use std::collections::HashSet;
use std::env;
use std::fmt::Write as _;
use std::mem::replace;
use std::path::{Path, PathBuf};
use tokio::fs::read_to_string;
use tracing::{debug, info};

use crate::{ContextBuilder, ProgressCallback};
use merlin_core::{Context, FileContext, Query};
use merlin_core::{Result, RoutingError};
use merlin_languages::{Language, create_backend};

/// Extracts file references and builds contextual information for tasks
pub struct ContextFetcher {
    /// Root directory of the project
    project_root: PathBuf,
    /// Context builder for file scanning and analysis
    context_builder: Option<ContextBuilder>,
    /// Optional progress callback for embedding operations
    progress_callback: Option<ProgressCallback>,
}

impl ContextFetcher {
    /// Create a new context fetcher
    pub fn new(project_root: PathBuf) -> Self {
        // Check if we should skip expensive operations (for tests)
        let skip_embeddings = env::var("MERLIN_SKIP_EMBEDDINGS").is_ok();

        // Try to create a language backend (Rust for now) unless skipping
        let context_builder = if skip_embeddings {
            debug!("Skipping language backend initialization (MERLIN_SKIP_EMBEDDINGS set)");
            None
        } else {
            let mut builder = ContextBuilder::new(project_root.clone());

            if let Ok(backend) = create_backend(Language::Rust) {
                builder = builder.with_language_backend(backend);
                debug!("Language backend (Rust) initialized for context fetcher");
            } else {
                debug!("Failed to initialize language backend, will use vector search only");
            }

            Some(builder)
        };

        Self {
            project_root,
            context_builder,
            progress_callback: None,
        }
    }

    /// Get the project root path
    pub fn project_root(&self) -> &PathBuf {
        &self.project_root
    }

    /// Set a progress callback for embedding operations
    #[must_use]
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Extract file references from text
    ///
    /// Supports multiple formats:
    /// - Absolute paths: /path/to/file.rs
    /// - Relative paths: src/main.rs
    /// - File mentions: "in file.rs" or "file `config.toml`"
    /// - Module paths: `crate::module::function` (attempts to resolve to file)
    ///
    /// # Panics
    /// Panics if regex compilation fails (should never happen with valid patterns)
    fn extract_file_references(&self, text: &str) -> Vec<PathBuf> {
        let mut files = HashSet::new();

        // Pattern 1: Explicit file paths (with extension)
        let Ok(path_regex) = Regex::new(r"([a-zA-Z0-9_\-./]+\.[a-z]{1,4})") else {
            // Hardcoded regex pattern is guaranteed valid, but handle gracefully
            return Vec::new();
        };
        for cap in path_regex.captures_iter(text) {
            if let Some(matched) = cap.get(1) {
                let path_str = matched.as_str();
                let path = self.resolve_path(path_str);
                if path.exists() && path.is_file() {
                    files.insert(path);
                }
            }
        }

        // Pattern 2: Module paths (Rust-specific for now)
        let Ok(module_regex) = Regex::new(r"(?:crate|super|self)::([a-zA-Z0-9_:]+)") else {
            // Hardcoded regex pattern is guaranteed valid, but handle gracefully
            return files.into_iter().collect();
        };
        for cap in module_regex.captures_iter(text) {
            let Some(matched) = cap.get(1) else {
                continue;
            };
            let module_path = matched.as_str();
            let Some(file_path) = self.resolve_module_path(module_path) else {
                continue;
            };
            if file_path.exists() && file_path.is_file() {
                files.insert(file_path);
            }
        }

        files.into_iter().collect()
    }

    /// Resolve a path string to an absolute path
    fn resolve_path(&self, path_str: &str) -> PathBuf {
        let path = Path::new(path_str);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        }
    }

    /// Attempt to resolve a Rust module path to a file path
    ///
    /// e.g., `crate::module::submodule` -> `src/module/submodule.rs` or `src/module/submodule/mod.rs`
    fn resolve_module_path(&self, module_path: &str) -> Option<PathBuf> {
        let parts: Vec<&str> = module_path.split("::").collect();
        if parts.is_empty() {
            return None;
        }

        // Try src/module/submodule.rs
        let mut file_path = self.project_root.join("src");
        for part in &parts {
            file_path = file_path.join(part);
        }
        file_path.set_extension("rs");
        if file_path.exists() {
            return Some(file_path);
        }

        // Try src/module/submodule/mod.rs
        let mut mod_path = self.project_root.join("src");
        for part in &parts {
            mod_path = mod_path.join(part);
        }
        mod_path = mod_path.join("mod.rs");
        if mod_path.exists() {
            return Some(mod_path);
        }

        None
    }

    /// Build comprehensive context for a query, including:
    /// - Extracted file references
    /// - Vector/semantic search results
    /// - Relevant project files
    ///
    /// # Errors
    /// Returns an error if context building fails
    pub async fn build_context_for_query(&mut self, query: &Query) -> Result<Context> {
        info!("Building context for query: {}", query.text);

        // Extract explicitly mentioned files
        let explicit_files = self.extract_file_references(&query.text);
        debug!(
            "Extracted {} explicit file references",
            explicit_files.len()
        );

        // Use context builder if available
        if let Some(builder) = &mut self.context_builder {
            if let Some(callback) = self.progress_callback.clone() {
                *builder = replace(builder, ContextBuilder::new(self.project_root.clone()))
                    .with_progress_callback(callback);
            }

            let context = builder
                .build_context(query)
                .await
                .map_err(|err| RoutingError::Other(format!("Context building failed: {err}")))?;

            info!("Built context with {} files", context.files.len());
            return Ok(context);
        }

        // Fallback: create basic context with explicit files only
        let mut context = Context::new(&query.text);
        for file_path in explicit_files {
            if let Ok(content) = read_to_string(&file_path).await {
                let file_context = FileContext {
                    path: file_path.clone(),
                    content,
                };
                context.files.push(file_context);
            }
        }

        Ok(context)
    }

    /// Build context from conversation history
    ///
    /// Extracts file references from all messages and builds comprehensive context
    ///
    /// # Errors
    /// Returns an error if context building fails
    pub async fn build_context_from_conversation(
        &mut self,
        messages: &[(String, String)], // (role, content) pairs
        current_query: &Query,
    ) -> Result<Context> {
        info!(
            "Building context from conversation with {} messages",
            messages.len()
        );

        // Extract files from all messages
        let mut all_files = HashSet::new();
        for (_, content) in messages {
            for file in self.extract_file_references(content) {
                all_files.insert(file);
            }
        }
        for file in self.extract_file_references(&current_query.text) {
            all_files.insert(file);
        }

        debug!(
            "Extracted {} total file references from conversation",
            all_files.len()
        );

        // Build context using the context builder
        let mut context = self.build_context_for_query(current_query).await?;

        // Add conversation history to system prompt (newest last) only if not empty
        if !messages.is_empty() {
            let mut conversation_text =
                String::from("\n\n=== Previous Conversation (newest at bottom) ===\n");
            for (role, content) in messages {
                let _write_result = write!(conversation_text, "{role}: {content}\n\n");
            }
            conversation_text.push_str("=== End Previous Conversation ===\n\n");
            context.system_prompt.push_str(&conversation_text);
        }

        Ok(context)
    }
}
