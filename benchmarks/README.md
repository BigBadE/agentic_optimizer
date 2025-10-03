# Context Fetching Benchmarks

This directory contains benchmarks for evaluating the quality of context retrieval.

## Structure

```
benchmarks/
├── README.md                    # This file
├── USAGE.md                     # Detailed usage guide
├── SUMMARY.md                   # Overview and best practices
├── test_cases/                  # Test cases for different projects
│   └── valor/                   # Test cases for Valor browser engine
│       ├── css_parsing.toml
│       ├── rendering_pipeline.toml
│       ├── dom_tree.toml
│       └── layout_engine.toml
└── test_repositories/           # External projects for testing
    └── valor/                   # Valor browser engine (pinned commit)
```

## Test Case Format

Each test case is a TOML file with the following structure:

```toml
# Test metadata
name = "CSS Parsing Implementation"
description = "Query about how CSS is parsed in the browser engine"
query = "how does CSS parsing work"
project_root = "benchmarks/test_repositories/valor"  # Optional: override default project

# Expected relevant files (in priority order)
[[expected]]
path = "crates/css/src/parser.rs"
priority = "critical"  # critical, high, medium, low
reason = "Main CSS parser implementation"

[[expected]]
path = "crates/css/src/tokenizer.rs"
priority = "critical"
reason = "CSS tokenization logic"

[[expected]]
path = "crates/css/orchestrator/src/lib.rs"
priority = "high"
reason = "CSS orchestration and coordination"

# Files that should NOT appear in results
[[excluded]]
path = "crates/renderer/wgpu_backend"
reason = "Rendering backend, not parsing"

[[excluded]]
path = "crates/html"
reason = "HTML parsing, different domain"
```

## Priority Levels

- **critical**: Must be in top 3 results (weight: 1.0)
- **high**: Should be in top 5 results (weight: 0.8)
- **medium**: Should be in top 10 results (weight: 0.5)
- **low**: Nice to have in top 20 (weight: 0.2)

## Metrics

The benchmark calculates:

1. **Precision@K**: Percentage of top-K results that are relevant
2. **Recall@K**: Percentage of relevant files found in top-K
3. **Mean Reciprocal Rank (MRR)**: Average of 1/rank for first relevant result
4. **Normalized Discounted Cumulative Gain (NDCG)**: Quality-weighted ranking metric
5. **Exclusion Rate**: Percentage of excluded files that don't appear in top-20

## Running Benchmarks

```bash
# Run all benchmarks for Valor project (default)
cargo run --bin benchmark

# Run specific test case
cargo run --bin benchmark --test css_parsing

# Run with detailed output
cargo run --bin benchmark --verbose

# Generate report
cargo run --bin benchmark --report benchmarks/report.md

# Use different project
cargo run --bin benchmark --project other_project
```

## Adding New Test Cases

1. Create a new `.toml` file in `test_cases/<project>/`
2. Define the query and expected files
3. Run the benchmark to validate
4. Iterate on context fetching improvements
5. Re-run to measure progress

## Example Output

```
Running benchmark: CSS Parsing Implementation
Query: "how does CSS parsing work"

Results:
  Precision@3:  66.7% (2/3 critical files found)
  Precision@5:  80.0% (4/5 high-priority files found)
  Precision@10: 60.0% (6/10 relevant files found)
  Recall@10:    85.7% (6/7 expected files found)
  MRR:          0.500 (first relevant at rank #2)
  NDCG@10:      0.765
  Exclusion:    100.0% (0/3 excluded files in top-20)

Top 10 Results:
  1. ✅ crates/css/src/parser.rs (expected: critical)
  2. ✅ crates/css/src/tokenizer.rs (expected: critical)
  3. ❌ crates/css/src/lib.rs (not expected)
  4. ✅ crates/css/orchestrator/src/lib.rs (expected: high)
  5. ✅ crates/css/modules/... (expected: medium)
  ...
```
