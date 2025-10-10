//! Benchmarks for response caching performance.

#![allow(
    clippy::min_ident_chars,
    missing_docs,
    let_underscore_drop,
    reason = "Benchmarks use standard loop variables and drop values intentionally"
)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use merlin_core::{Response, TokenUsage};
use merlin_routing::ResponseCache;
use std::hint::black_box;
use std::time::Duration;

fn create_test_response(text: &str) -> Response {
    Response {
        text: text.to_owned(),
        confidence: 1.0,
        tokens_used: TokenUsage::default(),
        provider: "test".to_owned(),
        latency_ms: 100,
    }
}

fn bench_cache_put(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_put");

    for size in &[10, 100] {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut cache = ResponseCache::default();
                for i in 0..size {
                    let query = format!("query_{i}");
                    let response = create_test_response(&format!("response_{i}"));
                    cache.put(black_box(query), black_box(response));
                }
            });
        });
    }
    group.finish();
}

fn bench_cache_get_hit(c: &mut Criterion) {
    let mut cache = ResponseCache::default();

    // Pre-populate cache
    for i in 0..100 {
        let query = format!("query_{i}");
        let response = create_test_response(&format!("response_{i}"));
        cache.put(query, response);
    }

    c.bench_function("cache_get_hit", |b| {
        b.iter(|| {
            for i in 0..100 {
                let query = format!("query_{i}");
                drop(cache.get(black_box(&query)));
            }
        });
    });
}

fn bench_cache_get_miss(c: &mut Criterion) {
    let mut cache = ResponseCache::default();

    c.bench_function("cache_get_miss", |b| {
        b.iter(|| {
            for i in 0..100 {
                let query = format!("nonexistent_{i}");
                drop(cache.get(black_box(&query)));
            }
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(2))
        .warm_up_time(Duration::from_millis(500))
        .sample_size(10);
    targets = bench_cache_put,
             bench_cache_get_hit,
             bench_cache_get_miss
}
criterion_main!(benches);
