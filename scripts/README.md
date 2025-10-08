# Benchmark Comparison Scripts

## Quick Reference

### Initial Setup (One Time)

```bash
# Run benchmarks and save as baseline
./scripts/benchmark_compare.sh --run --baseline main
```

### Development Workflow

```bash
# Before making changes
./scripts/benchmark_compare.sh --run --baseline before-optimization

# Make your changes...

# After changes, compare
./scripts/benchmark_compare.sh --run --compare before-optimization
```

### Common Tasks

```bash
# List all saved baselines
./scripts/benchmark_compare.sh --list

# Just run benchmarks (no save/compare)
./scripts/benchmark_compare.sh --run

# Compare existing results without re-running
./scripts/benchmark_compare.sh --compare main

# Full workflow: run, compare, and save new baseline
./scripts/benchmark_compare.sh --run --compare old --baseline new
```

## Output Explained

### Comparison Results

```
✓ Improved    - Performance improved (faster)
⚠️ REGRESSION - Performance degraded by >15% (investigate!)
⚠  Slower     - Performance degraded by 5-15% (monitor)
➡️  No change  - Within 5% (statistical noise)
```

### Example Output

```
  request_analysis/simple          ✓ Improved (45.2 ms → 42.1 ms, -6.9%)
  request_analysis/complex         ⚠️ REGRESSION (523 ms → 678 ms, +29.6%)
  tier_selection/simple            ➡️ No change (0.8 ms → 0.9 ms, +2.1%)
```

## Workflow Examples

### Feature Development

```bash
# 1. Save baseline before starting
./scripts/benchmark_compare.sh --run --baseline feature-start

# 2. Develop your feature...

# 3. Check performance impact
./scripts/benchmark_compare.sh --run --compare feature-start

# 4. If regressions found, optimize and re-check
./scripts/benchmark_compare.sh --run --compare feature-start

# 5. When satisfied, save final state
./scripts/benchmark_compare.sh --run --baseline feature-complete
```

### Performance Optimization

```bash
# 1. Establish current baseline
./scripts/benchmark_compare.sh --run --baseline pre-optimization

# 2. Make optimization attempt #1
./scripts/benchmark_compare.sh --run --compare pre-optimization

# 3. If improved, save new baseline
./scripts/benchmark_compare.sh --baseline optimization-v1

# 4. Try another optimization
./scripts/benchmark_compare.sh --run --compare optimization-v1

# 5. Compare final result against original
./scripts/benchmark_compare.sh --compare pre-optimization
```

### Regression Investigation

```bash
# 1. Check which baseline regressed
./scripts/benchmark_compare.sh --list

# 2. Compare against known-good baseline
./scripts/benchmark_compare.sh --run --compare known-good

# 3. Bisect using git to find regression
git bisect start
git bisect bad HEAD
git bisect good <known-good-commit>

# For each bisect step:
./scripts/benchmark_compare.sh --run --compare known-good
git bisect good  # or git bisect bad
```

## Tips

### Consistent Results

For consistent benchmarking:
1. Close other applications
2. Disable CPU power management: `sudo cpupower frequency-set -g performance`
3. Run on the same machine
4. Use the same power state (plugged in vs. battery)

### Quick Checks

For quick iteration during development:

```bash
# Run only specific benchmarks
cargo bench request_analysis

# Run with fewer samples (faster but less accurate)
cargo bench -- --sample-size 50
```

### Long-term Tracking

For tracking over many commits:

```bash
# Tag each release
./scripts/benchmark_compare.sh --run --baseline release-0.1.0
./scripts/benchmark_compare.sh --run --baseline release-0.2.0

# Compare releases
./scripts/benchmark_compare.sh --compare release-0.1.0
```

## CI Integration

The GitHub Actions workflow automatically:
- Runs benchmarks on every push to master
- Compares against previous runs
- Posts alerts on PRs if regressions >25%
- Publishes results to GitHub Pages

View results at: `https://<org>.github.io/<repo>/dev/bench/`

## Troubleshooting

### "No baseline found"

Run benchmarks to create baseline:
```bash
./scripts/benchmark_compare.sh --run
```

### "jq: command not found"

Install jq (JSON processor):
```bash
# Ubuntu/Debian
sudo apt-get install jq

# macOS
brew install jq

# Windows (Git Bash)
# Download from https://stedolan.github.io/jq/download/
```

### Large Variance

If results vary widely between runs:
1. Increase sample size in benchmark config
2. Close background applications
3. Check for thermal throttling
4. Run multiple times and average

## Advanced Usage

### Custom Thresholds

Edit the script to change regression threshold:

```bash
# Line 12 in benchmark_compare.sh
REGRESSION_THRESHOLD=15  # Change this value (percentage)
```

### Automated Comparisons

Add to your git hooks (`.git/hooks/pre-push`):

```bash
#!/bin/bash
./scripts/benchmark_compare.sh --run --compare main
if [ $? -ne 0 ]; then
    echo "WARNING: Performance regressions detected!"
    read -p "Continue with push? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi
```

---

**See also**: [BENCHMARKING.md](../docs/BENCHMARKING.md) for complete benchmarking guide
