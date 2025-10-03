# Benchmark Usage Guide

## Quick Start

```bash
# Run all benchmarks for the current project
cargo run --bin benchmark

# Run a specific test case
cargo run --bin benchmark --test cli_argument

# Generate a detailed report
cargo run --bin benchmark --report benchmarks/report.md

# Verbose output
cargo run --bin benchmark --verbose
```

## Creating New Test Cases

1. **Create a TOML file** in `benchmarks/test_cases/agentic_optimizer/`:

```toml
name = "Your Test Name"
description = "What this test evaluates"
query = "your search query here"

[[expected]]
path = "path/to/expected/file.rs"
priority = "critical"  # critical, high, medium, or low
reason = "Why this file should be found"

[[excluded]]
path = "path/to/excluded/file.rs"
reason = "Why this file should NOT appear"
```

2. **Run the benchmark**:
```bash
cargo run --bin benchmark --test your_test_name
```

3. **Analyze results** and iterate on improvements

## Understanding Metrics

### Precision@K
**What it measures**: Of the top K results, how many are relevant?

**Formula**: `relevant_in_top_k / k`

**Example**: If 2 out of top 3 results are expected files → Precision@3 = 66.7%

**Good score**: >70% for Precision@3

### Recall@K
**What it measures**: Of all relevant files, how many did we find in top K?

**Formula**: `found_in_top_k / total_expected`

**Example**: If we found 5 out of 7 expected files in top 10 → Recall@10 = 71.4%

**Good score**: >80% for Recall@10

### Mean Reciprocal Rank (MRR)
**What it measures**: How quickly do we find the first relevant result?

**Formula**: `1 / rank_of_first_relevant`

**Example**: First relevant file at rank #2 → MRR = 0.500

**Good score**: >0.7 (first relevant in top 2)

### NDCG@10 (Normalized Discounted Cumulative Gain)
**What it measures**: Quality-weighted ranking (critical files should rank higher)

**How it works**:
- Critical files get weight 1.0
- High priority gets 0.8
- Medium gets 0.5
- Low gets 0.2
- Discounts by position (later = less value)

**Good score**: >0.8

### Exclusion Rate
**What it measures**: How well we avoid irrelevant files

**Formula**: `1 - (excluded_found_in_top_20 / total_excluded)`

**Example**: 0 excluded files in top 20 → Exclusion = 100%

**Good score**: >90%

### Critical in Top-3
**What it measures**: Are critical files in the top 3 results?

**Good score**: 100% (all critical files in top 3)

### High in Top-5
**What it measures**: Are high-priority files in top 5?

**Good score**: >80%

## Interpreting Results

### Example Output

```
# Benchmark: CLI Argument Cleanup

**Query**: "clean up the --prompt argument"

## Metrics

- **Precision@3**:  66.7%     ← 2/3 top results are relevant
- **Precision@5**:  60.0%     ← 3/5 top results are relevant  
- **Recall@10**:    71.4%     ← Found 5/7 expected files
- **MRR**:          0.500     ← First relevant at rank #2
- **NDCG@10**:      0.723     ← Good quality-weighted ranking
- **Exclusion**:    100.0%    ← No excluded files in top 20
- **Critical in Top-3**: 50.0% ← Only 1/2 critical files in top 3

## Top 10 Results

1. config/mod.rs ✅ (expected: medium)
2. cli.rs ✅ (expected: critical) ⚠️ Should be #1
3. fs_utils.rs ❌ (not expected)
...
```

### What to Look For

**🔴 Red Flags**:
- Precision@3 < 50% → Too much noise in top results
- Critical in Top-3 < 100% → Missing critical files
- Exclusion < 80% → Irrelevant files appearing
- MRR < 0.5 → First relevant file too far down

**🟡 Needs Improvement**:
- Precision@5 < 70%
- Recall@10 < 80%
- NDCG@10 < 0.7

**🟢 Good Performance**:
- Precision@3 > 70%
- Recall@10 > 80%
- Critical in Top-3 = 100%
- Exclusion > 90%
- NDCG@10 > 0.8

## Workflow for Improvements

1. **Run baseline benchmark**:
   ```bash
   cargo run --bin benchmark --report benchmarks/baseline.md
   ```

2. **Implement improvement** (e.g., better BM25 weighting)

3. **Re-run benchmark**:
   ```bash
   cargo run --bin benchmark --report benchmarks/after_improvement.md
   ```

4. **Compare metrics**:
   ```bash
   diff benchmarks/baseline.md benchmarks/after_improvement.md
   ```

5. **Iterate** until metrics meet targets

## Adding Test Cases for New Projects

1. Create directory: `benchmarks/test_cases/your_project/`

2. Add test cases as TOML files

3. Run benchmarks:
   ```bash
   cargo run --bin benchmark --project your_project
   ```

## Tips for Writing Good Test Cases

### DO:
- ✅ Include 3-7 expected files with varied priorities
- ✅ Add 2-5 excluded files that might appear but shouldn't
- ✅ Write realistic queries users would actually ask
- ✅ Explain WHY each file is expected/excluded
- ✅ Cover different query types (exact match, conceptual, task-based)

### DON'T:
- ❌ Include too many expected files (makes metrics noisy)
- ❌ Use overly specific queries that only match one file
- ❌ Forget to specify priorities (critical vs low)
- ❌ Leave out exclusions (can't measure false positives)

## Example Test Suite

A good test suite should cover:

1. **Exact Match Queries**
   - "fix the --prompt argument"
   - "UserService::find_by_email implementation"

2. **Conceptual Queries**
   - "how does vector search work"
   - "authentication implementation"

3. **Task-Based Queries**
   - "add logging to API calls"
   - "implement caching for embeddings"

4. **Navigation Queries**
   - "where is the config loaded"
   - "find the CLI argument parsing"

5. **Debug Queries**
   - "why is the search slow"
   - "fix the ranking algorithm"
