use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use agentic_core::{Context, FileContext, Query, Result};
use agentic_languages::LanguageProvider;
use core::result::Result as CoreResult;

use crate::query::{QueryAnalyzer, QueryIntent};
use crate::subagent::LocalContextAgent;
use crate::expander::ContextExpander;

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
    /// Optional language backend for semantic analysis
    language_backend: Option<Box<dyn LanguageProvider>>,
    /// Whether the language backend has been initialized
    language_backend_initialized: bool,
}

impl ContextBuilder {
    /// Create a new builder with defaults.
    #[must_use]
    pub const fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            max_files: 50,
            max_file_size: 100_000,
            language_backend: None,
            language_backend_initialized: false,
        }
    }

    /// Override the maximum number of files included in context.
    #[must_use]
    pub const fn with_max_files(mut self, max_files: usize) -> Self {
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

    /// Build a `Context` for the provided query.
    ///
    /// # Errors
    /// Returns an error if file scanning or reading fails.
    pub async fn build_context(&mut self, query: &Query) -> Result<Context> {
        // Step 1: Analyze the query to extract intent
        let analyzer = QueryAnalyzer::new();
        let intent = analyzer.analyze(&query.text);
        
        tracing::info!("Query intent: action={:?}, scope={:?}, complexity={:?}", 
            intent.action, intent.scope, intent.complexity);
        tracing::debug!("Keywords: {:?}, Entities: {:?}", intent.keywords, intent.entities);

        let mut files = if query.files_context.is_empty() {
            // Step 2: Initialize backend if available
            if self.language_backend.is_some() && !self.language_backend_initialized {
                if let Some(backend) = &mut self.language_backend {
                    eprintln!("⚙️  Initializing rust-analyzer (this may take a moment)...");
                    tracing::info!("Initializing language backend...");
                    match backend.initialize(&self.project_root) {
                        Ok(()) => {
                            self.language_backend_initialized = true;
                            eprintln!("✓ Rust-analyzer initialized successfully");
                            tracing::info!("Language backend initialized successfully");
                        }
                        Err(e) => {
                            eprintln!("⚠️  Warning: Failed to initialize rust-analyzer: {}", e);
                            eprintln!("   Falling back to basic file scanning");
                            tracing::warn!("Failed to initialize language backend: {}", e);
                        }
                    }
                }
            }

            // Step 3: Use subagent to generate context plan (backend is required)
            if self.language_backend.is_some() && self.language_backend_initialized {
                let agent_files = self.use_subagent_for_context(&intent, &query.text).await?;
                eprintln!("✓ Intelligent context fetching found {} files", agent_files.len());
                tracing::info!("Subagent found {} files", agent_files.len());
                agent_files
            } else {
                return Err(agentic_core::Error::Other(
                    "Language backend not initialized. This should not happen.".into()
                ));
            }
        } else {
            // User provided specific files
            let mut collected = Vec::new();
            for file_path in &query.files_context {
                if let Ok(file_context) = FileContext::from_path(file_path) {
                    collected.push(file_context);
                }
            }
            if collected.is_empty() {
                let collected = self.collect_all_files();
                tracing::info!("Collected {} files from project scan", collected.len());
                collected
            } else {
                collected
            }
        };

        files.truncate(self.max_files);
        tracing::info!("Final context: {} files (max: {})", files.len(), self.max_files);

        Ok(Context::new(DEFAULT_SYSTEM_PROMPT).with_files(files))
    }

    /// Use the subagent to intelligently gather context
    async fn use_subagent_for_context(&self, intent: &QueryIntent, query_text: &str) -> Result<Vec<FileContext>> {
        // Initialize the language backend if needed
        if !self.language_backend_initialized {
            return Err(agentic_core::Error::Other("Language backend not initialized".into()));
        }

        // Create and check subagent availability
        let agent = LocalContextAgent::new();
        
        if !agent.is_available().await? {
            return Err(agentic_core::Error::Other(
                "Ollama not available. Please ensure Ollama is running with: ollama serve".into()
            ));
        }

        tracing::info!("Using LocalContextAgent (Ollama) to generate context plan");

        // Generate context plan
        let plan = agent.generate_plan(intent, query_text).await?;
        
        tracing::info!("Context plan generated: {}", plan.reasoning);
        tracing::debug!("Plan details: keywords={:?}, symbols={:?}, patterns={:?}, depth={}", 
            plan.keywords, plan.symbols_to_find, plan.file_patterns, plan.max_depth);

        // Use expander to execute the plan
        let expander = ContextExpander::new(
            self.language_backend.as_ref(),
            &self.project_root,
            self.max_file_size,
        );

        expander.expand(&plan)
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
