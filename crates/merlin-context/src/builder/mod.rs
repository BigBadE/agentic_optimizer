//! Context builder module for assembling LLM contexts from project files.

mod chunk_processor;
mod file_scanner;
mod search;
mod system_init;

use std::path::PathBuf;

use merlin_core::{Context, CoreResult as Result, FileContext, Query};
use merlin_languages::LanguageProvider;

use crate::embedding::{ProgressCallback, VectorSearchManager};
use crate::query::{QueryAnalyzer, QueryIntent};

/// Builds a `Context` by scanning files under a project root.
pub struct ContextBuilder {
    /// Root directory of the project to scan
    project_root: PathBuf,
    /// Maximum number of files to include in context
    max_files: usize,
    /// Maximum file size in bytes to include
    max_file_size: usize,
    /// Optional language backend for semantic analysis
    language_backend: Option<Box<dyn LanguageProvider>>,
    /// Whether the language backend has been initialized
    language_backend_initialized: bool,
    /// Vector search manager for semantic search
    vector_manager: Option<VectorSearchManager>,
    /// Optional progress callback for embedding operations
    progress_callback: Option<ProgressCallback>,
}

impl ContextBuilder {
    /// Create a new builder with defaults.
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            max_files: 50,
            max_file_size: 100_000,
            language_backend: None,
            language_backend_initialized: false,
            vector_manager: None,
            progress_callback: None,
        }
    }

    /// Override the maximum number of files included in context.
    #[must_use]
    pub fn with_max_files(mut self, max_files: usize) -> Self {
        self.max_files = max_files;
        self
    }

    /// Enable a language backend for semantic analysis.
    ///
    /// This accepts any implementation of the `LanguageProvider` trait,
    /// allowing support for multiple languages (Rust, Java, Python, etc.)
    #[must_use]
    pub fn with_language_backend(mut self, backend: Box<dyn LanguageProvider>) -> Self {
        self.language_backend = Some(backend);
        self
    }

    /// Set a progress callback for embedding operations
    #[must_use]
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Build a `Context` for the provided query.
    ///
    /// # Errors
    /// Returns an error if file scanning or reading fails.
    pub async fn build_context(&mut self, query: &Query) -> Result<Context> {
        // Step 1: Analyze the query to extract intent
        let analyzer = QueryAnalyzer;
        let intent = analyzer.analyze(&query.text);

        merlin_deps::tracing::info!(
            "Query intent: action={:?}, scope={:?}, complexity={:?}",
            intent.action,
            intent.scope,
            intent.complexity
        );
        merlin_deps::tracing::debug!(
            "Keywords: {:?}, Entities: {:?}",
            intent.keywords,
            intent.entities
        );

        let mut files = if query.files_context.is_empty() {
            // Step 2: Initialize backend and vector search IN PARALLEL
            self.initialize_systems_parallel().await?;

            // Step 3: Use hybrid search for context (vector search works without backend)
            let agent_files = self.use_subagent_for_context(&intent, &query.text).await?;
            merlin_deps::tracing::info!(
                "Intelligent context fetching found {} files",
                agent_files.len()
            );
            agent_files
        } else {
            // User provided specific files
            let mut collected = Vec::new();
            for file_path in &query.files_context {
                if let Ok(file_context) = FileContext::from_path(file_path) {
                    collected.push(file_context);
                }
            }
            if collected.is_empty() {
                let all_files = self.collect_all_files();
                merlin_deps::tracing::info!(
                    "Collected {} files from project scan",
                    all_files.len()
                );
                all_files
            } else {
                collected
            }
        };

        files.truncate(self.max_files);
        merlin_deps::tracing::info!(
            "Final context: {} files (max: {})",
            files.len(),
            self.max_files
        );

        Ok(Context::new(String::new()).with_files(files))
    }

    /// Use hybrid search to intelligently gather context
    ///
    /// # Errors
    /// Returns an error if hybrid search fails
    async fn use_subagent_for_context(
        &self,
        intent: &QueryIntent,
        query_text: &str,
    ) -> Result<Vec<FileContext>> {
        search::use_subagent_for_context(
            self.vector_manager.as_ref(),
            &self.project_root,
            intent,
            query_text,
        )
        .await
    }

    /// Collect a list of readable code files under the project root.
    fn collect_all_files(&self) -> Vec<FileContext> {
        file_scanner::collect_all_files(&self.project_root, self.max_files, self.max_file_size)
    }

    /// Initializes systems (language backend and vector search) in parallel.
    ///
    /// # Errors
    /// Returns an error if critical initialization fails.
    async fn initialize_systems_parallel(&mut self) -> Result<()> {
        system_init::initialize_systems_parallel(
            &mut self.language_backend,
            &mut self.language_backend_initialized,
            &mut self.vector_manager,
            self.project_root.as_path(),
            self.progress_callback.as_ref(),
        )
        .await
    }
}
