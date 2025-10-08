//! Integration benchmarks for end-to-end performance and resource usage.
#![allow(
    dead_code,
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::tests_outside_test_module,
    reason = "Test allows"
)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use tokio::runtime::Runtime;
use tokio::spawn;

/// Helper to create runtime or panic (benchmarks expect setup to succeed)
fn create_runtime() -> Runtime {
    Runtime::new().unwrap_or_else(|err| panic!("Failed to create runtime: {err}"))
}
use merlin_routing::{RoutingConfig, RoutingOrchestrator};
use std::time::Duration;

/// Benchmark end-to-end request processing
fn bench_end_to_end_request(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("end_to_end_request");

    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);

    let test_cases = vec![
        ("simple_query", "What does the main function do?"),
        (
            "code_modification",
            "Add error handling to the parser module",
        ),
        (
            "complex_refactor",
            "Refactor the authentication system to use OAuth2 with JWT tokens and refresh logic",
        ),
    ];

    for (name, request) in test_cases {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &request,
            |bencher, &request| {
                bencher.iter(|| {
                    let runtime = create_runtime();
                    runtime
                        .block_on(async { orchestrator.analyze_request(black_box(request)).await })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_usage(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("memory_usage");

    group.bench_function("orchestrator_creation", |bencher| {
        bencher.iter(|| {
            let config = RoutingConfig::default();
            black_box(RoutingOrchestrator::new(config))
        });
    });

    group.bench_function("multiple_requests", |bencher| {
        let config = RoutingConfig::default();
        let orchestrator = RoutingOrchestrator::new(config);
        let requests = vec![
            "Add a comment",
            "Fix the bug",
            "Refactor the code",
            "Add tests",
            "Update documentation",
        ];

        bencher.iter(|| {
            let runtime = create_runtime();
            for request in &requests {
                runtime.block_on(async {
                    drop(orchestrator.analyze_request(black_box(request)).await);
                });
            }
        });
    });

    group.finish();
}

/// Helper function to process concurrent requests
async fn process_concurrent_requests(requests: &[String]) {
    let handles: Vec<_> = requests
        .iter()
        .map(|request| {
            let req = request.clone();
            spawn(async move {
                // Simulate concurrent request processing
                black_box(req.len())
            })
        })
        .collect();

    for handle in handles {
        drop(handle.await);
    }
}

/// Benchmark concurrent request handling
fn bench_concurrent_requests(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("concurrent_requests");

    let concurrency_levels = vec![1, 2, 4, 8];

    for level in concurrency_levels {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{level}_concurrent")),
            &level,
            |bencher, &level| {
                let config = RoutingConfig::default();
                let _orchestrator = RoutingOrchestrator::new(config);
                let requests: Vec<_> = (0..level).map(|idx| format!("Request {idx}")).collect();

                bencher.iter(|| {
                    let runtime = create_runtime();
                    runtime.block_on(process_concurrent_requests(&requests));
                });
            },
        );
    }

    group.finish();
}

/// Helper function to process requests sequentially
async fn process_requests_sequentially(orchestrator: &RoutingOrchestrator, requests: &[String]) {
    for request in requests {
        drop(orchestrator.analyze_request(black_box(request)).await);
    }
}

/// Benchmark request throughput
fn bench_request_throughput(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("request_throughput");

    let batch_sizes = vec![10, 50, 100];

    for size in batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size}_requests")),
            &size,
            |bencher, &size| {
                let config = RoutingConfig::default();
                let orchestrator = RoutingOrchestrator::new(config);
                let requests: Vec<_> = (0..size).map(|idx| format!("Add feature {idx}")).collect();

                bencher.iter(|| {
                    let runtime = create_runtime();
                    runtime.block_on(process_requests_sequentially(&orchestrator, &requests));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark configuration overhead
fn bench_config_overhead(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("config_overhead");

    group.bench_function("default_config", |bencher| {
        bencher.iter(|| black_box(RoutingConfig::default()));
    });

    group.bench_function("orchestrator_with_config", |bencher| {
        bencher.iter(|| {
            let config = RoutingConfig::default();
            black_box(RoutingOrchestrator::new(config))
        });
    });

    group.finish();
}

criterion_group! {
    name = integration_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(15))
        .warm_up_time(Duration::from_secs(5))
        .sample_size(30);
    targets = bench_end_to_end_request,
             bench_memory_usage,
             bench_concurrent_requests,
             bench_request_throughput,
             bench_config_overhead
}

criterion_main!(integration_benches);
