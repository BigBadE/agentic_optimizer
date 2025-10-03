use std::path::{Path, PathBuf};
use std::collections::HashMap;
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle};
use ignore::WalkBuilder;

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
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        spinner.set_message("Checking Ollama availability...");
        let agent = LocalContextAgent::new();
        
        if !agent.is_available().await? {
            spinner.finish_and_clear();
            return Err(agentic_core::Error::Other(
                "Ollama not available. Please ensure Ollama is running with: ollama serve".into()
            ));
        }

        spinner.set_message("Generating context plan with AI agent...");
        tracing::info!("Using LocalContextAgent (Ollama) to generate context plan");

        // Build lightweight file tree for context
        let file_tree = self.build_file_tree();

        // Generate context plan
        let plan = agent.generate_plan(intent, query_text, &file_tree).await?;
        
        spinner.finish_with_message(format!("✓ Context plan: {}", plan.reasoning));
        eprintln!("  Keywords: {:?}", plan.keywords);
        eprintln!("  Symbols: {:?}", plan.symbols_to_find);
        eprintln!("  Patterns: {:?}", plan.file_patterns);
        
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

        // Don't filter the root directory itself (depth 0)
        if entry.depth() == 0 {
            return false;
        }

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

    /// Build a lightweight file tree for context planning
    fn build_file_tree(&self) -> String {
        let mut entries: Vec<(String, bool)> = Vec::new();
        let mut filtered_count = 0;
        let mut dir_entry_counts: HashMap<String, usize> = HashMap::new();
        const MAX_ENTRIES: usize = 1000;
        const MAX_PER_DIR: usize = 10;

        eprintln!("Building file tree from: {}", self.project_root.display());
        
        // Use ignore crate's WalkBuilder which properly handles .gitignore
        let walker = WalkBuilder::new(&self.project_root)
            .max_depth(Some(4))
            .hidden(true)  // Skip hidden files/dirs automatically
            .git_ignore(true)  // Respect .gitignore
            .git_global(false)  // Don't use global gitignore
            .git_exclude(false)  // Don't use .git/info/exclude
            .filter_entry(|entry| {
                // Filter out hardcoded ignored directories
                if let Some(file_name) = entry.path().file_name() {
                    let name = file_name.to_string_lossy();
                    if entry.file_type().map_or(false, |ft| ft.is_dir()) 
                        && IGNORED_DIRS.contains(&name.as_ref()) {
                        return false;
                    }
                }
                true
            })
            .build();

        for entry_result in walker {
            match entry_result {
                Ok(entry) => {
                    if entries.len() >= MAX_ENTRIES {
                        break;
                    }

                    let path = entry.path();

                    if let Ok(rel_path) = path.strip_prefix(&self.project_root) {
                        let path_str = rel_path.to_string_lossy().replace('\\', "/");
                        
                        // Skip empty path (root directory itself)
                        if path_str.is_empty() {
                            continue;
                        }
                        
                        let is_dir = entry.file_type().map_or(false, |ft| ft.is_dir());
                        
                        // Only include source files, not all files
                        if !is_dir && !Self::is_code_file(path) {
                            filtered_count += 1;
                            continue;
                        }
                        
                        // Get parent directory for per-directory limiting
                        let parent_dir = if let Some(parent) = Path::new(&path_str).parent() {
                            parent.to_string_lossy().to_string()
                        } else {
                            String::from(".")
                        };
                        
                        // Check per-directory limit
                        let dir_count = dir_entry_counts.entry(parent_dir.clone()).or_insert(0);
                        if *dir_count >= MAX_PER_DIR {
                            filtered_count += 1;
                            continue;
                        }
                        *dir_count += 1;
                        
                        entries.push((path_str, is_dir));
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Error walking directory: {}", e);
                }
            }
        }

        // Filter out empty directories (directories with no files under them)
        let mut non_empty_dirs = std::collections::HashSet::new();
        
        // Mark all parent directories of files as non-empty
        for (path, is_dir) in &entries {
            if !is_dir {
                // Mark all parent directories as non-empty
                let mut current = Path::new(path);
                while let Some(parent) = current.parent() {
                    let parent_str = parent.to_string_lossy().replace('\\', "/");
                    if !parent_str.is_empty() {
                        non_empty_dirs.insert(parent_str);
                    }
                    current = parent;
                }
            }
        }
        
        // Filter entries to only include files and non-empty directories
        let filtered_entries: Vec<_> = entries.into_iter()
            .filter(|(path, is_dir)| {
                if *is_dir {
                    non_empty_dirs.contains(path)
                } else {
                    true
                }
            })
            .collect();
        
        let dir_count = filtered_entries.iter().filter(|(_, is_dir)| *is_dir).count();
        let file_count = filtered_entries.len() - dir_count;
        
        eprintln!("File tree stats: {} dirs, {} files, {} total entries, {} filtered", 
            dir_count, file_count, filtered_entries.len(), filtered_count);

        // Build hierarchical tree structure
        let mut tree = String::from("Project Structure:\n");
        
        for (path, is_dir) in &filtered_entries {
            let depth = path.matches('/').count();
            let indent = "    ".repeat(depth);
            let name = Path::new(path).file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            
            if *is_dir {
                tree.push_str(&format!("{}{}/\n", indent, name));
            } else {
                tree.push_str(&format!("{}{}\n", indent, name));
            }
        }

        if filtered_entries.len() >= MAX_ENTRIES {
            tree.push_str(&format!("\n(Truncated at {} entries)\n", MAX_ENTRIES));
        }

        tree
    }
}
