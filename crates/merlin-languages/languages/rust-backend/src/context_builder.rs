//! Build semantic context for a given Rust file using rust-analyzer.

use std::collections::HashSet;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use ra_ap_ide::Analysis;
use ra_ap_syntax::{ast, AstNode as _, SyntaxKind};

use merlin_core::{FileContext, Result};
use crate::RustBackend;

/// Helper to compute related files and imports for a Rust source file.
pub struct ContextBuilder<'analysis> {
    /// The active rust-analyzer analysis snapshot
    analysis: &'analysis Analysis,
    /// The Rust backend used for file/id resolution
    backend: &'analysis RustBackend,
}

impl<'analysis> ContextBuilder<'analysis> {
    /// Create a new context builder bound to an analysis session and backend.
    #[must_use]
    pub fn new(analysis: &'analysis Analysis, backend: &'analysis RustBackend) -> Self {
        Self { analysis, backend }
    }

    /// Compute a minimal set of related file contexts for a given file.
    ///
    /// This includes the file itself and any directly imported modules that
    /// can be resolved heuristically to paths.
    ///
    /// # Errors
    /// Returns an error if rust-analyzer parsing fails for the file.
    pub fn get_related_context(&self, file: &Path) -> Result<Vec<FileContext>> {
        let mut related_files = HashSet::new();
        related_files.insert(file.to_path_buf());

        let imports = self.extract_imports(file)?;
        related_files.extend(imports);

        let mut contexts = Vec::new();
        for path in related_files {
            if let Ok(content) = read_to_string(&path) {
                contexts.push(FileContext::new(path, content));
            }
        }

        Ok(contexts)
    }

    /// Extract import paths (use trees) from a file and resolve to file paths where possible.
    ///
    /// # Errors
    /// Returns an error if rust-analyzer cannot parse the file.
    pub fn extract_imports(&self, file: &Path) -> Result<Vec<PathBuf>> {
        let file_id = self.backend.get_file_id(file)
            .ok_or_else(|| merlin_core::Error::FileNotFound(file.display().to_string()))?;

        let parsed = self
            .analysis
            .parse(file_id)
            .map_err(|error| merlin_core::Error::Other(error.to_string()))?;

        let syntax = parsed.syntax();
        let mut imports = Vec::new();

        for node in syntax.descendants() {
            if node.kind() == SyntaxKind::USE
                && let Some(use_item) = ast::Use::cast(node)
                && let Some(tree) = use_item.use_tree()
                && let Some(path) = tree.path()
            {
                let import_path = path.to_string();

                if let Some(resolved_path) = self.resolve_import(&import_path, file) {
                    imports.push(resolved_path);
                }
            }
        }

        Ok(imports)
    }

    /// Resolve a single import path relative to the current file into a file path.
    fn resolve_import(&self, import_path: &str, current_file: &Path) -> Option<PathBuf> {
        let parts: Vec<&str> = import_path.split("::").collect();
        
        if parts.is_empty() {
            return None;
        }

        if parts[0] == "crate" {
            let project_src = self.backend.project_root.join("src");
            return self.resolve_crate_import(&parts[1..], &project_src);
        }

        if parts[0] == "super" {
            if let Some(parent) = current_file.parent()
                && let Some(grandparent) = parent.parent()
            {
                return self.resolve_crate_import(&parts[1..], grandparent);
            }
            return None;
        }

        if parts[0] == "self" {
            if let Some(parent) = current_file.parent() {
                return self.resolve_crate_import(&parts[1..], parent);
            }
            return None;
        }

        None
    }

    /// Resolve a module path inside a crate starting from a base path.
    fn resolve_crate_import(&self, parts: &[&str], base_path: &Path) -> Option<PathBuf> {
        if parts.is_empty() {
            return None;
        }

        let module_name = parts[0];
        let module_file = base_path.join(format!("{module_name}.rs"));
        
        if module_file.exists() && parts.len() == 1 {
            return Some(module_file);
        }

        let module_dir = base_path.join(module_name).join("mod.rs");
        if module_dir.exists() {
            if parts.len() == 1 {
                return Some(module_dir);
            }
            
            return self.resolve_crate_import(&parts[1..], &base_path.join(module_name));
        }

        None
    }
}

