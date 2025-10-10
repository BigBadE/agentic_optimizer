# Performance Benchmark Results

**Date**: 2025-10-10 00:42:30

## Summary

**Total Benchmarks**: 27

**Average Time**: 0.464 ms

## Benchmark Results

| Benchmark | Mean | Median | Std Dev |
|-----------|------|--------|--------|
| `complexity_analysis/complex` | 39.6 ns | 39.6 ns | 0.1 ns |
| `complexity_analysis/medium` | 48.5 ns | 48.3 ns | 0.8 ns |
| `complexity_analysis/simple` | 19.0 ns | 18.9 ns | 0.1 ns |
| `concurrent_requests/1_concurrent` | 736.813 μs | 735.702 μs | 5.340 μs |
| `concurrent_requests/2_concurrent` | 736.762 μs | 735.544 μs | 7.353 μs |
| `concurrent_requests/4_concurrent` | 738.869 μs | 736.738 μs | 5.895 μs |
| `concurrent_requests/8_concurrent` | 738.060 μs | 737.129 μs | 6.480 μs |
| `config_overhead/default_config` | 150.1 ns | 149.7 ns | 1.6 ns |
| `config_overhead/orchestrator_with_config` | 811.2 ns | 810.2 ns | 4.0 ns |
| `end_to_end_request/code_modification` | 738.104 μs | 735.468 μs | 9.200 μs |
| `end_to_end_request/complex_refactor` | 737.843 μs | 736.134 μs | 4.646 μs |
| `end_to_end_request/simple_query` | 720.023 μs | 716.313 μs | 7.772 μs |
| `memory_usage/multiple_requests` | 745.019 μs | 741.342 μs | 11.227 μs |
| `memory_usage/orchestrator_creation` | 811.6 ns | 810.5 ns | 3.6 ns |
| `request_analysis/complex` | 732.376 μs | 731.103 μs | 5.945 μs |
| `request_analysis/medium` | 745.190 μs | 744.282 μs | 12.994 μs |
| `request_analysis/simple` | 714.543 μs | 713.438 μs | 4.868 μs |
| `request_throughput/100_requests` | 746.932 μs | 746.761 μs | 5.589 μs |
| `request_throughput/10_requests` | 740.787 μs | 736.917 μs | 7.452 μs |
| `request_throughput/50_requests` | 737.275 μs | 736.770 μs | 3.040 μs |
| `task_decomposition/add error handling` | 735.512 μs | 733.001 μs | 7.374 μs |
| `task_decomposition/create a new rest api endpoint with validation` | 735.655 μs | 732.660 μs | 9.208 μs |
| `task_decomposition/implement a comprehensive test suite for the authentication modu` | 732.896 μs | 731.833 μs | 4.867 μs |
| `task_graph/10_tasks` | 1.256 μs | 1.252 μs | 11.3 ns |
| `task_graph/20_tasks` | 2.472 μs | 2.470 μs | 7.9 ns |
| `task_graph/50_tasks` | 5.880 μs | 5.879 μs | 9.7 ns |
| `task_graph/5_tasks` | 646.9 ns | 640.8 ns | 25.5 ns |

## Viewing Results

To view detailed HTML reports:
```bash
# Open the main report
open target/criterion/report/index.html
```

## Raw Data

Full benchmark data is stored in `target/criterion/` (27 benchmarks)
