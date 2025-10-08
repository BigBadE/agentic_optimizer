//! IAI-Callgrind integration benchmarks for end-to-end performance.
#![allow(
    dead_code,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::tests_outside_test_module,
    missing_docs,
    reason = "Test allows"
)]

use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use merlin_routing::{RoutingConfig, RoutingOrchestrator};
use std::hint::black_box;
use tokio::runtime::Runtime;

fn create_runtime() -> Runtime {
    Runtime::new().unwrap_or_else(|err| panic!("Failed to create runtime: {err}"))
}

// Benchmark end-to-end simple query
#[library_benchmark]
fn iai_e2e_simple_query() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    let runtime = create_runtime();

    runtime.block_on(async {
        let _result = orchestrator
            .analyze_request(black_box("What does the main function do?"))
            .await;
    });
}

// Benchmark end-to-end code modification
#[library_benchmark]
fn iai_e2e_code_modification() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    let runtime = Runtime::new().unwrap();

    runtime.block_on(async {
        let _result = orchestrator
            .analyze_request(black_box("Add error handling to the parser module"))
            .await;
    });
}

// Benchmark end-to-end complex refactor
#[library_benchmark]
fn iai_e2e_complex_refactor() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    let runtime = Runtime::new().unwrap();

    runtime.block_on(async {
        let _result = orchestrator
            .analyze_request(black_box(
                "Refactor the authentication system to use OAuth2 with JWT tokens and refresh logic",
            ))
            .await;
    });
}

// Benchmark multiple sequential requests
#[library_benchmark]
fn iai_sequential_requests() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    let runtime = Runtime::new().unwrap();

    let requests = vec!["Add a comment", "Fix the bug", "Refactor the code"];

    runtime.block_on(async {
        for request in &requests {
            let _result = orchestrator.analyze_request(black_box(request)).await;
        }
    });
}

library_benchmark_group!(
    name = integration_group;
    benchmarks =
        iai_e2e_simple_query,
        iai_e2e_code_modification,
        iai_e2e_complex_refactor,
        iai_sequential_requests
);

main!(library_benchmark_groups = integration_group);
