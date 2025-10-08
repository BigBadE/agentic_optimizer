# Merlin Performance Benchmarking

**Status**: ✅ Implemented with continuous tracking

## Overview

Merlin uses [Criterion.rs](https://github.com/bheisler/criterion.rs) for performance benchmarking with:
- **Historical tracking** - Compare against previous runs
- **Regression detection** - Automatic alerts on 25%+ slowdowns
- **Trend visualization** - Charts showing performance over time
- **CI integration** - Benchmarks run on every commit to master

## Quick Start

### Run All Benchmarks

```bash
# Run all benchmarks with default settings
cargo bench --workspace

# Run benchmarks with more samples (slower, more accurate)
cargo bench --workspace -- --sample-size 100

# Run specific benchmark group
cargo bench --bench routing_benchmarks

# Run specific test within a group
cargo bench request_analysis
```

### View Results

Benchmark results are saved to:
- **HTML Reports**: `target/criterion/report/index.html`
- **Raw Data**: `target/criterion/<benchmark_name>/`
- **Comparison**: Results automatically compared to previous runs

```bash
# Open HTML report (Linux/macOS)
open target/criterion/report/index.html

# Windows
start target/criterion/report/index.html
```

## Benchmark Suites

### 1. Request Analysis (`request_analysis`)

Tests the performance of analyzing incoming requests:

**Tests**:
- Simple requests (e.g., "Add a comment")
- Medium complexity (e.g., "Refactor parser with error handling")
- Complex requests (e.g., "Create OAuth2 authentication system")

**Metrics**: Time to analyze and extract intent/complexity

**Expected Performance**:
- Simple: <50ms
- Medium: <200ms
- Complex: <500ms

### 2. Task Decomposition (`task_decomposition`)

Tests breaking down complex requests into subtasks:

**Tests**:
- Single-task requests
- Multi-step workflows
- Large refactoring operations

**Expected Performance**: <100ms per decomposition

### 3. Tier Selection (`tier_selection`)

Tests the routing strategy selection speed:

**Tests**:
- Simple → Local tier
- Medium → Groq tier
- Complex → Premium tier

**Expected Performance**: <1ms (should be nearly instant)

### 4. Context Building (`context_building`)

Tests the speed of gathering relevant code files:

**Tests**:
- Query: "routing implementation"
- Query: "task manager"
- Query: "validation pipeline"

**Expected Performance**: <2s for 10 files

## Historical Tracking

### CI Integration

Benchmarks run automatically on:
- Every push to `master`
- All pull requests
- Daily schedule (00:00 UTC)

Results are published to GitHub Pages at:
```
https://<your-org>.github.io/<repo-name>/dev/bench/
```

### Regression Detection

Automatic alerts trigger when:
- Performance degrades by **≥25%** vs. previous run
- Alerts posted as PR comments
- Workflow does NOT fail (informational only)

### Local Comparison

Compare current run against baseline:

```bash
# Save baseline
cargo bench --workspace --save-baseline baseline

# Make changes...

# Compare against baseline
cargo bench --workspace --baseline baseline

# Example output:
# request_analysis/simple  time:   [45.2 ms 46.1 ms 47.0 ms]
#                          change: [-5.2% -3.8% -2.1%] (improvement)
```

### Trend Analysis

View performance trends:

```bash
# Generate historical chart
cd target/criterion/<benchmark_name>
# Open index.html to see trend chart
```

Charts show:
- Performance over last N runs
- Confidence intervals
- Outlier detection
- Slope (improving/degrading)

## Interpreting Results

### Sample Output

```
request_analysis/simple time:   [42.123 ms 42.456 ms 42.801 ms]
                       change: [-3.2156% -2.5123% -1.7891%] (p = 0.00 < 0.05)
                       Performance has improved.
```

**Breakdown**:
- **time**: [lower_bound **mean** upper_bound] (95% confidence)
- **change**: Compared to previous run
- **p-value**: Statistical significance (p < 0.05 = significant)
- **verdict**: Improvement/regression/no change

### Performance Indicators

- ✅ **Improvement**: -X% change (faster)
- ⚠️ **Regression**: +X% change (slower)
- ➡️ **No Change**: Within statistical noise

**Thresholds**:
- **<5% change**: Likely noise, ignore
- **5-15% change**: Minor, review if repeated
- **15-25% change**: Moderate, investigate
- **>25% change**: Major regression, fix immediately

## Adding New Benchmarks

### 1. Add to `routing_benchmarks.rs`

```rust
fn bench_my_feature(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_feature");

    group.bench_function("test_case_1", |b| {
        b.iter(|| {
            // Code to benchmark
            black_box(expensive_function())
        });
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = bench_request_analysis, bench_my_feature  // Add here
}
```

### 2. Key Guidelines

**DO**:
- Use `black_box()` to prevent compiler optimizations
- Set appropriate `sample_size` (default: 100)
- Group related tests together
- Use descriptive names

**DON'T**:
- Include setup/teardown in benchmark
- Use random data (makes results unstable)
- Benchmark I/O without mocking
- Mix different units in one group

### 3. Test the Benchmark

```bash
# Run your new benchmark
cargo bench my_feature

# Check results
ls target/criterion/my_feature/
```

## CI/CD Configuration

### GitHub Actions Workflow

Location: `.github/workflows/benchmark.yml`

**Features**:
- Runs on Ubuntu (consistent environment)
- Caches dependencies for speed
- Stores results in gh-pages branch
- Compares against previous runs
- Posts regression alerts to PRs

### Customizing Alerts

Edit `.github/workflows/benchmark.yml`:

```yaml
- name: Store benchmark result
  uses: benchmark-action/github-action-benchmark@v1
  with:
    alert-threshold: '125%'  # Change this (125% = 25% regression)
    fail-on-alert: false     # Set true to fail CI on regression
```

## Best Practices

### 1. Consistent Environment

Run benchmarks on the same machine:
- **CI**: Always use same runner (ubuntu-latest)
- **Local**: Close other applications, disable CPU throttling

### 2. Statistical Significance

- Default: 100 samples (good balance)
- Quick check: 50 samples
- Production: 200+ samples

```bash
cargo bench -- --sample-size 200
```

### 3. Warm-up Period

Criterion automatically does warm-up runs. For specific needs:

```rust
config = Criterion::default()
    .warm_up_time(Duration::from_secs(5))
    .measurement_time(Duration::from_secs(10))
```

### 4. Comparing Branches

```bash
# On main branch
cargo bench --save-baseline main

# Switch to feature branch
git checkout feature-branch

# Compare
cargo bench --baseline main
```

## Troubleshooting

### Unstable Results

**Problem**: Large variance between runs

**Solutions**:
- Increase sample size: `--sample-size 200`
- Increase measurement time in benchmark config
- Close background applications
- Check for thermal throttling

### Missing Baseline

**Problem**: "No baseline found"

**Solution**: Run benchmark once to create baseline:
```bash
cargo bench
```

### Slow Benchmarks

**Problem**: Benchmarks take too long

**Solutions**:
- Reduce sample size: `--sample-size 50`
- Reduce measurement time in config
- Run specific benchmark: `cargo bench <name>`

## Performance Targets

Current targets for Merlin:

| Benchmark | Target | Current | Status |
|-----------|--------|---------|--------|
| Request Analysis (Simple) | <50ms | TBD | 🔵 To measure |
| Request Analysis (Complex) | <500ms | TBD | 🔵 To measure |
| Task Decomposition | <100ms | TBD | 🔵 To measure |
| Tier Selection | <1ms | TBD | 🔵 To measure |
| Context Building | <2s | TBD | 🔵 To measure |

Run initial benchmarks to establish baselines:

```bash
cargo bench --workspace
```

## Resources

- **Criterion.rs Book**: https://bheisler.github.io/criterion.rs/book/
- **GitHub Action**: https://github.com/benchmark-action/github-action-benchmark
- **Statistical Analysis**: https://bheisler.github.io/criterion.rs/book/analysis.html

---

**Last Updated**: 2025-10-07
**Benchmark Suite Version**: 1.0
