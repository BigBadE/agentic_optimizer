//! Gungraun benchmarks for agent executor performance.

use divan::Divan;
use hint::black_box;
use merlin_agent::RoutingOrchestrator;
use merlin_core::{RoutingConfig, Task};
use merlin_routing::{UiChannel, UiEvent};
use mimalloc::MiMalloc;
use runtime::Runtime;
use std::hint;
use tokio::runtime;
use tokio::sync::mpsc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    Divan::from_args().main();
}

/// Benchmark simple task execution
#[divan::bench]
fn simple_task_execution() {
    let config = RoutingConfig::default();
    let Ok(orchestrator) = RoutingOrchestrator::new(config) else {
        return;
    };

    let Ok(tokio_runtime) = Runtime::new() else {
        return;
    };

    tokio_runtime.block_on(async {
        let task = Task::new(black_box("What is in main.rs?").to_string());
        let (sender, _receiver) = mpsc::channel::<UiEvent>(100);
        let ui_channel = UiChannel::from_sender(sender);
        let _result = orchestrator.execute_task_streaming(task, ui_channel).await;
    });
}

/// Benchmark with conversation history
#[divan::bench(args = [0, 5, 10])]
fn task_with_history(history_size: usize) {
    let config = RoutingConfig::default();
    let Ok(orchestrator) = RoutingOrchestrator::new(config) else {
        return;
    };

    let Ok(tokio_runtime) = Runtime::new() else {
        return;
    };

    tokio_runtime.block_on(async {
        let task = Task::new(black_box("Continue conversation").to_string());
        let (sender, _receiver) = mpsc::channel::<UiEvent>(100);
        let ui_channel = UiChannel::from_sender(sender);
        let history: Vec<(String, String)> = (0..history_size)
            .map(|idx| ("user".to_string(), format!("Message {idx}")))
            .collect();

        let _result = orchestrator
            .execute_task_streaming_with_history(task, ui_channel, history)
            .await;
    });
}
