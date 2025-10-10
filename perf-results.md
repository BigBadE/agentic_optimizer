# Performance Benchmark Results

**Date**: 2025-10-10 01:33:00

## Summary

**Total Benchmarks**: 27

**Average Time**: 0.510 ms

## Benchmark Results

| Benchmark | Mean | Median | Std Dev |
|-----------|------|--------|--------|
| `complexity_analysis/complex` | 40.2 ns | 40.0 ns | 0.9 ns |
| `complexity_analysis/medium` | 48.9 ns | 48.8 ns | 0.5 ns |
| `complexity_analysis/simple` | 19.5 ns | 19.4 ns | 0.3 ns |
| `concurrent_requests/1_concurrent` | 743.934 μs | 743.022 μs | 5.952 μs |
| `concurrent_requests/2_concurrent` | 752.787 μs | 749.833 μs | 14.399 μs |
| `concurrent_requests/4_concurrent` | 757.990 μs | 753.585 μs | 14.418 μs |
| `concurrent_requests/8_concurrent` | 740.033 μs | 739.553 μs | 5.070 μs |
| `config_overhead/default_config` | 148.4 ns | 147.3 ns | 2.5 ns |
| `config_overhead/orchestrator_with_config` | 1.024 μs | 1.023 μs | 4.3 ns |
| `end_to_end_request/code_modification` | 1.326 ms | 1.167 ms | 593.595 μs |
| `end_to_end_request/complex_refactor` | 848.006 μs | 832.501 μs | 34.041 μs |
| `end_to_end_request/simple_query` | 1.029 ms | 920.066 μs | 262.593 μs |
| `memory_usage/multiple_requests` | 776.839 μs | 764.193 μs | 30.054 μs |
| `memory_usage/orchestrator_creation` | 952.9 ns | 914.2 ns | 149.2 ns |
| `request_analysis/complex` | 752.702 μs | 733.695 μs | 59.862 μs |
| `request_analysis/medium` | 736.819 μs | 734.759 μs | 9.202 μs |
| `request_analysis/simple` | 713.937 μs | 712.310 μs | 5.276 μs |
| `request_throughput/100_requests` | 778.472 μs | 763.061 μs | 53.678 μs |
| `request_throughput/10_requests` | 746.488 μs | 748.484 μs | 10.983 μs |
| `request_throughput/50_requests` | 746.469 μs | 746.728 μs | 10.138 μs |
| `task_decomposition/add error handling` | 765.151 μs | 764.488 μs | 5.655 μs |
| `task_decomposition/create a new rest api endpoint with validation` | 765.969 μs | 765.212 μs | 5.781 μs |
| `task_decomposition/implement a comprehensive test suite for the authentication modu` | 770.192 μs | 767.844 μs | 11.067 μs |
| `task_graph/10_tasks` | 1.336 μs | 1.299 μs | 91.0 ns |
| `task_graph/20_tasks` | 2.528 μs | 2.517 μs | 36.0 ns |
| `task_graph/50_tasks` | 6.110 μs | 6.100 μs | 65.7 ns |
| `task_graph/5_tasks` | 655.1 ns | 653.8 ns | 3.9 ns |

## Viewing Results

To view detailed HTML reports:
```bash
# Open the main report
open target/criterion/report/index.html
```

## Raw Data

Full benchmark data is stored in `target/criterion/` (27 benchmarks)
