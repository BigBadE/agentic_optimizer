# merlin-context

Context building, file indexing, semantic search, and conversation context management.

## Purpose

This crate provides intelligent context management for LLM interactions:
- Semantic search using BM25 and vector embeddings
- File chunking with language-aware strategies
- Context building from project files
- Conversation context management
- Query analysis and intent extraction

## Module Structure

### Context Management
- `builder.rs` - `ContextBuilder` for assembling LLM prompts
- `context_fetcher.rs` - Fetch relevant context using semantic search
- `context_inclusion.rs` - Manage conversation context inclusion
- `models.rs` - Data models for context structures
- `fs_utils.rs` - File system utilities

### Query Analysis (`query/`)
- `analyzer.rs` - Analyze user queries for intent
- `types.rs` - Query analysis types

### Embedding System (`embedding/`)
- `client.rs` - `EmbeddingClient` for generating embeddings
- `bm25.rs` - BM25 text search implementation
- `vector_search.rs` - Vector similarity search
- `chunking/` - File chunking strategies
  - `config.rs` - Chunking configuration
  - `generic.rs` - Generic file chunker
  - `markdown.rs` - Markdown-aware chunking
  - `rust.rs` - Rust-aware chunking
  - `text.rs` - Plain text chunking

## Public API

**94 public items** including:
- `ContextBuilder` - Build context from project files
- `ContextFetcher` - Fetch context with semantic search
- `EmbeddingClient` - Generate embeddings via API
- `EmbeddingProvider` - Embedding provider enum (OpenAI, Voyage)
- `VectorStore` - In-memory vector storage
- `VectorSearchManager` - Manage vector search operations
- `BM25Index` - BM25 text search
- `FileChunk` - Chunked file representation
- `chunk_file()` - Chunk files with language awareness

## Features

### Semantic Search
- **BM25**: Fast keyword-based search with TF-IDF weighting
- **Vector embeddings**: Dense vector search using OpenAI/Voyage embeddings
- **Hybrid search**: Combine BM25 and vector search for best results

### File Chunking
Language-aware chunking preserves semantic boundaries:
- **Rust**: Chunks by function, struct, impl, mod boundaries
- **Markdown**: Chunks by heading hierarchy
- **Plain text**: Fixed-size chunks with overlap
- **Generic**: Fallback for unknown file types

### Context Building
Assemble relevant context for LLM prompts:
- File content with metadata
- Conversation history
- Query analysis
- Token limit management

## Testing Status

**✅ Well-tested**

- **Unit tests**: Multiple files with comprehensive coverage
- **Integration tests**: `tests/integration_tests.rs`
- **Modular tests**:
  - `tests/modules/bm25_tokenization.rs` - BM25 tokenization
  - `tests/modules/chunking_validation.rs` - File chunking
  - `tests/modules/embedding_cache.rs` - Embedding caching
- **Fixture coverage**: 10+ fixtures for context requests and conversation management

## Code Quality

- ✅ **Documentation**: All public items documented
- ✅ **Error handling**: Proper `Result<T, E>` usage
- ✅ **No dead code**: All modules actively used
- ✅ **No clippy violations**: Strict linting compliance
- ✅ **No TODOs**: Implementation complete

## Dependencies

- `serde` - Serialization
- `tokio` - Async runtime
- `reqwest` - HTTP client for embedding APIs
- `glob` - File pattern matching
- `ignore` - Gitignore support

## Usage Example

```rust
use merlin_context::{ContextBuilder, ContextFetcher, EmbeddingProvider};

// Build context from files
let context = ContextBuilder::new()
    .add_file("src/main.rs")
    .add_file("src/lib.rs")
    .build()?;

// Semantic search
let fetcher = ContextFetcher::new(EmbeddingProvider::OpenAI, "api_key");
let results = fetcher.search("error handling", 5).await?;

// Use in LLM query
let query = Query::new("Explain error handling")
    .with_context(context);
```

## Issues and Recommendations

**None** - This crate is well-maintained with excellent test coverage and comprehensive documentation.
