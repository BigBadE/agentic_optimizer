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
    let runtime = create_runtime();

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
                    runtime
                        .block_on(async { orchestrator.analyze_request(black_box(request)).await })
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

    let batch_sizes = vec![5, 10];
    let runtime = create_runtime();

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
                    runtime.block_on(process_requests_sequentially(&orchestrator, &requests));
                });
            },
        );
    }

    group.finish();
}

criterion_group! {
    name = integration_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(2))
        .warm_up_time(Duration::from_millis(500))
        .sample_size(10);
    targets = bench_end_to_end_request,
             bench_request_throughput
}

criterion_main!(integration_benches);
