//! IAI benchmarks for routing performance using Cachegrind.
//!
//! These benchmarks provide precise, single-shot measurements of instruction counts,
//! cache accesses, and estimated cycles. Unlike Criterion benchmarks which use
//! statistical analysis, IAI uses Valgrind's Cachegrind to get deterministic results.
#![allow(
    clippy::unwrap_used,
    clippy::absolute_paths,
    clippy::missing_panics_doc,
    reason = "Benchmark code has different conventions"
)]

use iai::black_box;
use merlin_routing::{RoutingConfig, RoutingOrchestrator};
use tokio::runtime::Runtime;

/// Benchmark simple request analysis
fn iai_analyze_simple_request() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    let runtime = Runtime::new().unwrap();

    runtime.block_on(async {
        let _result = orchestrator
            .analyze_request(black_box("Add a comment to main function"))
            .await;
    });
}

/// Benchmark medium complexity request
fn iai_analyze_medium_request() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    let runtime = Runtime::new().unwrap();

    runtime.block_on(async {
        let _result = orchestrator
            .analyze_request(black_box(
                "Refactor the parser module to use better error handling",
            ))
            .await;
    });
}

/// Benchmark complex request analysis
fn iai_analyze_complex_request() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    let runtime = Runtime::new().unwrap();

    runtime.block_on(async {
        let _result = orchestrator
            .analyze_request(black_box(
                "Create a new authentication system with OAuth2, JWT tokens, and refresh logic",
            ))
            .await;
    });
}

/// Benchmark orchestrator creation overhead
fn iai_create_orchestrator() {
    let config = RoutingConfig::default();
    let _orchestrator = black_box(RoutingOrchestrator::new(config));
}

/// Benchmark config creation
fn iai_create_config() {
    let _config = black_box(RoutingConfig::default());
}

iai::main!(
    iai_analyze_simple_request,
    iai_analyze_medium_request,
    iai_analyze_complex_request,
    iai_create_orchestrator,
    iai_create_config
);
