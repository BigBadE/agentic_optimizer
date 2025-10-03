# Benchmark Setup Guide

This guide explains how to set up and use the context fetching benchmark system.

## Quick Setup

The benchmark system uses the **Valor browser engine** as an external test repository to avoid being tied to this project's structure.

### Automatic Setup

The Valor repository is already cloned in `benchmarks/test_repositories/valor/` and pinned to commit `367ecde76cfe1a587256f9c6f318a56afee5ac17`.

Just run:
```bash
cargo run --bin benchmark
```

### Manual Setup (if needed)

If the repository is missing:

```bash
# Clone Valor
git clone https://github.com/BigBadE/Valor.git benchmarks/test_repositories/valor

# Pin to specific commit
cd benchmarks/test_repositories/valor
git reset --hard 367ecde76cfe1a587256f9c6f318a56afee5ac17
```

## Running Benchmarks

### Basic Usage

```bash
# Run all test cases
cargo run --bin benchmark

# Run specific test
cargo run --bin benchmark --test css_parsing

# Verbose output
cargo run --bin benchmark --verbose

# Generate markdown report
cargo run --bin benchmark --report benchmarks/report.md
```

### Understanding Output

```
Running: CSS Parsing Implementation
═══════════════════════════════════════════════════════════

# Benchmark: CSS Parsing Implementation

**Query**: "how does CSS parsing work"

## Metrics

- **Precision@3**:  66.7%    ← 2/3 top results are relevant
- **Precision@5**:  80.0%    ← 4/5 top results are relevant
- **Recall@10**:    85.7%    ← Found 6/7 expected files
- **MRR**:          0.500    ← First relevant at rank #2
- **NDCG@10**:      0.765    ← Quality-weighted ranking
- **Exclusion**:    100.0%   ← No excluded files in top 20
- **Critical in Top-3**: 66.7%  ← 2/3 critical files in top 3

## Top 10 Results

1. crates/css/src/parser.rs ✅ (expected: critical) (score: 0.950)
2. crates/css/src/tokenizer.rs ✅ (expected: critical) (score: 0.920)
3. crates/css/src/lib.rs ❌ (not expected) (score: 0.850)
...
```

## Test Cases

### Current Test Cases for Valor

1. **css_parsing.toml** - "how does CSS parsing work"
   - Tests: Conceptual queries about implementation
   - Expected: parser.rs, tokenizer.rs, orchestrator

2. **rendering_pipeline.toml** - "fix the rendering pipeline to handle text layout"
   - Tests: Task-based queries with specific features
   - Expected: wgpu_backend, pipeline.rs, text.rs

3. **dom_tree.toml** - "where is the DOM tree built and modified"
   - Tests: Navigation queries about architecture
   - Expected: tree_builder.rs, dom.rs, parser.rs

4. **layout_engine.toml** - "implement flexbox layout algorithm"
   - Tests: Implementation queries for specific algorithms
   - Expected: layout.rs, box_model.rs, flexbox.rs

## Creating New Test Cases

### 1. Understand the Project Structure

```bash
# Explore the test repository
ls benchmarks/test_repositories/valor/crates/
```

### 2. Create Test Case File

Create `benchmarks/test_cases/valor/my_test.toml`:

```toml
name = "My Test Name"
description = "What this test evaluates"
query = "your search query"
project_root = "benchmarks/test_repositories/valor"

[[expected]]
path = "crates/some/file.rs"
priority = "critical"
reason = "Why this file is critical"

[[excluded]]
path = "crates/other/file.rs"
reason = "Why this shouldn't appear"
```

### 3. Run and Validate

```bash
cargo run --bin benchmark --test my_test --verbose
```

### 4. Adjust Expectations

Based on results, refine your expected/excluded files and priorities.

## Workflow for Improvements

### 1. Establish Baseline

```bash
# Run all benchmarks and save results
cargo run --bin benchmark --report benchmarks/baseline.md
```

### 2. Implement Improvement

Edit code in:
- `crates/agentic-context/src/embedding/bm25.rs` (BM25 search)
- `crates/agentic-context/src/embedding/vector_search.rs` (Vector search & fusion)
- `crates/agentic-context/src/builder.rs` (Context building)

### 3. Rebuild Index (if needed)

If you changed tokenization or scoring:
```bash
# Clear cache for test repository
rm -rf benchmarks/test_repositories/valor/.agentic_cache

# Or just delete the entire cache
rm -rf .agentic_cache
```

### 4. Re-run Benchmarks

```bash
cargo run --bin benchmark --report benchmarks/after_change.md
```

### 5. Compare Results

```bash
# Manual diff
diff benchmarks/baseline.md benchmarks/after_change.md

# Or just look at summary metrics
cargo run --bin benchmark
```

### 6. Document Changes

Update `CONTEXT_IMPROVEMENT.md` with:
- What you changed
- Before/after metrics
- Analysis of improvements

## Metrics Targets

| Metric | Current | Target | Priority |
|--------|---------|--------|----------|
| Precision@3 | ~40% | 70% | 🔴 Critical |
| Recall@10 | ~70% | 80% | 🟡 High |
| MRR | ~0.5 | 0.7 | 🟡 High |
| NDCG@10 | ~0.7 | 0.8 | 🟡 Medium |
| Critical in Top-3 | ~50% | 100% | 🔴 Critical |
| Exclusion | ~80% | 90% | 🟡 Medium |

## Troubleshooting

### "Test cases directory not found"

The test repository might be missing. Clone it:
```bash
git clone https://github.com/BigBadE/Valor.git benchmarks/test_repositories/valor
cd benchmarks/test_repositories/valor
git reset --hard 367ecde76cfe1a587256f9c6f318a56afee5ac17
```

### "No test cases found"

Make sure `.toml` files exist in `benchmarks/test_cases/valor/`:
```bash
ls benchmarks/test_cases/valor/
```

### Slow initialization

The first run needs to:
1. Build embeddings for all files (can take 5-10 minutes)
2. Build BM25 index
3. Cache results

Subsequent runs use the cache and are much faster.

### Cache issues

If results seem wrong, try clearing the cache:
```bash
rm -rf benchmarks/test_repositories/valor/.agentic_cache
```

## Adding New Test Repositories

To benchmark against a different project:

1. **Clone repository**:
   ```bash
   git clone <url> benchmarks/test_repositories/<name>
   cd benchmarks/test_repositories/<name>
   git reset --hard <commit>
   ```

2. **Document pin**:
   Create `.git-pin` file with commit info

3. **Create test cases**:
   ```bash
   mkdir benchmarks/test_cases/<name>
   # Add .toml files
   ```

4. **Run benchmarks**:
   ```bash
   cargo run --bin benchmark --project <name>
   ```

## Best Practices

### ✅ DO
- Run benchmarks before and after changes
- Use realistic queries users would ask
- Document why files are expected/excluded
- Cover different query types (exact, conceptual, task-based)
- Pin test repositories to specific commits

### ❌ DON'T
- Test against this project's own structure
- Use synthetic queries
- Skip benchmarking "small" changes
- Ignore failing test cases
- Optimize for one query at expense of others

## Next Steps

1. **Run initial benchmark**: `cargo run --bin benchmark`
2. **Review results**: Check which metrics need improvement
3. **Implement fixes**: See `CONTEXT_IMPROVEMENT.md` for ideas
4. **Re-benchmark**: Measure improvement
5. **Iterate**: Keep improving until targets met

For more details, see:
- `README.md` - Format specification
- `USAGE.md` - Detailed metrics explanation
- `SUMMARY.md` - Overview and best practices
