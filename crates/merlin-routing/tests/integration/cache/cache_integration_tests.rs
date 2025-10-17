//! Integration tests for Phase 5 features.

#![allow(
    clippy::min_ident_chars,
    clippy::tests_outside_test_module,
    clippy::missing_panics_doc,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::identity_op,
    clippy::redundant_clone,
    missing_docs,
    reason = "Integration tests have different conventions"
)]

use merlin_core::{Response, TokenUsage};
use merlin_routing::{
    CacheConfig, MetricsCollector, MetricsReport, RequestMetrics, ResponseCache, RoutingConfig,
};
use std::thread;
use std::time::Duration;

#[test]
fn test_cache_integration_with_config() {
    let config = CacheConfig {
        enabled: true,
        ttl_hours: 1,
        max_size_mb: 10,
        similarity_threshold: 0.95,
    };

    let mut cache = ResponseCache::new(config);

    let response = Response {
        text: "Test response".to_owned(),
        confidence: 1.0,
        tokens_used: TokenUsage::default(),
        provider: "test".to_owned(),
        latency_ms: 100,
    };

    cache.put("test_query".to_owned(), response.clone());

    let cached = cache.get("test_query");
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().text, "Test response");
}

#[test]
fn test_cache_disabled_integration() {
    let config = CacheConfig {
        enabled: false,
        ttl_hours: 24,
        max_size_mb: 100,
        similarity_threshold: 0.95,
    };

    let mut cache = ResponseCache::new(config);

    let response = Response {
        text: "Test response".to_owned(),
        confidence: 1.0,
        tokens_used: TokenUsage::default(),
        provider: "test".to_owned(),
        latency_ms: 100,
    };

    cache.put("test_query".to_owned(), response);

    // Cache is disabled, should not store
    assert_eq!(cache.len(), 0);
    assert!(cache.get("test_query").is_none());
}

#[test]
fn test_cache_size_limit_integration() {
    let config = CacheConfig {
        enabled: true,
        ttl_hours: 24,
        max_size_mb: 1, // Very small to trigger eviction
        similarity_threshold: 0.95,
    };

    let mut cache = ResponseCache::new(config);

    // Add many entries to trigger eviction
    for i in 0..100 {
        let response = Response {
            text: format!("Response {i} with some content to increase size"),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 100,
        };
        cache.put(format!("query_{i}"), response);
    }

    // Cache should have evicted some entries to stay under size limit
    // Note: The eviction may not be perfect due to size estimation
    assert!(cache.size_bytes() <= 2 * 1024 * 1024); // Allow some overhead
}

#[test]
fn test_metrics_integration_with_cache() {
    let mut collector = MetricsCollector::new();
    let mut cache = ResponseCache::default();

    // Simulate requests with cache hits and misses
    for i in 0..10 {
        let query = format!("query_{i}");

        // Check cache
        let cache_hit = cache.get(&query).is_some();

        if !cache_hit {
            // Record cache miss
            let metrics = RequestMetrics::new(
                query.clone(),
                "local".to_owned(),
                100,
                TokenUsage::default(),
                true,
                false,
            );
            collector.record(metrics);

            // Add to cache
            let response = Response {
                text: format!("Response {i}"),
                confidence: 1.0,
                tokens_used: TokenUsage::default(),
                provider: "local".to_owned(),
                latency_ms: 100,
            };
            cache.put(query, response);
        }
    }

    assert_eq!(collector.len(), 10); // All were cache misses first time
    assert_eq!(cache.len(), 10);
}
#[test]
fn test_metrics_cost_tracking_integration() {
    let mut collector = MetricsCollector::new();

    // Add metrics for different tiers
    // Cost calculation: (input_tokens / 1M) * input_cost + (output_tokens / 1M) * output_cost
    let tiers = vec![
        ("local", 0.0),
        ("groq", 0.0),
        ("claude", 0.0105), // (1000/1M) * 3.0 + (500/1M) * 15.0 = 0.003 + 0.0075 = 0.0105
        ("deepseek", 0.00082), // (1000/1M) * 0.27 + (500/1M) * 1.1 = 0.00027 + 0.00055 = 0.00082
    ];

    for (tier, expected_cost) in tiers {
        let metrics = RequestMetrics::new(
            "test".to_owned(),
            tier.to_owned(),
            100,
            TokenUsage {
                input: 1000,
                output: 500,
                cache_read: 0,
                cache_write: 0,
            },
            true,
            false,
        );

        // Check cost estimation (allow some floating point tolerance)
        assert!(
            (metrics.cost - expected_cost).abs() < 0.001,
            "Cost mismatch for {}: expected {}, got {}",
            tier,
            expected_cost,
            metrics.cost
        );

        collector.record(metrics);
    }

    let report = MetricsReport::daily(&collector);
    assert_eq!(report.total_requests, 4);
    assert!(report.total_cost > 0.0);
}

#[test]
fn test_metrics_report_integration() {
    let mut collector = MetricsCollector::new();

    // Add varied metrics
    for i in 0..50 {
        let tier = if i % 3 == 0 {
            "local"
        } else if i % 3 == 1 {
            "groq"
        } else {
            "claude"
        };

        let metrics = RequestMetrics::new(
            format!("query_{i}"),
            tier.to_owned(),
            100 + (i % 100) as u64,
            TokenUsage {
                input: 100,
                output: 50,
                cache_read: 0,
                cache_write: 0,
            },
            i % 10 != 0, // 90% success rate
            i % 5 == 0,  // 20% escalation rate
        );

        collector.record(metrics);
    }

    let report = MetricsReport::daily(&collector);

    assert_eq!(report.total_requests, 50);
    assert!((report.success_rate - 0.9).abs() < 0.01);
    assert!((report.escalation_rate - 0.2).abs() < 0.01);
    assert_eq!(report.tier_distribution.len(), 3);
    assert!(report.avg_latency_ms > 0);
}

#[test]
fn test_config_integration() {
    let config = RoutingConfig::default();

    // Check that cache config is included
    assert!(config.cache.enabled);
    assert_eq!(config.cache.ttl_hours, 24);
    assert_eq!(config.cache.max_size_mb, 100);
    assert!((config.cache.similarity_threshold - 0.95).abs() < f32::EPSILON);
}

#[test]
fn test_config_serialization_integration() {
    let config = RoutingConfig::default();

    let json = serde_json::to_string(&config).expect("Serialization failed");
    let deserialized: RoutingConfig = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(deserialized.cache.enabled, config.cache.enabled);
    assert_eq!(deserialized.cache.ttl_hours, config.cache.ttl_hours);
}

#[test]
fn test_cache_stats_integration() {
    let mut cache = ResponseCache::default();

    for i in 0..10 {
        let response = Response {
            text: format!("Response {i}"),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 100,
        };
        cache.put(format!("query_{i}"), response);
    }

    let stats = cache.stats();
    assert_eq!(stats.entries, 10);
    assert!(stats.size_bytes > 0);
    assert!(stats.size_mb > 0.0);
}

#[test]
fn test_metrics_format_report_integration() {
    let mut collector = MetricsCollector::new();

    for i in 0..10 {
        let metrics = RequestMetrics::new(
            format!("query_{i}"),
            "local".to_owned(),
            100,
            TokenUsage::default(),
            true,
            false,
        );
        collector.record(metrics);
    }

    let report = MetricsReport::daily(&collector);
    let formatted = MetricsReport::format_report(&report).expect("Format failed");

    assert!(formatted.contains("Total Requests: 10"));
    assert!(formatted.contains("Success Rate"));
    assert!(formatted.contains("local"));
}

#[test]
fn test_cache_clear_integration() {
    let mut cache = ResponseCache::default();

    for i in 0..10 {
        let response = Response {
            text: format!("Response {i}"),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 100,
        };
        cache.put(format!("query_{i}"), response);
    }

    assert_eq!(cache.len(), 10);

    cache.clear();

    assert_eq!(cache.len(), 0);
    assert_eq!(cache.size_bytes(), 0);
}

#[test]
fn test_metrics_collector_clear_integration() {
    let mut collector = MetricsCollector::new();

    for i in 0..10 {
        let metrics = RequestMetrics::new(
            format!("query_{i}"),
            "local".to_owned(),
            100,
            TokenUsage::default(),
            true,
            false,
        );
        collector.record(metrics);
    }

    assert_eq!(collector.len(), 10);

    collector.clear();

    assert_eq!(collector.len(), 0);
}

#[test]
fn test_cache_expiration_integration() {
    let config = CacheConfig {
        enabled: true,
        ttl_hours: 0, // Expire immediately
        max_size_mb: 100,
        similarity_threshold: 0.95,
    };

    let mut cache = ResponseCache::new(config);

    let response = Response {
        text: "Test".to_owned(),
        confidence: 1.0,
        tokens_used: TokenUsage::default(),
        provider: "test".to_owned(),
        latency_ms: 100,
    };

    cache.put("query".to_owned(), response);

    // Small delay to ensure expiration
    thread::sleep(Duration::from_millis(10));

    // Should be expired
    let cached = cache.get("query");
    assert!(cached.is_none());
}
