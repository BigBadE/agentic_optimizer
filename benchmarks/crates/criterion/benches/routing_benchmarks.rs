//! Benchmarks for routing performance and task analysis.
#![cfg_attr(
    test,
    allow(
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        reason = "Allow for tests"
    )
)]

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use merlin_agent::RoutingOrchestrator;
use merlin_core::RoutingConfig;
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
    let orchestrator = RoutingOrchestrator::new(config).expect("Failed to create orchestrator");
    let runtime = create_runtime();

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

    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config).expect("Failed to create orchestrator");
    let runtime = create_runtime();

    let requests = vec![
        "Add error handling",
        "Create a new REST API endpoint with validation",
        "Implement a comprehensive test suite for the authentication module",
    ];

    for request in requests {
        group.bench_function(request, |bencher| {
            bencher.iter(|| {
                runtime.block_on(async { orchestrator.analyze_request(black_box(request)).await })
            });
        });
    }

    group.finish();
}

/// Benchmark task graph construction
fn bench_task_graph(criterion: &mut Criterion) {
    use merlin_agent::TaskGraph;
    use merlin_core::Task;

    let mut group = criterion.benchmark_group("task_graph");

    let task_counts = vec![5, 10, 20];

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
        .measurement_time(Duration::from_secs(2))
        .warm_up_time(Duration::from_millis(500))
        .sample_size(10);
    targets = bench_request_analysis,
             bench_task_decomposition,
             bench_task_graph
}

criterion_main!(benches);
