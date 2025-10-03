# Smart Chunking System

## Overview

The chunking system breaks files into logical segments (functions, structs, headers, paragraphs) and embeds each chunk separately. This provides **precise semantic search** - only the relevant chunk is included in context, not the entire file.

## Benefits

1. **Precision**: Match specific functions/sections, not entire files
2. **Efficiency**: Smaller chunks = better embeddings
3. **Context Control**: Only relevant code in context
4. **Scalability**: Large files don't overwhelm embeddings

## Chunking Strategies

### Rust Files (`.rs`)

**Strategy**: Split by language constructs
- Functions: `fn name(...)`
- Structs: `struct Name { ... }`
- Enums: `enum Name { ... }`
- Traits: `trait Name { ... }`
- Impls: `impl Name { ... }`
- Modules: `mod name { ... }`

**Example**:
```rust
// File: src/auth.rs (300 lines)

// Chunk 1: "fn login" (lines 1-45)
pub fn login(user: &str, pass: &str) -> Result<Token> {
    // ... 40 lines of login logic
}

// Chunk 2: "fn validate_token" (lines 47-89)
pub fn validate_token(token: &Token) -> bool {
    // ... 40 lines of validation
}

// Chunk 3: "struct Session" (lines 91-150)
pub struct Session {
    // ... session implementation
}
```

**Result**: 3 chunks instead of 1 large file

### Markdown Files (`.md`)

**Strategy**: Split by headers
- Major sections: `#`, `##` headers
- Merge small sections (<5 lines)
- Split large sections (>100 lines) on empty lines

**Example**:
```markdown
# API Documentation

## Authentication
[20 lines about auth]

## Endpoints
[80 lines about endpoints]

## Error Handling
[30 lines about errors]
```

**Result**: 3 chunks (preamble, Authentication, Endpoints, Error Handling)

### Text Files (`.txt`, `.log`)

**Strategy**: Split by paragraphs
- Break on empty lines
- Max 50 lines per chunk
- Minimum 5 lines per chunk

### Config Files (`.toml`, `.yaml`, `.json`)

**Strategy**: Split by top-level sections
- TOML: `[section]` headers
- YAML: Top-level keys
- JSON: Top-level objects

### Generic Code

**Strategy**: Split on empty lines
- Max 80 lines per chunk
- Minimum 20 lines per chunk
- Force split at max size

## Cache Format

Each chunk is cached separately:

```rust
struct CachedEmbedding {
    path: PathBuf,           // Original file path
    chunk_id: String,        // "fn login", "## Overview", etc.
    start_line: usize,       // 1-indexed start line
    end_line: usize,         // 1-indexed end line
    embedding: Vec<f32>,     // 768-dim vector
    preview: String,         // First 200 chars
    modified: SystemTime,    // File modification time
}
```

**Cache Version**: 2 (bumped for chunk-based system)

## Search Results

Search results now include chunk information:

```
--- Semantic search found 5 matches
  1. src/auth.rs:1-45 [fn login] (score: 0.892)
  2. src/auth.rs:47-89 [fn validate_token] (score: 0.847)
  3. src/middleware/auth.rs:120-180 [impl AuthMiddleware] (score: 0.801)
  4. docs/API.md:45-89 [## Authentication] (score: 0.776)
  5. src/session.rs:1-60 [struct Session] (score: 0.734)
```

## Context Inclusion

Only the **matching chunk** is added to context, not the entire file:

**Before (whole file)**:
```
File: src/auth.rs (300 lines, ~2500 tokens)
[entire file content]
```

**After (chunk only)**:
```
File: src/auth.rs:1-45 (fn login)
pub fn login(user: &str, pass: &str) -> Result<Token> {
    // ... only the login function
}
```

**Savings**: 2500 tokens → 400 tokens (84% reduction)

## Performance

### Embedding Time

**Before (whole files)**:
- 1000 files × 1 embedding = 1000 embeddings
- Time: ~50-100 seconds

**After (chunked)**:
- 1000 files × 3 chunks avg = 3000 embeddings
- Time: ~150-300 seconds (first run)
- Cache hit: ~100ms (subsequent runs)

### Search Quality

**Before**: Match entire file, get 300 lines of code
**After**: Match specific function, get 40 lines of relevant code

**Precision improvement**: ~7.5x more relevant context

## Configuration

### Chunk Size Limits

```rust
// In chunker.rs

// Rust: Natural boundaries (functions, structs)
// No artificial limits

// Markdown
const MIN_CHUNK_SIZE: usize = 5;    // Lines
const MAX_CHUNK_SIZE: usize = 100;  // Lines

// Text
const MAX_CHUNK_SIZE: usize = 50;   // Lines

// Generic code
const MAX_CHUNK_SIZE: usize = 80;   // Lines
```

### File Size Limits

```rust
// In vector_search.rs

// Skip chunks larger than 200KB
if chunk.content.len() > 200_000 {
    continue;
}
```

## Example Output

### Indexing
```
⚙️  Building embedding index...
  Found 247 source files to embed
    Embedded: src/main.rs:1-45 [fn main] (dim: 768)
    Embedded: src/main.rs:47-120 [fn setup] (dim: 768)
    Embedded: src/lib.rs:1-30 [module] (dim: 768)
    Embedded: src/auth.rs:1-45 [fn login] (dim: 768)
    Embedded: src/auth.rs:47-89 [fn validate_token] (dim: 768)
    ...
  Embedded 247 files total (892 chunks)
✓ Indexed 247 files with embeddings
```

### Searching
```
  Vector search: store has 892 embeddings
  Query embedded (dim: 768)
  Found 50 results before filtering
  Top scores: [0.892, 0.847, 0.801, 0.776, 0.734]
  After filtering (score >= 0.3): 12 results

--- Semantic search found 12 matches
  1. src/auth.rs:1-45 [fn login] (score: 0.892)
  2. src/auth.rs:47-89 [fn validate_token] (score: 0.847)
  ...
```

## Future Improvements

- [ ] AST-based chunking for better boundaries
- [ ] Overlap between chunks for context
- [ ] Chunk size optimization per file type
- [ ] Hierarchical chunking (file → section → chunk)
- [ ] Cross-chunk references
