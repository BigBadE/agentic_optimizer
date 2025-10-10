# Performance Benchmark Results

**Date**: 2025-10-10 02:33:41

## Summary

**Total Benchmarks**: 27

**Average Time**: 0.462 ms

## Benchmark Results

| Benchmark | Mean | Median | Std Dev |
|-----------|------|--------|--------|
| `complexity_analysis/complex` | 39.6 ns | 39.6 ns | 0.3 ns |
| `complexity_analysis/medium` | 48.3 ns | 48.3 ns | 0.1 ns |
| `complexity_analysis/simple` | 19.0 ns | 19.0 ns | 0.1 ns |
| `concurrent_requests/1_concurrent` | 736.180 μs | 734.571 μs | 5.304 μs |
| `concurrent_requests/2_concurrent` | 733.177 μs | 732.848 μs | 2.362 μs |
| `concurrent_requests/4_concurrent` | 732.948 μs | 731.999 μs | 3.649 μs |
| `concurrent_requests/8_concurrent` | 736.250 μs | 733.434 μs | 6.114 μs |
| `config_overhead/default_config` | 147.2 ns | 147.2 ns | 0.1 ns |
| `config_overhead/orchestrator_with_config` | 836.8 ns | 836.7 ns | 1.1 ns |
| `end_to_end_request/code_modification` | 734.488 μs | 734.022 μs | 3.713 μs |
| `end_to_end_request/complex_refactor` | 735.401 μs | 735.122 μs | 4.019 μs |
| `end_to_end_request/simple_query` | 719.446 μs | 714.367 μs | 10.573 μs |
| `memory_usage/multiple_requests` | 738.439 μs | 737.951 μs | 3.582 μs |
| `memory_usage/orchestrator_creation` | 800.4 ns | 800.7 ns | 1.4 ns |
| `request_analysis/complex` | 730.725 μs | 729.727 μs | 6.591 μs |
| `request_analysis/medium` | 736.941 μs | 734.363 μs | 9.713 μs |
| `request_analysis/simple` | 721.667 μs | 717.858 μs | 18.146 μs |
| `request_throughput/100_requests` | 744.418 μs | 740.434 μs | 9.419 μs |
| `request_throughput/10_requests` | 735.675 μs | 734.827 μs | 6.409 μs |
| `request_throughput/50_requests` | 733.513 μs | 733.161 μs | 3.690 μs |
| `task_decomposition/add error handling` | 736.250 μs | 730.672 μs | 13.819 μs |
| `task_decomposition/create a new rest api endpoint with validation` | 728.911 μs | 728.593 μs | 4.870 μs |
| `task_decomposition/implement a comprehensive test suite for the authentication modu` | 730.097 μs | 728.167 μs | 6.736 μs |
| `task_graph/10_tasks` | 1.278 μs | 1.274 μs | 14.4 ns |
| `task_graph/20_tasks` | 2.519 μs | 2.506 μs | 35.5 ns |
| `task_graph/50_tasks` | 6.099 μs | 6.098 μs | 6.2 ns |
| `task_graph/5_tasks` | 657.6 ns | 649.2 ns | 16.2 ns |

## Viewing Results

To view detailed HTML reports:
```bash
# Open the main report
open target/criterion/report/index.html
```

## Raw Data

Full benchmark data is stored in `target/criterion/` (27 benchmarks)
