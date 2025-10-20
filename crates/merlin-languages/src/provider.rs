use merlin_core::{CoreResult as Result, FileContext};
use std::path::{Path, PathBuf};

/// Information about a code symbol (function, struct, etc.)
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// The name of the symbol
    pub name: String,
    /// The kind/type of symbol
    pub kind: SymbolKind,
    /// The file containing this symbol
    pub file_path: PathBuf,
    /// The line number where the symbol is defined
    pub line: u32,
    /// Optional documentation for the symbol
    pub documentation: Option<String>,
}

/// The kind of code symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// A function
    Function,
    /// A struct type
    Struct,
    /// An enum type
    Enum,
    /// A trait definition
    Trait,
    /// A module
    Module,
    /// A constant
    Constant,
    /// A variable
    Variable,
    /// A struct or enum field
    Field,
    /// A method on a type
    Method,
    /// A type alias
    Type,
}

/// Query parameters for symbol search
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Optional symbol name to search for
    pub symbol_name: Option<String>,
    /// Whether to include references to the symbol
    pub include_references: bool,
    /// Whether to include trait implementations
    pub include_implementations: bool,
    /// Maximum number of results to return
    pub max_results: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            symbol_name: None,
            include_references: false,
            include_implementations: false,
            max_results: 50,
        }
    }
}

/// Results from a symbol search
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The symbols found
    pub symbols: Vec<SymbolInfo>,
    /// Related file contexts
    pub related_files: Vec<FileContext>,
}

/// Language-specific code analysis provider
pub trait LanguageProvider: Send + Sync {
    /// Initialize the provider for a project
    ///
    /// # Errors
    /// Returns an error if the project cannot be loaded or analyzed
    fn initialize(&mut self, project_root: &Path) -> Result<()>;

    /// Search for symbols matching a query
    ///
    /// # Errors
    /// Returns an error if the search fails
    fn search_symbols(&self, query: &SearchQuery) -> Result<SearchResult>;

    /// Find the definition of a symbol at a specific location
    ///
    /// # Errors
    /// Returns an error if the file cannot be analyzed
    fn find_definition(
        &self,
        symbol_name: &str,
        file: &Path,
        line: u32,
    ) -> Result<Option<SymbolInfo>>;

    /// Find all references to a symbol
    ///
    /// # Errors
    /// Returns an error if the search fails
    fn find_references(&self, symbol_name: &str) -> Result<Vec<SymbolInfo>>;

    /// Get files related to a given file through imports/dependencies
    ///
    /// # Errors
    /// Returns an error if the file cannot be analyzed
    fn get_related_context(&self, file: &Path) -> Result<Vec<FileContext>>;

    /// Extract import paths from a file
    ///
    /// # Errors
    /// Returns an error if the file cannot be parsed
    fn extract_imports(&self, file: &Path) -> Result<Vec<PathBuf>>;

    /// List all symbols defined in a file
    ///
    /// # Errors
    /// Returns an error if the file cannot be analyzed
    fn list_symbols_in_file(&self, file: &Path) -> Result<Vec<SymbolInfo>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_kind_equality() {
        assert_eq!(SymbolKind::Function, SymbolKind::Function);
        assert_ne!(SymbolKind::Function, SymbolKind::Struct);
        assert_eq!(SymbolKind::Method, SymbolKind::Method);
    }

    #[test]
    fn test_symbol_info_creation() {
        let symbol = SymbolInfo {
            name: "test_function".to_owned(),
            kind: SymbolKind::Function,
            file_path: PathBuf::from("src/lib.rs"),
            line: 42,
            documentation: Some("Test documentation".to_owned()),
        };

        assert_eq!(symbol.name, "test_function");
        assert_eq!(symbol.kind, SymbolKind::Function);
        assert_eq!(symbol.line, 42);
        assert!(symbol.documentation.is_some());
    }

    #[test]
    fn test_search_query_default() {
        let query = SearchQuery::default();
        assert!(query.symbol_name.is_none());
        assert!(!query.include_references);
        assert!(!query.include_implementations);
        assert_eq!(query.max_results, 50);
    }

    #[test]
    fn test_search_query_with_name() {
        let query = SearchQuery {
            symbol_name: Some("MyStruct".to_owned()),
            include_references: true,
            include_implementations: false,
            max_results: 10,
        };

        assert_eq!(query.symbol_name, Some("MyStruct".to_owned()));
        assert!(query.include_references);
        assert_eq!(query.max_results, 10);
    }

    #[test]
    fn test_search_result_creation() {
        let symbols = vec![
            SymbolInfo {
                name: "func1".to_owned(),
                kind: SymbolKind::Function,
                file_path: PathBuf::from("src/lib.rs"),
                line: 10,
                documentation: None,
            },
            SymbolInfo {
                name: "MyStruct".to_owned(),
                kind: SymbolKind::Struct,
                file_path: PathBuf::from("src/types.rs"),
                line: 20,
                documentation: Some("A struct".to_owned()),
            },
        ];

        let result = SearchResult {
            symbols,
            related_files: vec![],
        };

        assert_eq!(result.symbols.len(), 2);
        assert_eq!(result.symbols[0].name, "func1");
        assert_eq!(result.symbols[1].kind, SymbolKind::Struct);
        assert!(result.related_files.is_empty());
    }

    #[test]
    fn test_symbol_kind_debug() {
        let kind = SymbolKind::Function;
        let debug_str = format!("{kind:?}");
        assert_eq!(debug_str, "Function");
    }
}
