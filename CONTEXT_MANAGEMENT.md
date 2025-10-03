# Context Management System

## Overview

The context management system intelligently selects and prioritizes files to include in the LLM context, respecting token limits and quality thresholds.

## Key Features

### 1. **Token-Based Limits**
- **Maximum**: 10,000 tokens per context
- **Estimation**: ~4 chars per token (character-based) + word-based averaging
- **Smart Truncation**: Files can be truncated to fit within limits

### 2. **Priority-Based Selection**
Files are prioritized and added in order:

1. **Critical** (Priority 3): Explicitly requested files
2. **High** (Priority 2): Symbol matches, entry points from AI plan
3. **Medium** (Priority 1): Semantic search results (with scores)
4. **Low** (Priority 0): Pattern matches with low similarity

### 3. **Semantic Search Filtering**
- **Minimum Score**: 0.7 (configurable via `MIN_SIMILARITY_SCORE`)
- **Top-K Results**: 10 files (increased from 5)
- Only files above threshold are included

### 4. **Smart File Size Limits**
Dynamic limits based on file size:
- **Small files** (<10KB): Up to 50KB embedded
- **Medium files** (10-50KB): Up to 200KB embedded
- **Large files** (>50KB): Up to 100KB embedded
- **Empty files**: Automatically skipped

## Architecture

```
src/
├── context_inclusion.rs       # Token counting & priority management
└── embedding/
    ├── mod.rs                 # Module exports
    ├── client.rs              # Ollama embedding client
    └── vector_search.rs       # Vector store & search
```

## Usage Example

```rust
use agentic_context::context_inclusion::{
    ContextManager, PrioritizedFile, FilePriority, MAX_CONTEXT_TOKENS
};

// Create context manager
let mut context_mgr = ContextManager::new(MAX_CONTEXT_TOKENS);

// Add files with priorities
let mut prioritized_files = vec![
    PrioritizedFile::new(high_priority_file, FilePriority::High),
    PrioritizedFile::with_score(semantic_match, FilePriority::Medium, 0.85),
];

// Add to context (sorted by priority, respects token limits)
let added = add_prioritized_files(&mut context_mgr, prioritized_files);

println!("Added {} files ({} tokens)", added, context_mgr.token_count());
```

## Configuration

### Token Limits
```rust
// In context_inclusion.rs
pub const MAX_CONTEXT_TOKENS: usize = 10_000;
```

### Similarity Threshold
```rust
// In context_inclusion.rs
pub const MIN_SIMILARITY_SCORE: f32 = 0.4;  // Lowered for code similarity
```

### File Size Limits
```rust
// In embedding/vector_search.rs
let max_size = if content.len() < 10_000 {
    50_000   // Small files
} else if content.len() < 50_000 {
    200_000  // Medium files
} else {
    100_000  // Large files
};
```

## Token Estimation

The system uses a hybrid approach:

```rust
fn estimate_tokens(text: &str) -> usize {
    let chars = text.len();
    let words = text.split_whitespace().count();
    
    // Character-based: ~4 chars per token
    let char_estimate = chars / 4;
    
    // Word-based: ~1.3 words per token
    let word_estimate = (words * 10) / 13;
    
    // Average of both
    (char_estimate + word_estimate) / 2
}
```

**Accuracy**: ±10% for most code files

## Priority System

### How Files Are Selected

1. **Collect all candidate files**:
   - AI plan results (symbols, patterns, entry points)
   - Semantic search results (with similarity scores)

2. **Assign priorities**:
   - Plan results → High priority
   - Semantic matches → Medium priority (with score)

3. **Sort by priority**:
   - Primary: Priority level (high → low)
   - Secondary: Similarity score (high → low)

4. **Add until token limit**:
   - Add files in sorted order
   - Stop when token limit reached
   - Optionally truncate last file to fit

### Example Selection

```
Query: "Fix the authentication bug"

Files collected:
1. auth/login.rs (High, plan match)
2. auth/session.rs (High, plan match)
3. middleware/auth.rs (Medium, score: 0.89)
4. utils/crypto.rs (Medium, score: 0.75)
5. tests/auth_test.rs (Medium, score: 0.71)

Token budget: 10,000 tokens

Selection:
✓ auth/login.rs (2,500 tokens) - High priority
✓ auth/session.rs (3,000 tokens) - High priority
✓ middleware/auth.rs (2,800 tokens) - Medium, score 0.89
✓ utils/crypto.rs (1,500 tokens) - Medium, score 0.75
✗ tests/auth_test.rs - Would exceed limit (9,800 + 2,100 > 10,000)

Final: 4 files, 9,800 tokens
```

## Performance

### Token Counting
- **Speed**: O(n) where n = text length
- **Memory**: Minimal (single pass)

### Priority Sorting
- **Speed**: O(n log n) where n = number of files
- **Memory**: O(n) for sorted list

### Context Building
- **Speed**: O(n) where n = number of files
- **Memory**: O(n) for file contents

## Future Improvements

- [ ] Chunk-based embedding for large files
- [ ] Dynamic token limits based on model
- [ ] File importance scoring (beyond similarity)
- [ ] Dependency-aware selection
- [ ] Incremental context updates
