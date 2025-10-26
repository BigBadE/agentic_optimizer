# merlin-languages

Language-specific code analysis via language servers.

## Purpose

This crate provides language-aware code analysis through Language Server Protocol (LSP) backends. Currently supports Rust via rust-analyzer, with extensibility for additional languages.

## Module Structure

- `lib.rs` - Language enum and factory function
- `provider.rs` - `LanguageProvider` trait definition
- `backends.rs` - rust-analyzer backend implementation

## Public API

- `Language` - Enum of supported languages (currently: `Rust`)
- `LanguageProvider` - Trait for language backend implementations
- `create_backend(language)` - Factory function to create language backends
- `SearchQuery` - Query for code search
- `SearchResult` - Code search results
- `SymbolInfo` - Symbol information (name, kind, location)
- `SymbolKind` - Symbol types (Function, Struct, Enum, Trait, etc.)

## Features

### Rust Support (via rust-analyzer)
- Symbol search
- Definition lookup
- Type information
- Code navigation

### Extensibility
The `LanguageProvider` trait allows adding new language backends:
```rust
pub trait LanguageProvider {
    fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>>;
    fn get_definition(&self, file: &Path, position: Position) -> Result<Location>;
    fn get_type_info(&self, file: &Path, position: Position) -> Result<String>;
}
```

## Testing Status

**✅ Basic coverage**

- **Unit tests**: 1 basic test in lib.rs
- **Integration tests**: `tests/rust_backend_tests.rs` with 8 tests
  - Backend creation and trait implementation
  - Search query handling
  - Symbol kind validation
  - Error handling

## Code Quality

- ✅ **Documentation**: All public items documented
- ✅ **Error handling**: Proper `Result<T, E>` usage
- ✅ **No dead code**: All modules used
- ✅ **No TODOs**: Implementation complete

## Dependencies

- `lsp-types` - Language Server Protocol types
- `serde` - Serialization
- `thiserror` - Error handling

## Usage Example

```rust
use merlin_languages::{Language, create_backend, SearchQuery};

// Create Rust backend
let backend = create_backend(Language::Rust)?;

// Search for symbols
let query = SearchQuery::new("MyStruct");
let results = backend.search(&query)?;

for result in results {
    println!("{}: {:?}", result.symbol.name, result.symbol.kind);
}
```

## Issues and Recommendations

### Future Enhancements
1. Add more comprehensive rust-analyzer integration tests with real code
2. Add fixture coverage for language analysis scenarios
3. Add support for more languages (TypeScript, Python, Go)
4. Implement caching for language server results
5. Add incremental analysis support
