//! Timing and benchmarking infrastructure for fixture tests.
//!
//! Provides hierarchical span-based tracing with automatic timing collection.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{Id as SpanId, Subscriber, span};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

/// Timing data for a single span
#[derive(Debug, Clone)]
pub struct SpanTiming {
    /// Span name
    pub name: String,
    /// Start time
    pub start: Instant,
    /// Duration (set when span closes)
    pub duration: Option<Duration>,
    /// Parent span ID
    pub parent_id: Option<SpanId>,
    /// Metadata about the span
    pub metadata: HashMap<String, String>,
}

impl Default for SpanTiming {
    /// Creates a new `SpanTiming` with default values
    ///
    /// Default values:
    /// - Empty name
    /// - Current instant as start time
    /// - No duration (not yet completed)
    /// - No parent span
    /// - Empty metadata
    fn default() -> Self {
        Self {
            name: String::new(),
            start: Instant::now(),
            duration: None,
            parent_id: None,
            metadata: HashMap::new(),
        }
    }
}

/// Collected timing data from all spans
#[derive(Debug, Clone, Default)]
pub struct TimingData {
    /// All span timings by ID
    pub spans: HashMap<SpanId, SpanTiming>,
    /// Root span IDs (no parent)
    pub roots: Vec<SpanId>,
}

impl TimingData {
    /// Print hierarchical timing report
    pub fn print_report(&self) {
        use tracing;
        tracing::debug!("\n=== Timing Report ===");
        for root_id in &self.roots {
            self.print_span_tree(root_id, 0);
        }
        tracing::debug!("=====================\n");
    }

    /// Print a span and its children recursively
    fn print_span_tree(&self, span_id: &SpanId, depth: usize) {
        use tracing;
        if let Some(timing) = self.spans.get(span_id) {
            let indent = "  ".repeat(depth);
            let duration_str = timing.duration.map_or_else(
                || "RUNNING".to_owned(),
                |dur| format!("{:.3}s", dur.as_secs_f64()),
            );

            tracing::debug!("{indent}{}: {duration_str}", timing.name);

            // Print children
            for (child_id, child_timing) in &self.spans {
                if child_timing.parent_id.as_ref() == Some(span_id) {
                    self.print_span_tree(child_id, depth + 1);
                }
            }
        }
    }

    /// Get total duration for a named span (sum across all instances)
    pub fn total_duration_for(&self, name: &str) -> Duration {
        self.spans
            .values()
            .filter(|timing| timing.name == name)
            .filter_map(|timing| timing.duration)
            .sum()
    }

    /// Get count of spans with given name
    pub fn count_for(&self, name: &str) -> usize {
        self.spans
            .values()
            .filter(|timing| timing.name == name)
            .count()
    }

    /// Export to JSON for external analysis
    #[must_use]
    pub fn to_json(&self) -> String {
        // Simple JSON generation (could use serde_json for production)
        let mut json = String::from("{\n  \"spans\": [\n");

        for (id, timing) in &self.spans {
            let duration = timing.duration.map_or(0.0, |dur| dur.as_secs_f64());
            let parent_id_str = timing.parent_id.as_ref().map_or_else(
                || "null".to_owned(),
                |parent| format!("{}", parent.into_u64()),
            );

            let entry = format!(
                "    {{\"id\": {}, \"name\": \"{}\", \"duration\": {}, \"parent\": {}}},\n",
                id.into_u64(),
                timing.name,
                duration,
                parent_id_str
            );
            json.push_str(&entry);
        }

        if !self.spans.is_empty() {
            json.pop(); // Remove trailing comma
            json.pop(); // Remove trailing newline
            json.push('\n');
        }

        json.push_str("  ]\n}");
        json
    }
}

/// Tracing layer that collects timing data
pub struct TimingLayer {
    /// Shared timing data
    data: Arc<Mutex<TimingData>>,
}

impl TimingLayer {
    /// Create new timing layer
    #[must_use]
    pub fn new() -> (Self, Arc<Mutex<TimingData>>) {
        let data = Arc::new(Mutex::new(TimingData::default()));
        (
            Self {
                data: Arc::clone(&data),
            },
            data,
        )
    }
}

impl Default for TimingLayer {
    fn default() -> Self {
        let (layer, _) = Self::new();
        layer
    }
}

impl<S> Layer<S> for TimingLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, _ctx: Context<'_, S>) {
        let Ok(mut data) = self.data.lock() else {
            return;
        };

        let timing = SpanTiming {
            name: attrs.metadata().name().to_owned(),
            parent_id: attrs.parent().cloned(),
            ..Default::default()
        };

        // Track root spans
        if timing.parent_id.is_none() {
            data.roots.push(id.clone());
        }

        data.spans.insert(id.clone(), timing);
    }

    fn on_close(&self, id: span::Id, _ctx: Context<'_, S>) {
        let Ok(mut data) = self.data.lock() else {
            return;
        };

        if let Some(timing) = data.spans.get_mut(&id) {
            timing.duration = Some(timing.start.elapsed());
        }
    }
}

/// Helper macro to create a timed span
#[macro_export]
macro_rules! timed_span {
    ($name:expr) => {
        tracing::span!(tracing::Level::INFO, $name)
    };
    ($name:expr, $($field:tt)*) => {
        tracing::span!(tracing::Level::INFO, $name, $($field)*)
    };
}
