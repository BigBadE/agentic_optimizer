# Performance Benchmark Results

**Date**: 2025-10-10 11:36:17

## Summary

**Total Benchmarks**: 27

**Average Time**: 0.549 ms

## Benchmark Results

| Benchmark | Mean | Median | Std Dev |
|-----------|------|--------|--------|
| `complexity_analysis/complex` | 40.4 ns | 39.9 ns | 1.1 ns |
| `complexity_analysis/medium` | 70.3 ns | 67.5 ns | 15.8 ns |
| `complexity_analysis/simple` | 21.8 ns | 21.2 ns | 3.3 ns |
| `concurrent_requests/1_concurrent` | 773.383 μs | 768.326 μs | 21.747 μs |
| `concurrent_requests/2_concurrent` | 774.913 μs | 774.970 μs | 11.824 μs |
| `concurrent_requests/4_concurrent` | 796.458 μs | 794.227 μs | 20.748 μs |
| `concurrent_requests/8_concurrent` | 769.631 μs | 767.719 μs | 13.148 μs |
| `config_overhead/default_config` | 153.3 ns | 152.3 ns | 3.0 ns |
| `config_overhead/orchestrator_with_config` | 1.106 μs | 1.123 μs | 58.8 ns |
| `end_to_end_request/code_modification` | 781.588 μs | 771.158 μs | 24.919 μs |
| `end_to_end_request/complex_refactor` | 767.110 μs | 764.519 μs | 13.455 μs |
| `end_to_end_request/simple_query` | 748.440 μs | 738.955 μs | 22.673 μs |
| `memory_usage/multiple_requests` | 797.504 μs | 791.274 μs | 25.186 μs |
| `memory_usage/orchestrator_creation` | 853.3 ns | 835.1 ns | 46.2 ns |
| `request_analysis/complex` | 763.612 μs | 760.616 μs | 15.182 μs |
| `request_analysis/medium` | 764.349 μs | 761.674 μs | 12.733 μs |
| `request_analysis/simple` | 772.269 μs | 763.328 μs | 29.609 μs |
| `request_throughput/100_requests` | 840.778 μs | 840.547 μs | 60.716 μs |
| `request_throughput/10_requests` | 775.650 μs | 773.204 μs | 22.259 μs |
| `request_throughput/50_requests` | 2.376 ms | 897.437 μs | 5.298 ms |
| `task_decomposition/add error handling` | 759.900 μs | 754.820 μs | 12.681 μs |
| `task_decomposition/create a new rest api endpoint with validation` | 781.962 μs | 765.423 μs | 73.294 μs |
| `task_decomposition/implement a comprehensive test suite for the authentication modu` | 762.928 μs | 760.860 μs | 12.376 μs |
| `task_graph/10_tasks` | 1.294 μs | 1.289 μs | 20.3 ns |
| `task_graph/20_tasks` | 2.491 μs | 2.479 μs | 33.0 ns |
| `task_graph/50_tasks` | 6.130 μs | 6.070 μs | 132.9 ns |
| `task_graph/5_tasks` | 679.6 ns | 657.6 ns | 61.2 ns |

## Viewing Results

To view detailed HTML reports:
```bash
# Open the main report
open target/criterion/report/index.html
```

## Raw Data

Full benchmark data is stored in `target/criterion/` (27 benchmarks)
