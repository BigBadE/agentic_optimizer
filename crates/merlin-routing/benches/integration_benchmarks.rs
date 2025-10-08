//! Integration benchmarks for end-to-end performance and resource usage.
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::absolute_paths,
    clippy::min_ident_chars,
    clippy::used_underscore_binding,
    clippy::uninlined_format_args,
    clippy::missing_panics_doc,
    clippy::excessive_nesting,
    deprecated,
    reason = "Benchmark code has different conventions"
)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use merlin_routing::{RoutingConfig, RoutingOrchestrator};
use std::time::Duration;

/// Benchmark end-to-end request processing
fn bench_end_to_end_request(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end_request");

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
            |b, &request| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async { orchestrator.analyze_request(black_box(request)).await })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    group.bench_function("orchestrator_creation", |b| {
        b.iter(|| {
            let config = RoutingConfig::default();
            black_box(RoutingOrchestrator::new(config))
        });
    });

    group.bench_function("multiple_requests", |b| {
        let config = RoutingConfig::default();
        let orchestrator = RoutingOrchestrator::new(config);
        let requests = vec![
            "Add a comment",
            "Fix the bug",
            "Refactor the code",
            "Add tests",
            "Update documentation",
        ];

        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            for request in &requests {
                rt.block_on(async {
                    drop(orchestrator.analyze_request(black_box(request)).await);
                });
            }
        });
    });

    group.finish();
}

/// Benchmark concurrent request handling
fn bench_concurrent_requests(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_requests");

    let concurrency_levels = vec![1, 2, 4, 8];

    for level in concurrency_levels {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_concurrent", level)),
            &level,
            |b, &level| {
                let config = RoutingConfig::default();
                let _orchestrator = RoutingOrchestrator::new(config);
                let requests: Vec<_> = (0..level).map(|i| format!("Request {}", i)).collect();

                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let handles: Vec<_> = requests
                            .iter()
                            .map(|request| {
                                let req = request.clone();
                                tokio::spawn(async move {
                                    // Simulate concurrent request processing
                                    black_box(req.len())
                                })
                            })
                            .collect();

                        for handle in handles {
                            drop(handle.await);
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark request throughput
fn bench_request_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_throughput");

    let batch_sizes = vec![10, 50, 100];

    for size in batch_sizes {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_requests", size)),
            &size,
            |b, &size| {
                let config = RoutingConfig::default();
                let orchestrator = RoutingOrchestrator::new(config);
                let requests: Vec<_> = (0..size).map(|i| format!("Add feature {}", i)).collect();

                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        for request in &requests {
                            drop(orchestrator.analyze_request(black_box(request)).await);
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark configuration overhead
fn bench_config_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_overhead");

    group.bench_function("default_config", |b| {
        b.iter(|| black_box(RoutingConfig::default()));
    });

    group.bench_function("orchestrator_with_config", |b| {
        b.iter(|| {
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
