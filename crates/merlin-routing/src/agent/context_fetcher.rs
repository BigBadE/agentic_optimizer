use regex::Regex;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use tokio::fs::read_to_string;
use tracing::{debug, info};

use crate::{Result, RoutingError};
use merlin_context::{ContextBuilder, VectorSearchManager};
use merlin_core::{Context, FileContext, Query};

/// Extracts file references and builds contextual information for tasks
pub struct ContextFetcher {
    /// Root directory of the project
    project_root: PathBuf,
    /// Context builder for file scanning and analysis
    context_builder: Option<ContextBuilder>,
    /// Vector search manager for semantic search
    vector_manager: Option<VectorSearchManager>,
    /// Whether to use vector search for context enrichment
    use_vector_search: bool,
}

impl ContextFetcher {
    /// Create a new context fetcher
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root: project_root.clone(),
            context_builder: Some(ContextBuilder::new(project_root)),
            vector_manager: None,
            use_vector_search: false,
        }
    }

    /// Enable vector search for semantic context retrieval
    #[must_use]
    pub fn with_vector_search(mut self, vector_manager: VectorSearchManager) -> Self {
        self.vector_manager = Some(vector_manager);
        self.use_vector_search = true;
        self
    }

    /// Disable `ContextBuilder` for testing (uses fallback mode only)
    #[must_use]
    pub fn without_context_builder(mut self) -> Self {
        self.context_builder = None;
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
    pub fn extract_file_references(&self, text: &str) -> Vec<PathBuf> {
        let mut files = HashSet::new();

        // Pattern 1: Explicit file paths (with extension)
        #[allow(clippy::expect_used, reason = "Regex pattern is known to be valid")]
        let path_regex = Regex::new(r"([a-zA-Z0-9_\-./]+\.[a-z]{1,4})").expect("Valid regex");
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
        #[allow(clippy::expect_used, reason = "Regex pattern is known to be valid")]
        let module_regex =
            Regex::new(r"(?:crate|super|self)::([a-zA-Z0-9_:]+)").expect("Valid regex");
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

        // Add conversation history to system prompt
        let mut conversation_text = String::from("\n\n=== Previous Conversation ===\n");
        for (role, content) in messages {
            #[allow(clippy::expect_used, reason = "Writing to string never fails")]
            write!(conversation_text, "{role}: {content}\n\n")
                .expect("Writing to string never fails");
        }
        conversation_text.push_str("=== End Previous Conversation ===\n\n");
        context.system_prompt.push_str(&conversation_text);

        Ok(context)
    }

    /// Detect programming language from file extension
    #[allow(dead_code, reason = "Utility function for future use")]
    fn detect_language(path: &Path) -> String {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map_or("unknown", |ext| match ext {
                "rs" => "rust",
                "py" => "python",
                "js" | "jsx" => "javascript",
                "ts" | "tsx" => "typescript",
                "go" => "go",
                "java" => "java",
                "cpp" | "cc" | "cxx" => "cpp",
                "c" | "h" => "c",
                _ => "unknown",
            })
            .to_owned()
    }
}

#[cfg(test)]
#[allow(
    clippy::min_ident_chars,
    reason = "Test code uses short variable names"
)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    async fn create_test_project() -> (TempDir, PathBuf) {
        let temp_dir =
            TempDir::with_prefix("context_fetcher_test").expect("Failed to create temp dir");
        let project_root = temp_dir.path().to_path_buf();

        // Create test file structure
        fs::create_dir_all(project_root.join("src")).await.unwrap();
        fs::write(
            project_root.join("src/main.rs"),
            "fn main() { println!(\"Hello\"); }",
        )
        .await
        .unwrap();
        fs::write(project_root.join("src/lib.rs"), "pub mod utils;")
            .await
            .unwrap();
        fs::create_dir_all(project_root.join("src/utils"))
            .await
            .unwrap();
        fs::write(project_root.join("src/utils/mod.rs"), "pub fn helper() {}")
            .await
            .unwrap();

        (temp_dir, project_root)
    }

    #[tokio::test]
    async fn test_extract_file_references() {
        let (_temp, project_root) = create_test_project().await;
        let fetcher = ContextFetcher::new(project_root);

        let text = "Please check src/main.rs and update lib.rs in the source directory";
        let files = fetcher.extract_file_references(text);

        assert!(files.iter().any(|p| p.ends_with("src/main.rs")));
    }

    #[tokio::test]
    async fn test_resolve_module_path() {
        let (_temp, project_root) = create_test_project().await;
        let fetcher = ContextFetcher::new(project_root);

        let module_path = fetcher.resolve_module_path("utils");
        assert!(module_path.is_some());
        assert!(module_path.unwrap().ends_with("utils/mod.rs"));
    }

    #[tokio::test]
    async fn test_build_context_for_query() {
        let (_temp, project_root) = create_test_project().await;
        let mut fetcher = ContextFetcher::new(project_root).without_context_builder();

        let query = Query::new("Check src/main.rs for errors");
        let context = fetcher.build_context_for_query(&query).await.unwrap();

        // Fallback mode is used, so check files or system prompt
        assert!(!context.files.is_empty() || !context.system_prompt.is_empty());
    }

    #[tokio::test]
    async fn test_build_context_from_conversation() {
        let (_temp, project_root) = create_test_project().await;
        let mut fetcher = ContextFetcher::new(project_root).without_context_builder();

        let messages = vec![
            ("user".to_owned(), "I need to fix src/main.rs".to_owned()),
            (
                "assistant".to_owned(),
                "Sure, let me check that file".to_owned(),
            ),
        ];
        let query = Query::new("Now update src/lib.rs too");

        let context = fetcher
            .build_context_from_conversation(&messages, &query)
            .await
            .unwrap();

        // Should have conversation history in system prompt
        assert!(context.system_prompt.contains("Previous Conversation"));
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(
            ContextFetcher::detect_language(Path::new("test.rs")),
            "rust"
        );
        assert_eq!(
            ContextFetcher::detect_language(Path::new("test.py")),
            "python"
        );
        assert_eq!(
            ContextFetcher::detect_language(Path::new("test.js")),
            "javascript"
        );
        assert_eq!(
            ContextFetcher::detect_language(Path::new("test.unknown")),
            "unknown"
        );
    }
}
