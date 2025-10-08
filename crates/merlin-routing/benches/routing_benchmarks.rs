//! Benchmarks for routing performance and task analysis.
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::absolute_paths,
    clippy::min_ident_chars,
    clippy::used_underscore_binding,
    clippy::uninlined_format_args,
    clippy::missing_panics_doc,
    deprecated,
    reason = "Benchmark code has different conventions"
)]

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use merlin_routing::{RoutingConfig, RoutingOrchestrator};
use std::time::Duration;

/// Benchmark request analysis performance
fn bench_request_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_analysis");

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

/// Benchmark task decomposition
fn bench_task_decomposition(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_decomposition");

    let requests = vec![
        "Add error handling",
        "Create a new REST API endpoint with validation",
        "Implement a comprehensive test suite for the authentication module",
    ];

    for request in requests {
        group.bench_function(request, |b| {
            let config = RoutingConfig::default();
            let orchestrator = RoutingOrchestrator::new(config);

            b.iter(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async { orchestrator.analyze_request(black_box(request)).await })
            });
        });
    }

    group.finish();
}

/// Benchmark complexity analysis (synchronous operation)
fn bench_complexity_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("complexity_analysis");

    let test_cases = vec![
        ("simple", "Add a comment"),
        ("medium", "Refactor module with error handling"),
        ("complex", "Create OAuth2 system with JWT"),
    ];

    for (name, query) in test_cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), &query, |b, &query| {
            b.iter(|| {
                // Analyze complexity of the query
                let _chars = query.chars().count();
                let _words = query.split_whitespace().count();
                // Simplified complexity check
                black_box(_words)
            });
        });
    }

    group.finish();
}

/// Benchmark task graph construction
fn bench_task_graph(c: &mut Criterion) {
    use merlin_routing::{Task, executor::graph::TaskGraph};

    let mut group = c.benchmark_group("task_graph");

    let task_counts = vec![5, 10, 20, 50];

    for count in task_counts {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_tasks", count)),
            &count,
            |b, &count| {
                let tasks: Vec<Task> = (0..count)
                    .map(|i| Task::new(format!("Task {}", i)))
                    .collect();

                b.iter(|| TaskGraph::from_tasks(black_box(&tasks)));
            },
        );
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3))
        .sample_size(50);
    targets = bench_request_analysis, bench_task_decomposition, bench_complexity_analysis, bench_task_graph
}

criterion_main!(benches);
