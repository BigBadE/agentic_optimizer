# Running Benchmarks Locally

This guide covers both quality benchmarks and performance benchmarks.

## Performance Benchmarks

Performance benchmarks measure the speed and efficiency of core routing operations. We support two types:

1. **Criterion Benchmarks** - Statistical benchmarks with warm-up, multiple iterations, and HTML reports
2. **IAI Benchmarks** - Single-shot benchmarks using Cachegrind for precise, deterministic measurements

### Running Criterion Benchmarks

```bash
# Run all Criterion benchmarks and save results
cargo run --release --bin perf-bench -- --output perf-results.md

# Run with verbose output
cargo run --release --bin perf-bench -- --output perf-results.md --verbose

# Run specific benchmark
cargo run --release --bin perf-bench -- --output perf-results.md --name "request_analysis"
```

### Running IAI Benchmarks

**Prerequisites**: IAI requires [Valgrind](https://www.valgrind.org) to be installed. IAI is not available on Windows.

**IAI benchmarks run automatically in CI** on every push and pull request. You can also run them locally:

```bash
# Run all IAI benchmarks and save results
cargo run --release --bin perf-bench -- --iai --output iai-results.md

# Run with verbose output
cargo run --release --bin perf-bench -- --iai --output iai-results.md --verbose

# Run specific IAI benchmark
cargo run --release --bin perf-bench -- --iai --output iai-results.md --name "iai_routing"
```

**IAI Benefits**:
- **Precision**: Detects very small performance changes
- **Consistency**: Works reliably in CI environments
- **Profiling**: Generates Cachegrind profiles for detailed analysis
- **Speed**: Faster than statistical benchmarks (single execution)

### Committing Performance Results

**For Criterion benchmarks** (manual upload):
1. Run benchmarks locally
2. Review the generated `perf-results.md`
3. Force-add it (it's in .gitignore) and commit:
   ```bash
   git add -f perf-results.md
   git commit -m "Update performance benchmark results"
   git push
   ```
4. CI will publish results to gh-pages and remove the file from repo

**For IAI benchmarks** (automatic in CI):
- IAI benchmarks run automatically in CI on every push/PR
- Results are published to gh-pages automatically
- No manual upload needed

## Quality Benchmarks

Quality benchmarks require **Ollama** to be running locally because they use the actual context retrieval system with embeddings and BM25 search.

### Prerequisites for Quality Benchmarks

1. **Install Ollama**: https://ollama.ai/
2. **Start Ollama**: `ollama serve`
3. **Pull required model**: `ollama pull nomic-embed-text`

### Running Quality Benchmarks

#### Quick Run

```bash
cargo run --release --bin quality-bench -- --output quality-results.md
```

#### With Verbose Output

```bash
cargo run --release --bin quality-bench -- --output quality-results.md --verbose
```

#### Run Specific Test Case

```bash
cargo run --release --bin quality-bench -- --output quality-results.md --name "CSS Parsing"
```

### Test Repositories

Quality benchmarks run against test repositories in `benchmarks/test_repositories/`. The main test repository is **Valor Browser Engine**.

#### Setting Up Test Repositories

```bash
# Clone Valor (if not already present)
git clone https://github.com/BigBadE/Valor.git benchmarks/test_repositories/valor

# Pin to specific commit for reproducibility
cd benchmarks/test_repositories/valor
git reset --hard 367ecde76cfe1a587256f9c6f318a56afee5ac17
```

### Committing Quality Results

After running quality benchmarks locally:

1. Review the generated `quality-results.md`
2. Force-add it (it's in .gitignore) and commit:
   ```bash
   git add -f quality-results.md
   git commit -m "Update quality benchmark results"
   git push
   ```
3. CI will automatically:
   - Publish results to gh-pages
   - Remove `quality-results.md` from the repo (keeps repo clean)

### CI Workflow

**Three separate workflows**:

1. **`quality_benchmarks.yml`** - Quality benchmark results (manual upload)
   - Checks for uploaded `quality-results.md`
   - Publishes to gh-pages
   - Removes file from repo
   - *Reason*: Requires Ollama (not available on GitHub Actions)

2. **`benchmark.yml`** - Criterion performance benchmarks (manual upload)
   - Checks for uploaded `perf-results.md`
   - Publishes to gh-pages
   - Removes file from repo
   - *Reason*: Statistical benchmarks take time; run locally for consistency

3. **`iai_benchmarks.yml`** - IAI benchmarks (automatic execution)
   - **Runs automatically** on every push/PR
   - Installs Valgrind on Ubuntu runner
   - Executes IAI benchmarks
   - Publishes results to gh-pages
   - Uploads artifacts
   - *Reason*: Fast single-shot benchmarks; deterministic in CI

## Understanding Quality Benchmark Results

### Quality Metrics

- **Precision@3**: % of top 3 results that are relevant
- **Precision@10**: % of top 10 results that are relevant
- **Recall@10**: % of relevant files found in top 10
- **MRR**: Mean Reciprocal Rank (1/rank of first relevant result)
- **NDCG@10**: Normalized Discounted Cumulative Gain (quality of ranking)
- **Critical in Top-3**: % of critical files appearing in top 3

### Targets

| Metric | Target |
|--------|--------|
| Precision@3 | 60% |
| Precision@10 | 55% |
| Recall@10 | 70% |
| MRR | 0.700 |
| NDCG@10 | 0.750 |
| Critical in Top-3 | 65% |

## Adding New Test Cases

1. Create a `.toml` file in `benchmarks/test_cases/`
2. Define expected files with priorities:
   ```toml
   name = "My Test Case"
   description = "What this tests"
   query = "the search query"
   project_root = "benchmarks/test_repositories/valor"

   [[expected]]
   path = "crates/module/src/lib.rs"
   priority = "critical"
   reason = "Why this file is relevant"
   ```
3. Run benchmarks to see results
4. Adjust test case if needed

## Troubleshooting

### "Cannot start a runtime from within a runtime"

This error means you're trying to run benchmarks from within an async context. Always run the CLI binary directly, not from another async function.

### "No test cases found"

Check that:
- `benchmarks/test_cases/` exists
- It contains `.toml` files
- TOML files are valid

### "Project root does not exist"

The test repository hasn't been cloned. See "Setting Up Test Repositories" above.

### Poor Results (all 0%)

This usually means:
- Ollama isn't running
- The embedding model isn't pulled
- The test repository path is wrong
- Expected file paths in test cases don't match actual repository structure
