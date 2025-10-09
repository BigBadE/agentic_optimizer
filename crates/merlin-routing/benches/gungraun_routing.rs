//! Gungraun benchmarks for routing performance using Cachegrind.
//!
//! These benchmarks provide precise, single-shot measurements of instruction counts,
//! cache accesses, and estimated cycles. Unlike Criterion benchmarks which use
//! statistical analysis, Gungraun uses Valgrind's Cachegrind to get deterministic results.
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
    unsafe_code,
    reason = "Test allows"
)]

use gungraun::{library_benchmark, library_benchmark_group, main};
use merlin_routing::{RoutingConfig, RoutingOrchestrator};
use std::hint::black_box;
use tokio::runtime::Runtime;

// Helper to create runtime or panic (benchmarks expect setup to succeed)
fn create_runtime() -> Runtime {
    Runtime::new().unwrap_or_else(|err| panic!("Failed to create runtime: {err}"))
}

// Benchmark simple request analysis
#[library_benchmark]
fn gungraun_analyze_simple_request() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    let runtime = create_runtime();

    runtime.block_on(async {
        let _result = orchestrator
            .analyze_request(black_box("Add a comment to main function"))
            .await;
    });
}

// Benchmark medium complexity request
#[library_benchmark]
fn gungraun_analyze_medium_request() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    let runtime = create_runtime();

    runtime.block_on(async {
        let _result = orchestrator
            .analyze_request(black_box(
                "Refactor the parser module to use better error handling",
            ))
            .await;
    });
}

// Benchmark complex request analysis
#[library_benchmark]
fn gungraun_analyze_complex_request() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    let runtime = create_runtime();

    runtime.block_on(async {
        let _result = orchestrator
            .analyze_request(black_box(
                "Create a new authentication system with OAuth2, JWT tokens, and refresh logic",
            ))
            .await;
    });
}

// Benchmark Orchestrator creation overhead
#[library_benchmark]
fn gungraun_create_orchestrator() {
    let config = RoutingConfig::default();
    let _orchestrator = black_box(RoutingOrchestrator::new(config));
}

// Benchmark config creation
#[library_benchmark]
fn gungraun_create_config() {
    let _config = black_box(RoutingConfig::default());
}

library_benchmark_group!(
    name = routing_group;
    benchmarks =
        gungraun_analyze_simple_request,
        gungraun_analyze_medium_request,
        gungraun_analyze_complex_request,
        gungraun_create_orchestrator,
        gungraun_create_config
);

main!(library_benchmark_groups = routing_group);
