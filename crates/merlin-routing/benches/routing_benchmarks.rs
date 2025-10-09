//! Benchmarks for routing performance and task analysis.
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

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use merlin_routing::{RoutingConfig, RoutingOrchestrator};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Helper to create runtime or panic (benchmarks expect setup to succeed)
fn create_runtime() -> Runtime {
    Runtime::new().unwrap_or_else(|err| panic!("Failed to create runtime: {err}"))
}

/// Benchmark request analysis performance
fn bench_request_analysis(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("request_analysis");

    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);

    let test_cases = vec![
        ("simple", "Add a comment to main function"),
        (
            "medium",
            "Refactor the parser module to use better error handling",
        ),
        (
            "complex",
            "Create a new authentication system with OAuth2, JWT tokens, and refresh logic",
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

/// Benchmark task decomposition
fn bench_task_decomposition(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("task_decomposition");

    let requests = vec![
        "Add error handling",
        "Create a new REST API endpoint with validation",
        "Implement a comprehensive test suite for the authentication module",
    ];

    for request in requests {
        group.bench_function(request, |bencher| {
            let config = RoutingConfig::default();
            let orchestrator = RoutingOrchestrator::new(config);

            bencher.iter(|| {
                let runtime = create_runtime();
                runtime.block_on(async { orchestrator.analyze_request(black_box(request)).await })
            });
        });
    }

    group.finish();
}

/// Benchmark complexity analysis (synchronous operation)
fn bench_complexity_analysis(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("complexity_analysis");

    let test_cases = vec![
        ("simple", "Add a comment"),
        ("medium", "Refactor module with error handling"),
        ("complex", "Create OAuth2 system with JWT"),
    ];

    for (name, query) in test_cases {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &query,
            |bencher, &query| {
                bencher.iter(|| {
                    // Analyze complexity of the query
                    let char_count = query.chars().count();
                    let word_count = query.split_whitespace().count();
                    // Simplified complexity check
                    black_box((char_count, word_count))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark task graph construction
fn bench_task_graph(criterion: &mut Criterion) {
    use merlin_routing::{Task, executor::graph::TaskGraph};

    let mut group = criterion.benchmark_group("task_graph");

    let task_counts = vec![5, 10, 20, 50];

    for count in task_counts {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{count}_tasks")),
            &count,
            |bencher, &count| {
                let tasks: Vec<Task> = (0..count)
                    .map(|idx| Task::new(format!("Task {idx}")))
                    .collect();

                bencher.iter(|| TaskGraph::from_tasks(black_box(&tasks)));
            },
        );
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(2))
        .sample_size(20);
    targets = bench_request_analysis,
             bench_task_decomposition,
             bench_complexity_analysis,
             bench_task_graph
}

criterion_main!(benches);
