//! Benchmarks for metrics collection and reporting performance.

#![allow(
    clippy::min_ident_chars,
    missing_docs,
    let_underscore_drop,
    reason = "Benchmarks use standard loop variables and drop values intentionally"
)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use merlin_core::TokenUsage;
use merlin_routing::{
    MetricsCollector, MetricsReport, RequestMetrics, metrics::collector::RequestMetricsParams,
};
use std::hint::black_box;
use std::time::Duration;

fn bench_metrics_recording(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics_recording");
    for size in &[10, 100] {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut collector = MetricsCollector::new();
                for idx in 0..size {
                    let metrics = RequestMetrics::new(RequestMetricsParams {
                        query: format!("query_{idx}"),
                        tier_used: "local".to_owned(),
                        latency_ms: 100,
                        tokens_used: TokenUsage::default(),
                        success: true,
                        escalated: false,
                    });
                    collector.record(black_box(metrics));
                }
            });
        });
    }
    group.finish();
}

fn bench_daily_report_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("daily_report");

    for size in &[10, 100] {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut collector = MetricsCollector::new();

            // Pre-populate with metrics
            for idx in 0..size {
                let metrics = RequestMetrics::new(RequestMetricsParams {
                    query: format!("query_{idx}"),
                    tier_used: if idx % 3 == 0 {
                        "local"
                    } else if idx % 3 == 1 {
                        "groq"
                    } else {
                        "claude"
                    }
                    .to_owned(),
                    latency_ms: 100 + (idx % 200) as u64,
                    tokens_used: TokenUsage {
                        input: 100,
                        output: 50,
                        cache_read: 0,
                        cache_write: 0,
                    },
                    success: idx % 10 != 0,
                    escalated: idx % 5 == 0,
                });
                collector.record(metrics);
            }

            b.iter(|| {
                drop(MetricsReport::daily(black_box(&collector)));
            });
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(2))
        .warm_up_time(Duration::from_millis(500))
        .sample_size(10);
    targets = bench_metrics_recording,
             bench_daily_report_generation
}
criterion_main!(benches);
