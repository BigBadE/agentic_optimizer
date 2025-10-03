# Context Fetching Improvement Suggestions

## Current Performance (Phase 2 - Best Configuration)

| Metric | Current | Target | Gap |
|--------|---------|--------|-----|
| **Precision@3** | 25.0% | 40% | **+15%** needed |
| **Precision@10** | 20.0% | 50% | **+30%** needed |
| **Recall@10** | 49.2% | 60% | **+10.8%** needed |
| **MRR** | 0.479 | 0.650 | **+0.171** needed |
| **Critical in Top-3** | 22.5% | 50% | **+27.5%** needed |

## Problem Analysis

### Primary Issues

1. **Documentation Dominance** (affects Precision@3, MRR)
   - spec.md files ranking in top 10
   - README.md files appearing at #1
   - Current 0.1x penalty insufficient

2. **Critical Files Missing Top-3** (affects Critical in Top-3)
   - Only 22.5% of critical files appear in top 3
   - Entry point files (lib.rs) rank at #7+ instead of #1-3

3. **Semantic Over-Emphasis** (affects Precision)
   - Vector scores dominating for "explanation" queries
   - Semantically similar but irrelevant files ranking high

## Suggested Improvements

### 🎯 Priority 1: File Recency/Importance Signals

**Problem**: System treats all files equally regardless of importance

**Solution**: Add file importance scoring based on:

```rust
fn calculate_file_importance(path: &Path, repo_root: &Path) -> f32 {
    let mut importance = 1.0;
    
    // 1. Depth penalty - files deeper in tree less important
    let depth = path.components().count();
    importance *= 1.0 / (1.0 + (depth as f32 * 0.1));
    
    // 2. File size consideration - very small files less important
    if let Ok(metadata) = fs::metadata(path) {
        let size = metadata.len();
        if size < 100 {
            importance *= 0.3;  // Tiny files likely stubs
        } else if size < 500 {
            importance *= 0.7;  // Small files
        }
    }
    
    // 3. Naming patterns
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if filename.contains("test") || filename.contains("example") {
        importance *= 0.5;
    }
    
    importance
}
```

**Expected Impact**: 
- Precision@3: +5-8%
- Critical in Top-3: +10-15%

---

### 🎯 Priority 2: Query-File Relevance Scoring

**Problem**: Current system doesn't consider query-file alignment

**Solution**: Score how well query keywords match file purpose

```rust
fn calculate_query_file_alignment(query: &str, file_path: &Path, preview: &str) -> f32 {
    let mut alignment = 1.0;
    let query_lower = query.to_lowercase();
    let path_str = file_path.to_str().unwrap_or("").to_lowercase();
    
    // Extract query keywords
    let keywords: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|w| w.len() > 3)  // Skip short words
        .collect();
    
    // Check if filename contains query keywords
    for keyword in &keywords {
        if path_str.contains(keyword) {
            alignment *= 1.4;  // Filename match is strong signal
        }
    }
    
    // Check parent directory names
    let parent_names = file_path.parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    
    for keyword in &keywords {
        if parent_names.contains(keyword) {
            alignment *= 1.2;  // Directory match is good signal
        }
    }
    
    // Keyword density in preview
    let preview_lower = preview.to_lowercase();
    let keyword_count = keywords.iter()
        .filter(|k| preview_lower.contains(*k))
        .count();
    
    let density_boost = 1.0 + (keyword_count as f32 * 0.1);
    alignment *= density_boost.min(1.5);  // Cap at 1.5x
    
    alignment
}
```

**Expected Impact**:
- Precision@3: +8-12%
- MRR: +0.08-0.12

---

### 🎯 Priority 3: Result Diversity

**Problem**: Top 10 results often from same directory/file

**Solution**: Apply diversity penalty to similar results

```rust
fn apply_diversity_scoring(results: &mut [SearchResult]) {
    let mut seen_directories: HashMap<String, usize> = HashMap::new();
    let mut seen_filenames: HashMap<String, usize> = HashMap::new();
    
    for result in results.iter_mut() {
        let parent = result.file_path.parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        
        let filename = result.file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        // Penalize repeated directories
        let dir_count = seen_directories.entry(parent.to_string()).or_insert(0);
        *dir_count += 1;
        if *dir_count > 2 {
            result.score *= 0.8;  // Penalize 3rd+ file from same dir
        }
        
        // Penalize repeated filenames (e.g., multiple mod.rs)
        let file_count = seen_filenames.entry(filename.to_string()).or_insert(0);
        *file_count += 1;
        if *file_count > 1 {
            result.score *= 0.85;  // Penalize duplicates
        }
    }
}
```

**Expected Impact**:
- Precision@10: +5-10%
- Recall@10: +3-5%

---

### 🎯 Priority 4: Adaptive Documentation Penalty

**Problem**: 0.1x penalty too weak for README, too strong for inline docs

**Solution**: Context-aware documentation penalty

```rust
fn calculate_documentation_penalty(path: &Path, preview: &str) -> f32 {
    let path_str = path.to_str().unwrap_or("");
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    
    // Strong penalty for README and spec files
    if filename.eq_ignore_ascii_case("readme.md") {
        return 0.05;  // Very heavy penalty
    }
    
    if filename.contains("spec.md") || filename.contains("SPEC") {
        return 0.03;  // Extra heavy for specs
    }
    
    // Check if doc has code examples
    let has_code_blocks = preview.contains("```rust") 
        || preview.contains("```python")
        || preview.contains("```js");
    
    if has_code_blocks {
        return 0.3;  // Lighter penalty for docs with code
    }
    
    // Check if it's API documentation
    if preview.contains("## API") || preview.contains("### Methods") {
        return 0.4;  // API docs are more useful
    }
    
    0.1  // Default doc penalty
}
```

**Expected Impact**:
- Precision@3: +5-8%
- MRR: +0.05-0.08

---

### 🎯 Priority 5: Boost Critical Patterns

**Problem**: Critical implementation files not recognized

**Solution**: Pattern-based importance boost

```rust
fn calculate_pattern_boost(path: &Path, preview: &str) -> f32 {
    let mut boost = 1.0;
    let preview_lower = preview.to_lowercase();
    
    // Implementation pattern detection
    let has_impl = preview.contains("impl ") || preview.contains("impl<");
    let has_trait = preview.contains("trait ");
    let has_struct = preview.contains("pub struct") || preview.contains("pub enum");
    let has_main_fn = preview.contains("fn main(") || preview.contains("pub fn new(");
    
    if has_impl && has_struct {
        boost *= 1.3;  // Core implementation file
    }
    
    if has_trait {
        boost *= 1.2;  // Trait definitions are important
    }
    
    if has_main_fn {
        boost *= 1.25;  // Entry point functions
    }
    
    // Count pub items (public API)
    let pub_count = preview.matches("pub fn").count() 
        + preview.matches("pub struct").count()
        + preview.matches("pub enum").count();
    
    if pub_count > 5 {
        boost *= 1.2;  // Rich public API
    }
    
    // Module-level documentation at start
    if preview.trim_start().starts_with("//!") {
        boost *= 1.15;  // Module docs indicate important file
    }
    
    boost
}
```

**Expected Impact**:
- Critical in Top-3: +15-20%
- Precision@3: +5-8%

---

### 🎯 Priority 6: Two-Stage Ranking

**Problem**: Single-pass ranking misses nuanced relevance

**Solution**: Coarse-to-fine ranking approach

```rust
async fn two_stage_ranking(
    &self,
    query: &str,
    initial_results: Vec<SearchResult>,
    top_k: usize,
) -> Vec<SearchResult> {
    // Stage 1: Get top 50 candidates (current approach)
    let mut candidates = initial_results;
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    let top_candidates = candidates.into_iter().take(50).collect::<Vec<_>>();
    
    // Stage 2: Re-rank top 50 with expensive features
    let mut reranked = Vec::new();
    
    for mut result in top_candidates {
        let mut rerank_score = result.score;
        
        // Add file importance signal
        let importance = self.calculate_file_importance(&result.file_path);
        rerank_score *= importance;
        
        // Add query-file alignment
        let alignment = self.calculate_query_file_alignment(
            query,
            &result.file_path,
            &result.preview
        );
        rerank_score *= alignment;
        
        // Add pattern-based boost
        let pattern_boost = self.calculate_pattern_boost(&result.file_path, &result.preview);
        rerank_score *= pattern_boost;
        
        result.score = rerank_score;
        reranked.push(result);
    }
    
    // Apply diversity
    self.apply_diversity_scoring(&mut reranked);
    
    // Final sort
    reranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    reranked.into_iter().take(top_k).collect()
}
```

**Expected Impact**:
- All metrics: +5-10% improvement
- Better utilization of multiple signals

---

### 🎯 Priority 7: Learned Query Patterns

**Problem**: System doesn't learn from query patterns

**Solution**: Simple query type classifier with tuned parameters

```rust
#[derive(Debug, Clone, Copy)]
enum QueryType {
    BugFix,          // "fix", "bug", "error", "issue"
    Implementation,  // "implement", "add", "create"
    Explanation,     // "how", "what", "why"
    Location,        // "where", "which", "find"
    Optimization,    // "optimize", "improve", "faster"
}

impl QueryType {
    fn from_query(query: &str) -> Self {
        let q = query.to_lowercase();
        
        if q.starts_with("fix ") || q.contains(" bug") || q.contains("error") {
            QueryType::BugFix
        } else if q.starts_with("implement") || q.starts_with("add ") || q.starts_with("create") {
            QueryType::Implementation
        } else if q.starts_with("how ") || q.starts_with("what ") || q.starts_with("why ") {
            QueryType::Explanation
        } else if q.starts_with("where ") || q.starts_with("which ") || q.contains("find") {
            QueryType::Location
        } else if q.contains("optimize") || q.contains("improve") || q.contains("faster") {
            QueryType::Optimization
        } else {
            QueryType::Location  // Default
        }
    }
    
    fn get_weights(&self) -> (f32, f32) {
        match self {
            QueryType::BugFix => (0.7, 0.3),        // Heavy BM25 for exact matching
            QueryType::Implementation => (0.6, 0.4), // Favor keywords slightly
            QueryType::Explanation => (0.3, 0.7),    // Heavy semantic
            QueryType::Location => (0.7, 0.3),       // Heavy BM25 for finding
            QueryType::Optimization => (0.5, 0.5),   // Balanced
        }
    }
    
    fn should_boost_tests(&self) -> bool {
        matches!(self, QueryType::BugFix)  // Bug fixes may need test files
    }
    
    fn should_boost_docs(&self) -> bool {
        matches!(self, QueryType::Explanation)  // Explanations can use docs
    }
}
```

**Expected Impact**:
- MRR: +0.08-0.12
- Query-specific performance boost

---

## Implementation Priority

### Phase 4: Quick Wins (1-2 hours)
1. **Adaptive Documentation Penalty** - Easy, high impact
2. **Pattern Boost** - Simple regex checks, good ROI
3. **Query-File Alignment** - Straightforward path matching

**Expected After Phase 4**:
- Precision@3: 25% → 33-35%
- MRR: 0.479 → 0.530-0.550
- Critical in Top-3: 22.5% → 32-37%

### Phase 5: Medium Effort (3-4 hours)
4. **File Importance Signals** - Needs file system access
5. **Result Diversity** - Requires tracking seen items
6. **Improved Query Type Classification**

**Expected After Phase 5**:
- Precision@3: 33-35% → 38-42%
- Recall@10: 49.2% → 55-58%
- MRR: 0.530-0.550 → 0.600-0.630

### Phase 6: Advanced (4-6 hours)
7. **Two-Stage Ranking** - Architectural change
8. **Per-Query Threshold Tuning**
9. **Machine Learning Features** (if needed)

**Expected After Phase 6**:
- All metrics at or above target
- Precision@3: 40%+
- Recall@10: 60%+
- MRR: 0.650+

---

## Testing Strategy

After each phase:
1. Run full 20-test benchmark suite
2. Compare against baseline (Phase 2)
3. Check for regressions in any category
4. Document which improvements helped which query types

---

## Risk Mitigation

### Avoid Over-Fitting
- Test on diverse query types
- Don't tune for specific test cases
- Keep parameters general

### Performance Considerations
- Cache file importance scores
- Limit two-stage ranking to top-K only
- Profile slow operations

### Maintain Simplicity
- Start with simplest solutions
- Add complexity only if needed
- Keep code maintainable

---

## Long-Term Improvements

### Beyond Target Metrics
1. **Learning from User Feedback**
   - Track which results users select
   - Adjust weights based on implicit feedback

2. **Repository Structure Learning**
   - Learn common patterns per project
   - Adapt to monorepo vs single-crate structures

3. **Temporal Relevance**
   - Boost recently modified files
   - Consider git commit frequency

4. **Cross-File Relationship**
   - Use import graphs for relevance
   - Boost files that are imported by many others

---

## Summary

**Current Best**: Phase 2 with threshold 0.75
**Next Steps**: Implement Phase 4 quick wins
**Expected Timeline**: 2-3 weeks to hit all targets
**Highest Impact**: Query-file alignment + adaptive doc penalties
