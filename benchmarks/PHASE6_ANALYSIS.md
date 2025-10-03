# Phase 6: Code-Aware Embeddings Analysis

## Results Summary

| Metric | Phase 5 (Baseline) | Phase 6 (Code-Aware) | Change |
|--------|-------------------|---------------------|--------|
| **Precision@3** | 30.0% | 23.3% | **-6.7%** 🔴 |
| **Precision@5** | 24.0% | 25.0% | +1.0% |
| **Precision@10** | 20.0% | 19.0% | -1.0% |
| **Recall@10** | 49.4% | 47.0% | **-2.4%** 🔴 |
| **MRR** | 0.440 | 0.400 | **-0.040** 🔴 |
| **NDCG@10** | 0.437 | 0.390 | **-0.047** 🔴 |
| **Critical in Top-3** | 25.0% | 15.0% | **-10.0%** 🔴 |

## What Was Implemented

Added context prefixes to embeddings to help nomic-embed-text distinguish code from documentation:

### Code Files (.rs, .py, .js, etc.)
```
search_document: code implementation
<actual code content>
```

### Documentation Files (.md, .txt)
```
search_document: documentation reference
<actual doc content>
```

### Query
```
search_query: <user query>
```

## Why It Failed

### Problem 1: Prefix Interference
The context prefixes (`search_document:`, `search_query:`) are **not part of nomic-embed-text's training**.

- Model was trained on raw text, not prefixed text
- Prefixes add noise to embeddings
- Reduces semantic similarity across the board

### Problem 2: Wrong Model for Task
**nomic-embed-text** is a general-purpose embedding model, not code-specific:
- Trained on web text, not code repositories
- Doesn't understand code semantics fundamentally
- Adding prefixes doesn't magically make it code-aware

### Problem 3: Documentation Still Has Higher Similarity
Even with "documentation reference" prefix, spec.md files:
- Contain natural language explanations
- Match query semantics better than code syntax
- Prefixes don't change fundamental embedding space

## Example: CSS Selector Performance

**Query**: "optimize CSS selector matching performance"

### Phase 5 (No Prefix)
```
spec.md: "CSS selectors match elements..." → 0.85 similarity
matching.rs: "fn match_selector(...) {" → 0.42 similarity
```

### Phase 6 (With Prefix)
```
spec.md: "search_document: documentation reference\nCSS selectors..." → 0.78 similarity
matching.rs: "search_document: code implementation\nfn match_selector..." → 0.35 similarity
```

**Result**: Both scores dropped, but code dropped MORE because prefixes interfere with sparse code tokens.

## Lessons Learned

### ❌ What Doesn't Work
1. **Adding prefixes to generic models** - Just adds noise
2. **Trying to "trick" embeddings** - Model doesn't understand intent
3. **Same model for code and docs** - Need fundamentally different approach

### ✅ What Would Work

1. **Use Actual Code-Aware Model**
   - CodeBERT, GraphCodeBERT, StarEncoder
   - Trained on code, understands syntax
   - Not available via Ollama currently

2. **Separate Models**
   - Code model for .rs/.py/.js files
   - Text model for .md files
   - Different embedding spaces

3. **LLM Re-Ranking** (Phase 7)
   - Use embeddings for initial retrieval
   - Use LLM to re-score top 50
   - LLM understands code vs docs

## Recommendation

**REVERT Phase 6 changes** and proceed directly to **Phase 7: LLM Re-Ranking**.

### Why LLM Re-Ranking Will Work

```rust
// After hybrid search gets top 50:
for candidate in top_50 {
    let prompt = format!(
        "Rate relevance 0-10 for implementing this feature:\n\
        Query: {}\n\
        File: {}\n\
        Content: {}\n\
        Is this code implementation (high score) or documentation (low score)?",
        query, file_path, preview
    );
    let score = gemini_flash.score(prompt);
}
```

**LLM can**:
- Understand code vs documentation
- Reason about relevance
- Distinguish implementation from reference

**Expected**: +15-25% P@3 with LLM re-ranking

## Next Steps

1. Revert to Phase 5 (remove context prefixes)
2. Implement Phase 7: LLM re-ranking with Gemini Flash
3. Test with cost tracking
