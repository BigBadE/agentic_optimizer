# Vector Search System

## Overview

The vector search system provides semantic code search capabilities using Ollama embeddings. It automatically indexes your codebase on startup and caches embeddings for fast subsequent queries.

## Features

- **Automatic Indexing**: Embeds all source files on first run
- **Persistent Caching**: Stores embeddings in `.agentic_cache/embeddings.bin`
- **Smart Invalidation**: Detects file modifications and re-embeds only changed files
- **Parallel Processing**: Runs vector search in parallel with AI plan generation
- **Batch Embedding**: Processes files in batches of 10 for efficiency

## How It Works

### 1. Initialization

On startup, the `VectorSearchManager`:
1. Checks for existing cache at `.agentic_cache/embeddings.bin`
2. Validates cache version and file modification times
3. Re-embeds modified files or builds full index if cache is invalid
4. Saves updated cache to disk

### 2. Query Flow

When you run a query:
1. **Parallel Execution**: Vector search and AI plan generation run simultaneously
2. **Embedding**: Query text is embedded using `nomic-embed-text` model
3. **Search**: Cosine similarity computed against all indexed files
4. **Ranking**: Top-k most similar files returned with scores

### 3. Caching Strategy

**Cache Structure:**
```rust
struct VectorCache {
    version: u32,               // For cache invalidation
    embeddings: Vec<CachedEmbedding>
}

struct CachedEmbedding {
    path: PathBuf,              // File path
    preview: String,            // First 200 chars
    modified: SystemTime,       // Last modification time
}
```
**### Cache Invalidation:**
- Version mismatch → Full rebuild
- File modified → Re-embed that file
- File deleted → Remove from cache
- **New file → Automatically detected and embedded**

## Configuration

### Environment Variables
# Embedding model (default: nomic-embed-text)
export EMBEDDING_MODEL="nomic-embed-text"
```

### Automatic Model Installation

The system **automatically downloads** the embedding model if it's not found:

1. **Detection**: Checks if `nomic-embed-text` is installed
2. **Download**: Runs `ollama pull nomic-embed-text` automatically
3. **Progress**: Shows real-time download progress
4. **Verification**: Confirms successful installation

**No manual setup required!** Just ensure Ollama is running (`ollama serve`).

### Cache Location

```
<project_root>/.agentic_cache/embeddings.bin
```

**Size Estimates:**
- ~3KB per file (768-dim float32 vector + metadata)
- 1000 files ≈ 3MB cache
- 10,000 files ≈ 30MB cache

## Performance

### Initial Indexing

**Speed:**
- ~10-20 files/second (depends on Ollama GPU performance)
- 1000 files ≈ 1-2 minutes
- Batched processing (10 files at a time)

**Skips:**
- Files > 100KB (too large)
- Non-source files (filtered by `is_source_file()`)
- Gitignored files

### Query Performance

- **Embedding query**: 10-50ms
- **Cosine similarity**: <1ms per file
- **Total search time**: ~50-100ms for 1000 files

## Usage Example

```rust
// Initialize vector search manager
let mut manager = VectorSearchManager::new(project_root);
manager.initialize().await?;

// Search for similar files
let results = manager.search("fix infinite loading bug", 5).await?;

for result in results {
    println!("{} (score: {:.3})", result.file_path.display(), result.score);
}
```

## When to Use Vector Search

### ✅ **Good Use Cases**

- **Conceptual queries**: "find authentication logic", "locate error handling"
- **Vague bug descriptions**: "fix infinite loading", "memory leak"
- **Cross-cutting concerns**: "validation", "logging", "caching"
- **Exploratory searches**: "similar to this file"

### ❌ **Poor Use Cases**

- **Specific symbols**: "find function `build_context`" → Use symbol search instead
- **File paths**: "show me src/builder.rs" → Use pattern matching instead
- **Imports**: "trace from main.rs" → Use entry-point traversal instead

## Troubleshooting

### Cache Not Loading

**Symptoms**: Full re-indexing every time

**Causes:**
- Cache version mismatch
- Corrupted cache file
- Permission issues

**Solution:**
```bash
# Delete cache and rebuild
rm -rf .agentic_cache
# Next run will rebuild from scratch
```

### Slow Indexing

**Symptoms**: Takes >5 minutes for 1000 files

**Causes:**
- Ollama not using GPU
- CPU fallback mode
- Large files not being skipped

**Solution:**
1. Check GPU usage: `nvidia-smi` (during embedding)
2. Verify Ollama uses GPU: Check `ollama serve` logs
3. Reduce file size limit in `vector_search.rs:180` if needed

### Out of Memory

**Symptoms**: Ollama crashes during indexing

**Causes:**
- Too many concurrent embeddings
- Large files consuming VRAM

**Solution:**
1. Reduce batch size in `vector_search.rs:166` (default: 10)
2. Reduce file size limit (default: 100KB)
3. Use smaller embedding model (requires code change)

## Advanced Configuration

### Custom Batch Size

Edit `vector_search.rs`:
```rust
const BATCH_SIZE: usize = 5;  // Reduce from 10 if OOM
```

### Custom File Size Limit

Edit `vector_search.rs`:
```rust
if content.len() > 50_000 {  // Reduce from 100_000
    return None;
}
```

### Different Embedding Model

Edit `models.rs`:
```rust
embedding: env::var("EMBEDDING_MODEL")
    .unwrap_or_else(|_| "mxbai-embed-large".to_string()),
```

**Available Models:**
- `nomic-embed-text` (768-dim, fast, good quality) ← **Default**
- `mxbai-embed-large` (1024-dim, slower, better quality)
- `all-minilm` (384-dim, fastest, lower quality)

## Architecture

```
ContextBuilder
    ├── VectorSearchManager
    │   ├── VectorStore (in-memory)
    │   │   └── HashMap<PathBuf, Embedding>
    │   ├── EmbeddingClient (Ollama)
    │   └── Cache (disk: .agentic_cache/embeddings.bin)
    │
    └── Parallel Execution
        ├── Plan Generation (Ollama chat)
        └── Vector Search (Ollama embeddings)
```

## Cache Format

Binary format using `bincode` serialization:

```
[Version: u32]
[Embedding Count: usize]
[
  For each embedding:
    [Path Length: usize][Path: UTF-8 bytes]
    [Embedding Dim: usize][Embedding: f32 array]
    [Preview Length: usize][Preview: UTF-8 bytes]
    [Modified: SystemTime]
]
```

## Future Improvements

- [ ] Incremental updates (watch file system)
- [ ] Chunk-based embedding (for large files)
- [ ] Hybrid search (vector + keyword)
- [ ] Multiple embedding models
- [ ] Distributed caching
- [ ] Query result caching
