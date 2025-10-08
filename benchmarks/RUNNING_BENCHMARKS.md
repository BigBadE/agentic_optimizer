# Running Quality Benchmarks Locally

Quality benchmarks require **Ollama** to be running locally because they use the actual context retrieval system with embeddings and BM25 search.

## Prerequisites

1. **Install Ollama**: https://ollama.ai/
2. **Start Ollama**: `ollama serve`
3. **Pull required model**: `ollama pull nomic-embed-text`

## Running Benchmarks

### Quick Run

```bash
cargo run --release --bin quality-bench -- --output quality-results.md
```

### With Verbose Output

```bash
cargo run --release --bin quality-bench -- --output quality-results.md --verbose
```

### Run Specific Test Case

```bash
cargo run --release --bin quality-bench -- --output quality-results.md --name "CSS Parsing"
```

## Test Repositories

Benchmarks run against test repositories in `benchmarks/test_repositories/`. The main test repository is **Valor Browser Engine**.

### Setting Up Test Repositories

```bash
# Clone Valor (if not already present)
git clone https://github.com/BigBadE/Valor.git benchmarks/test_repositories/valor

# Pin to specific commit for reproducibility
cd benchmarks/test_repositories/valor
git reset --hard 367ecde76cfe1a587256f9c6f318a56afee5ac17
```

## Committing Results

After running benchmarks locally:

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

## CI Workflow

The CI workflow **does not run benchmarks**. It:
1. Checks that `quality-results.md` exists
2. Publishes it to gh-pages for historical tracking
3. Uploads it as an artifact
4. Removes `quality-results.md` from the repo (keeps repo clean)

This is because:
- Benchmarks require Ollama (not available on GitHub Actions runners)
- Benchmarks can take several minutes to run
- Results should be reviewed before committing
- Results are stored on gh-pages, not in the main repo

## Understanding Results

### Metrics

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
