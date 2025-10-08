//! IAI integration benchmarks for end-to-end performance.
#![allow(
    clippy::unwrap_used,
    clippy::absolute_paths,
    clippy::missing_panics_doc,
    reason = "Benchmark code has different conventions"
)]

use iai::black_box;
use merlin_routing::{RoutingConfig, RoutingOrchestrator};
use tokio::runtime::Runtime;

/// Benchmark end-to-end simple query
fn iai_e2e_simple_query() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    let runtime = Runtime::new().unwrap();
    
    runtime.block_on(async {
        let _result = orchestrator
            .analyze_request(black_box("What does the main function do?"))
            .await;
    });
}

/// Benchmark end-to-end code modification
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

/// Benchmark end-to-end complex refactor
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

/// Benchmark multiple sequential requests
fn iai_sequential_requests() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);
    let runtime = Runtime::new().unwrap();
    
    let requests = vec![
        "Add a comment",
        "Fix the bug",
        "Refactor the code",
    ];
    
    runtime.block_on(async {
        for request in &requests {
            let _result = orchestrator.analyze_request(black_box(request)).await;
        }
    });
}

iai::main!(
    iai_e2e_simple_query,
    iai_e2e_code_modification,
    iai_e2e_complex_refactor,
    iai_sequential_requests
);
