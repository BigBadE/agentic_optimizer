# IAI Performance Benchmark Results

**Date**: 2025-10-08 14:44:04

## Summary

IAI benchmarks use Cachegrind to provide precise, deterministic measurements.

## Benchmark Results

```

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 76 filtered out; finished in 0.00s

Unexpected error while launching valgrind. Error: program not found
Unexpected error while launching valgrind. Error: program not found
```

## Metrics Explained

- **Instructions**: Total CPU instructions executed
- **L1 Accesses**: Level 1 cache accesses
- **L2 Accesses**: Level 2 cache accesses
- **RAM Accesses**: Main memory accesses
- **Estimated Cycles**: Estimated CPU cycles (lower is better)

## Viewing Detailed Results

Cachegrind output files are stored in `target/iai/`.
You can analyze them with tools like `cg_annotate` or `kcachegrind`.
