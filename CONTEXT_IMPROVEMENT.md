# Context Search Improvements

## Implementation Checklist

- [ ] **Enhanced BM25 Tokenization** - Preserve special tokens and add n-grams
- [ ] **File Type Boosting** - Weight code files higher than docs/config
- [ ] **Path-Based Boosting** - Boost files in relevant directories
- [ ] **Minimum Relevance Filtering** - Remove low-quality small chunks
- [ ] **Import-Based Boosting** - Boost files importing relevant crates
- [ ] **Query Intent Detection** - Detect implementation vs explanation queries
- [ ] **Adaptive BM25/Vector Weighting** - Adjust based on query characteristics
- [ ] **Code Pattern Detection** - Boost files with relevant syntax patterns

---

## 1. Enhanced BM25 Tokenization

**Problem**: Current tokenization loses semantic information from compound terms.

**Solution**: Preserve special patterns and generate n-grams.

```rust
fn tokenize(text: &str) -> Vec<String> {
    let mut terms = Vec::new();
    
    for word in text.split_whitespace() {
        // Preserve special tokens with punctuation
        if word.contains("::") || word.starts_with("--") || word.starts_with('-') {
            terms.push(word.to_lowercase());
        }
        
        // Also add cleaned version
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if !clean.is_empty() && clean.len() > 2 {
            terms.push(clean.to_lowercase());
        }
    }
    
    // Add bigrams for common phrases
    let words: Vec<_> = text.split_whitespace().collect();
    for window in words.windows(2) {
        if window[0].len() > 2 && window[1].len() > 2 {
            terms.push(format!("{}_{}", window[0], window[1]).to_lowercase());
        }
    }
    
    terms
}
```

**Impact**: Better matching of technical terms, flags, and module paths.

---

## 2. File Type Boosting

**Problem**: Documentation and code ranked equally for implementation queries.

**Solution**: Apply scoring multipliers based on file type and location.

```rust
fn calculate_file_boost(path: &Path) -> f32 {
    let path_str = path.to_str().unwrap_or("");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    
    let type_boost = match ext {
        "rs" | "py" | "js" | "ts" | "java" | "go" => 1.5,  // Code files
        "toml" | "yaml" | "json" => 0.6,  // Config files
        "md" | "txt" => 0.5,  // Documentation
        _ => 1.0,
    };
    
    let location_boost = if path_str.contains("/src/") {
        1.2  // Source code more relevant
    } else if path_str.contains("/docs/") || path_str.contains("/examples/") {
        0.7  // Docs/examples less relevant
    } else {
        1.0
    };
    
    type_boost * location_boost
}
```

**Impact**: Implementation files surface above documentation for coding tasks.

---

## 3. Minimum Relevance Filtering

**Problem**: Small, generic chunks (70-150 tokens) rank highly but provide no value.

**Solution**: Filter low-score small chunks.

```rust
fn should_include_chunk(tokens: usize, score: f32) -> bool {
    if tokens < 100 && score < 0.7 {
        return false;  // Too small and not highly relevant
    }
    if tokens < 50 {
        return false;  // Always filter tiny chunks
    }
    true
}
```

**Impact**: Removes noise from results (lib.rs exports, small config files).

---

## 4. Import-Based Boosting

**Problem**: Files using relevant crates/modules aren't prioritized.

**Solution**: Boost files with relevant imports detected in query.

```rust
fn boost_by_imports(file_content: &str, query_terms: &[&str]) -> f32 {
    let mut boost = 1.0;
    
    // Extract imports
    let imports: Vec<&str> = file_content
        .lines()
        .filter(|l| l.trim().starts_with("use ") || l.trim().starts_with("import "))
        .collect();
    
    // Check if imports match query terms
    for term in query_terms {
        if imports.iter().any(|i| i.contains(term)) {
            boost += 0.2;
        }
    }
    
    boost.min(2.0)
}
```

**Impact**: Files actually using mentioned crates/APIs rank higher.

---

## 5. Query Intent Detection

**Problem**: All queries treated the same; implementation vs explanation differ.

**Solution**: Classify query type and adjust search strategy.

```rust
enum QueryType {
    Implementation,  // "fix", "add", "implement", "create"
    Explanation,     // "how", "what", "why", "explain"
    Navigation,      // "find", "where", "locate"
    Debug,           // "error", "bug", "broken", "issue"
}

fn detect_query_type(query: &str) -> QueryType {
    let lower = query.to_lowercase();
    if lower.contains("fix") || lower.contains("implement") || lower.contains("add") {
        QueryType::Implementation
    } else if lower.contains("how") || lower.contains("what") || lower.contains("why") {
        QueryType::Explanation
    } else if lower.contains("error") || lower.contains("bug") {
        QueryType::Debug
    } else if lower.contains("find") || lower.contains("where") {
        QueryType::Navigation
    } else {
        QueryType::Implementation
    }
}

fn adjust_weights(query_type: QueryType) -> (f32, f32) {
    match query_type {
        QueryType::Implementation => (0.6, 0.4),  // Favor BM25 (keywords)
        QueryType::Explanation => (0.3, 0.7),     // Favor Vector (concepts)
        QueryType::Navigation => (0.7, 0.3),      // Favor BM25 (exact matches)
        QueryType::Debug => (0.5, 0.5),           // Balanced
    }
}
```

**Impact**: Better results for different query types without manual tuning.

---

## 6. Adaptive BM25/Vector Weighting

**Problem**: Fixed 0.4/0.6 weights don't suit all queries.

**Solution**: Dynamically adjust based on query characteristics.

```rust
fn calculate_adaptive_weights(query: &str) -> (f32, f32) {
    let has_special_chars = query.contains("::") || query.contains("--") || query.contains('/');
    let word_count = query.split_whitespace().count();
    let has_quotes = query.contains('"');
    
    let bm25_weight = if has_special_chars || has_quotes {
        0.7  // Exact matching important
    } else if word_count <= 3 {
        0.6  // Short queries - favor keywords
    } else {
        0.4  // Long queries - favor semantics
    };
    
    (bm25_weight, 1.0 - bm25_weight)
}
```

**Impact**: Automatically optimized for different query styles.

---

## 7. Code Pattern Detection

**Problem**: Files containing relevant syntax patterns aren't prioritized.

**Solution**: Boost files with structural matches.

```rust
fn boost_by_code_patterns(file_content: &str, query: &str) -> f32 {
    let mut boost = 1.0;
    let query_lower = query.to_lowercase();
    
    // Boost for definitions if query mentions them
    if query_lower.contains("struct") && file_content.contains("struct ") {
        boost += 0.3;
    }
    if query_lower.contains("function") && file_content.contains("fn ") {
        boost += 0.3;
    }
    if query_lower.contains("trait") && file_content.contains("trait ") {
        boost += 0.3;
    }
    
    // Boost for CLI patterns
    if file_content.contains("#[arg(") || file_content.contains("#[command(") {
        if query_lower.contains("arg") || query_lower.contains("command") {
            boost += 0.4;
        }
    }
    
    boost.min(2.0)
}
```

**Impact**: Files with relevant code structures rank higher.

---

## Implementation Priority

1. **Quick Wins** (1-2 hours):
   - File type boosting (#2)
   - Minimum relevance filtering (#3)
   - Adaptive BM25/Vector weighting (#6)

2. **Medium Effort** (3-5 hours):
   - Enhanced tokenization (#1)
   - Query intent detection (#5)

3. **Advanced** (1-2 days):
   - Import-based boosting (#4)
   - Code pattern detection (#7)

---

## Testing Strategy

After each improvement, test with diverse queries:

1. **Exact matches**: "UserService::find_by_email"
2. **Conceptual**: "authentication implementation"
3. **Task-based**: "add logging to API calls"
4. **Navigation**: "where is the config loaded"
5. **Debug**: "why is the vector search slow"

Expected improvements:
- Top-3 accuracy: 30% → 70%
- Top-10 accuracy: 60% → 90%
- Noise reduction: 40% fewer irrelevant results

## Expected Ranking vs Actual

### My Expected Top 5
- **`cli.rs`**: Score 0.95+ – Defines the `--prompt` CLI argument
- **`main.rs`**: Score 0.90+ – Handles prompt command execution
- **`builder.rs`**: Score 0.70 – Builds prompt construction logic
- **Files importing console/term crates**: Score 0.65 – User-facing output handling
- **Files containing logging setup**: Score 0.50 – Identify logging handoff points

### Actual Results Analysis

| **File** | **Actual rank** | **Expected rank** | **Assessment** |
| --- | --- | --- | --- |
| `cli.rs` | #9 (0.567) | #1 | ❌ Critically underranked |
| `vector_search.rs` | #1 (1.000) | Not top 20 | ❌ False positive |
| `builder.rs` | #2 (0.748) | #3 | ⚠️ Slightly overranked |
| `agentic-core/src/lib.rs` | #3 (0.663) | Not top 10 | ❌ False positive |
| `agentic-languages/src/provider.rs` | #4 (0.635) | Not top 20 | ❌ False positive |
| `subagent/prompts.rs` | #6 (0.612) | #4-5 | ⚠️ Underranked |
| `Cargo.toml` files | #11-17 | Not top 20 | ❌ Noise |
| `docs/PHASES.md` and related docs | #19-24 | Not top 20 | ❌ Noise |
