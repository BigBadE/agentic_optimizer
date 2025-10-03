# Benchmark Setup Guide

## Quick Setup

The Valor repository is already set up in `benchmarks/test_repositories/valor/`.

```bash
# Run all benchmarks
cargo run --bin benchmark

# Generate report
cargo run --bin benchmark --report benchmarks/results.md
```

## Test Repository

**Valor Browser Engine** - Pinned to commit `367ecde76cfe1a587256f9c6f318a56afee5ac17`

If you need to re-clone:
```bash
git clone https://github.com/BigBadE/Valor.git benchmarks/test_repositories/valor
cd benchmarks/test_repositories/valor
git reset --hard 367ecde76cfe1a587256f9c6f318a56afee5ac17
```

## Test Cases (20 Total)

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

## Test Case Format

Each `.toml` file defines a test:

```toml
name = "CSS Parsing Implementation"
description = "Query about CSS parsing"
query = "how does CSS parsing work"
project_root = "benchmarks/test_repositories/valor"

[[expected]]
path = "crates/css/src/parser.rs"
priority = "critical"  # critical, high, medium, low
reason = "Main CSS parser"

[[excluded]]
path = "crates/renderer"
reason = "Rendering, not parsing"
```

**Priority Levels**:
- **critical**: Must be in top 3 (weight: 1.0)
- **high**: Should be in top 5 (weight: 0.8)
- **medium**: Should be in top 10 (weight: 0.5)
- **low**: Nice to have in top 20 (weight: 0.2)

## Workflow

1. **Baseline**: `cargo run --bin benchmark --report baseline.md`
2. **Make changes** in `crates/agentic-context/src/embedding/`
3. **Clear cache** (if needed): `rm -rf benchmarks/test_repositories/valor/.agentic_cache`
4. **Re-run**: `cargo run --bin benchmark --report after.md`
5. **Compare**: Check if metrics improved
6. **Document** in `CONTEXT_IMPROVEMENT.md`

## Troubleshooting

**Slow first run?** Building embeddings takes 5-10 minutes initially. Subsequent runs use cache.

**Wrong results?** Clear cache: `rm -rf benchmarks/test_repositories/valor/.agentic_cache`

**Missing test cases?** Ensure `.toml` files exist in `benchmarks/test_cases/valor/`
