//! Report generation for metrics analysis.

use super::collector::{MetricsCollector, RequestMetrics};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Error as FmtError, Write as _};

/// Breakdown of requests by tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierBreakdown {
    /// Tier name
    pub tier: String,
    /// Number of requests
    pub count: usize,
    /// Percentage of total requests
    pub percentage: f64,
    /// Total cost for this tier
    pub total_cost: f64,
}

/// Daily metrics report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReport {
    /// Total number of requests
    pub total_requests: usize,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: u64,
    /// Total cost in USD
    pub total_cost: f64,
    /// Breakdown by tier
    pub tier_distribution: Vec<TierBreakdown>,
    /// Escalation rate (0.0 to 1.0)
    pub escalation_rate: f64,
}

/// Metrics report generator
pub struct MetricsReport;

impl MetricsReport {
    /// Generates a daily report from the collector
    pub fn daily(collector: &MetricsCollector) -> DailyReport {
        let requests = collector.requests_today();

        if requests.is_empty() {
            return DailyReport {
                total_requests: 0,
                success_rate: 0.0,
                avg_latency_ms: 0,
                total_cost: 0.0,
                tier_distribution: Vec::new(),
                escalation_rate: 0.0,
            };
        }

        let total_requests = requests.len();
        let successful = requests.iter().filter(|req| req.success).count();
        let success_rate = successful as f64 / total_requests as f64;

        let total_latency: u64 = requests.iter().map(|req| req.latency_ms).sum();
        let avg_latency_ms = total_latency / total_requests as u64;

        let total_cost: f64 = requests.iter().map(|req| req.cost).sum();

        let escalated = requests.iter().filter(|req| req.escalated).count();
        let escalation_rate = escalated as f64 / total_requests as f64;

        let tier_distribution = Self::tier_breakdown(&requests);

        DailyReport {
            total_requests,
            success_rate,
            avg_latency_ms,
            total_cost,
            tier_distribution,
            escalation_rate,
        }
    }

    /// Generates a weekly report from the collector
    pub fn weekly(collector: &MetricsCollector) -> DailyReport {
        let requests = collector.requests_this_week();

        if requests.is_empty() {
            return DailyReport {
                total_requests: 0,
                success_rate: 0.0,
                avg_latency_ms: 0,
                total_cost: 0.0,
                tier_distribution: Vec::new(),
                escalation_rate: 0.0,
            };
        }

        let total_requests = requests.len();
        let successful = requests.iter().filter(|req| req.success).count();
        let success_rate = successful as f64 / total_requests as f64;

        let total_latency: u64 = requests.iter().map(|req| req.latency_ms).sum();
        let avg_latency_ms = total_latency / total_requests as u64;

        let total_cost: f64 = requests.iter().map(|req| req.cost).sum();

        let escalated = requests.iter().filter(|req| req.escalated).count();
        let escalation_rate = escalated as f64 / total_requests as f64;

        let tier_distribution = Self::tier_breakdown(&requests);

        DailyReport {
            total_requests,
            success_rate,
            avg_latency_ms,
            total_cost,
            tier_distribution,
            escalation_rate,
        }
    }

    /// Calculates tier breakdown from requests
    fn tier_breakdown(requests: &[&RequestMetrics]) -> Vec<TierBreakdown> {
        let mut tier_counts: HashMap<String, usize> = HashMap::new();
        let mut tier_costs: HashMap<String, f64> = HashMap::new();

        for request in requests {
            *tier_counts.entry(request.tier_used.clone()).or_insert(0) += 1;
            *tier_costs.entry(request.tier_used.clone()).or_insert(0.0) += request.cost;
        }

        let total = requests.len();

        tier_counts
            .into_iter()
            .map(|(tier, count)| {
                let percentage = (count as f64 / total as f64) * 100.0;
                let total_cost = tier_costs.get(&tier).copied().unwrap_or(0.0);

                TierBreakdown {
                    tier,
                    count,
                    percentage,
                    total_cost,
                }
            })
            .collect()
    }

    /// Formats a report as a human-readable string
    ///
    /// # Errors
    /// Returns an error if formatting fails
    pub fn format_report(report: &DailyReport) -> Result<String, FmtError> {
        let mut output = String::new();

        writeln!(output, "Total Requests: {}", report.total_requests)?;
        writeln!(output, "Success Rate: {:.1}%", report.success_rate * 100.0)?;
        writeln!(output, "Average Latency: {}ms", report.avg_latency_ms)?;
        writeln!(output, "Total Cost: ${:.4}", report.total_cost)?;
        writeln!(
            output,
            "Escalation Rate: {:.1}%",
            report.escalation_rate * 100.0
        )?;

        writeln!(output, "\nTier Distribution:")?;
        for tier in &report.tier_distribution {
            writeln!(
                output,
                "  {}: {} requests ({:.1}%) - ${:.4}",
                tier.tier, tier.count, tier.percentage, tier.total_cost
            )?;
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::collector::RequestMetricsParams;
    use merlin_core::TokenUsage;

    #[test]
    fn test_daily_report_empty() {
        let collector = MetricsCollector::new();
        let report = MetricsReport::daily(&collector);

        assert_eq!(report.total_requests, 0);
        assert!((report.success_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_daily_report_with_data() {
        let mut collector = MetricsCollector::new();

        let metrics = RequestMetrics::new(RequestMetricsParams {
            query: "test".to_owned(),
            tier_used: "local".to_owned(),
            latency_ms: 100,
            tokens_used: TokenUsage::default(),
            success: true,
            escalated: false,
        });

        collector.record(metrics);

        let report = MetricsReport::daily(&collector);

        assert_eq!(report.total_requests, 1);
        assert!((report.success_rate - 1.0).abs() < f64::EPSILON);
        assert_eq!(report.avg_latency_ms, 100);
    }

    #[test]
    fn test_format_report() -> Result<(), FmtError> {
        let report = DailyReport {
            total_requests: 10,
            success_rate: 0.9,
            avg_latency_ms: 150,
            total_cost: 0.05,
            tier_distribution: vec![TierBreakdown {
                tier: "local".to_owned(),
                count: 10,
                percentage: 100.0,
                total_cost: 0.0,
            }],
            escalation_rate: 0.1,
        };

        let formatted = MetricsReport::format_report(&report)?;
        assert!(formatted.contains("Total Requests: 10"));
        assert!(formatted.contains("Success Rate: 90.0%"));
        Ok(())
    }
}
