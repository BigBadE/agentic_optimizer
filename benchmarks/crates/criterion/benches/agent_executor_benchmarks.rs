//! Agent executor benchmarks for end-to-end streaming execution.

use anyhow::Error;
use criterion::{BenchmarkId, Criterion, Throughput};
use merlin_agent::RoutingOrchestrator;
use merlin_core::{RoutingConfig, Task};
use merlin_routing::{UiChannel, UiEvent};
use std::hint::black_box;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

/// Benchmark agent executor with streaming
///
/// # Errors
/// Returns an error if benchmark setup or execution fails.
fn bench_agent_executor_streaming(criterion: &mut Criterion) -> Result<(), Error> {
    let mut group = criterion.benchmark_group("agent_executor_streaming");

    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config)?;
    let runtime = Runtime::new()?;

    let test_cases = vec![
        ("simple_query", "What does the main function do?"),
        ("code_review", "Review the error handling in parser.rs"),
        ("explanation", "Explain how the routing system works"),
    ];

    for (name, request) in test_cases {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &request,
            |bencher, &request| {
                bencher.iter(|| {
                    let task = Task::new(black_box(request).to_string());
                    let (sender, _receiver) = mpsc::channel::<UiEvent>(100);
                    let ui_channel = UiChannel::from_sender(sender);
                    runtime.block_on(async {
                        let _result = orchestrator.execute_task_streaming(task, ui_channel).await;
                    });
                });
            },
        );
    }

    group.finish();
    Ok(())
}

/// Benchmark multi-turn conversation
///
/// # Errors
/// Returns an error if benchmark setup or execution fails.
fn bench_conversation_history(criterion: &mut Criterion) -> Result<(), Error> {
    let mut group = criterion.benchmark_group("conversation_history");

    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config)?;
    let runtime = Runtime::new()?;

    let history_sizes = vec![0, 5, 10];

    for size in history_sizes {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size}_messages")),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let task = Task::new(black_box("Continue the conversation").to_string());
                    let (sender, _receiver) = mpsc::channel::<UiEvent>(100);
                    let ui_channel = UiChannel::from_sender(sender);
                    let history: Vec<(String, String)> = (0..size)
                        .map(|idx| ("user".to_string(), format!("Question {idx}")))
                        .collect();

                    runtime.block_on(async {
                        let _result = orchestrator
                            .execute_task_streaming_with_history(task, ui_channel, history)
                            .await;
                    });
                });
            },
        );
    }

    group.finish();
    Ok(())
}

/// Executes the executor benchmarks
pub fn main() -> Result<(), Error> {
    let mut criterion = Criterion::default().configure_from_args();
    bench_agent_executor_streaming(&mut criterion)?;
    bench_conversation_history(&mut criterion)?;
    criterion.final_summary();
    Ok(())
}
