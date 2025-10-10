#!/usr/bin/env bash
# Generate benchmark dashboard HTML
set -e

OUTPUT_DIR="${1:-.}"
mkdir -p "$OUTPUT_DIR"

# Create index.html with comprehensive benchmark dashboard
cat > "$OUTPUT_DIR/index.html" <<'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Merlin Benchmarks</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            background: #0d1117;
            color: #c9d1d9;
            line-height: 1.6;
        }

        .container {
            max-width: 1400px;
            margin: 0 auto;
            padding: 2rem;
        }

        header {
            text-align: center;
            padding: 2rem 0;
            border-bottom: 2px solid #21262d;
            margin-bottom: 3rem;
        }

        h1 {
            font-size: 2.5rem;
            margin-bottom: 0.5rem;
            color: #58a6ff;
        }

        .subtitle {
            color: #8b949e;
            font-size: 1.1rem;
        }

        .benchmark-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
            gap: 2rem;
            margin-bottom: 3rem;
        }

        .card {
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 8px;
            padding: 1.5rem;
            transition: transform 0.2s, box-shadow 0.2s;
        }

        .card:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
        }

        .card-header {
            display: flex;
            align-items: center;
            margin-bottom: 1rem;
            padding-bottom: 1rem;
            border-bottom: 1px solid #21262d;
        }

        .card-icon {
            font-size: 2rem;
            margin-right: 1rem;
        }

        .card-title {
            font-size: 1.3rem;
            color: #58a6ff;
        }

        .metric {
            margin: 1rem 0;
        }

        .metric-label {
            font-size: 0.9rem;
            color: #8b949e;
            margin-bottom: 0.5rem;
        }

        .metric-value {
            font-size: 2rem;
            font-weight: bold;
            color: #58a6ff;
        }

        .metric-change {
            font-size: 0.9rem;
            margin-left: 0.5rem;
        }

        .metric-change.positive {
            color: #3fb950;
        }

        .metric-change.negative {
            color: #f85149;
        }

        .metrics-grid {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: 1rem;
            margin-top: 1rem;
        }

        .mini-metric {
            background: #0d1117;
            padding: 0.75rem;
            border-radius: 4px;
        }

        .mini-metric-value {
            font-size: 1.2rem;
            font-weight: bold;
            color: #58a6ff;
        }

        .mini-metric-label {
            font-size: 0.8rem;
            color: #8b949e;
        }

        .chart-container {
            margin-top: 1.5rem;
            height: 200px;
        }

        .loading {
            text-align: center;
            padding: 3rem;
            color: #8b949e;
            font-size: 1.1rem;
        }

        .error {
            background: #2c1616;
            border: 1px solid #f85149;
            color: #f85149;
            padding: 1rem;
            border-radius: 4px;
            margin: 1rem 0;
        }

        .status-badge {
            display: inline-block;
            padding: 0.25rem 0.75rem;
            border-radius: 12px;
            font-size: 0.85rem;
            font-weight: 600;
        }

        .status-good {
            background: #1a472a;
            color: #3fb950;
        }

        .status-warning {
            background: #4a3a1a;
            color: #d29922;
        }

        .status-bad {
            background: #2c1616;
            color: #f85149;
        }

        footer {
            text-align: center;
            padding: 2rem 0;
            color: #8b949e;
            border-top: 1px solid #21262d;
            margin-top: 3rem;
        }

        a {
            color: #58a6ff;
            text-decoration: none;
        }

        a:hover {
            text-decoration: underline;
        }

        .timestamp {
            color: #8b949e;
            font-size: 0.85rem;
        }
    </style>
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.min.js"></script>
</head>
<body>
    <div class="container">
        <header>
            <h1>🧙‍♂️ Merlin Benchmarks</h1>
            <p class="subtitle">Performance, Quality, and Memory Profiling Metrics</p>
        </header>

        <div id="loading" class="loading">
            Loading benchmark data...
        </div>

        <div id="benchmarks" style="display: none;">
            <div class="benchmark-grid">
                <!-- Performance Benchmarks Card -->
                <div class="card">
                    <div class="card-header">
                        <div class="card-icon">⚡</div>
                        <div>
                            <div class="card-title">Performance Benchmarks</div>
                            <div class="timestamp" id="perf-timestamp">-</div>
                        </div>
                    </div>
                    <div id="perf-content"></div>
                </div>

                <!-- Quality Benchmarks Card -->
                <div class="card">
                    <div class="card-header">
                        <div class="card-icon">🎯</div>
                        <div>
                            <div class="card-title">Quality Benchmarks</div>
                            <div class="timestamp" id="quality-timestamp">-</div>
                        </div>
                    </div>
                    <div id="quality-content"></div>
                </div>

                <!-- Gungraun Benchmarks Card -->
                <div class="card">
                    <div class="card-header">
                        <div class="card-icon">🔬</div>
                        <div>
                            <div class="card-title">Memory Profiling</div>
                            <div class="timestamp" id="gungraun-timestamp">-</div>
                        </div>
                    </div>
                    <div id="gungraun-content"></div>
                </div>
            </div>

            <div class="benchmark-grid">
                <!-- Performance History Chart -->
                <div class="card">
                    <div class="card-header">
                        <div class="card-title">Performance Trends</div>
                    </div>
                    <div class="chart-container">
                        <canvas id="perf-chart"></canvas>
                    </div>
                </div>

                <!-- Quality History Chart -->
                <div class="card">
                    <div class="card-header">
                        <div class="card-title">Quality Trends</div>
                    </div>
                    <div class="chart-container">
                        <canvas id="quality-chart"></canvas>
                    </div>
                </div>
            </div>
        </div>
    </div>

    <footer>
        <p>Generated by <a href="https://github.com/anthropics/merlin">Merlin</a> CI/CD Pipeline</p>
        <p><a href="https://github.com/BigBadE/agentic_optimizer">View on GitHub</a></p>
    </footer>

    <script>
        async function loadBenchmarkData() {
            try {
                // Load all benchmark data in parallel
                const [perfData, qualityData, gungraunData] = await Promise.all([
                    fetch('data/perf-latest.json').then(r => r.ok ? r.json() : null).catch(() => null),
                    fetch('data/quality-latest.json').then(r => r.ok ? r.json() : null).catch(() => null),
                    fetch('data/gungraun-latest.json').then(r => r.ok ? r.json() : null).catch(() => null)
                ]);

                document.getElementById('loading').style.display = 'none';
                document.getElementById('benchmarks').style.display = 'block';

                // Display performance benchmarks
                if (perfData) {
                    displayPerformanceBenchmarks(perfData);
                } else {
                    document.getElementById('perf-content').innerHTML = '<div class="error">No performance data available</div>';
                }

                // Display quality benchmarks
                if (qualityData) {
                    displayQualityBenchmarks(qualityData);
                } else {
                    document.getElementById('quality-content').innerHTML = '<div class="error">No quality data available</div>';
                }

                // Display gungraun benchmarks
                if (gungraunData) {
                    displayGungraunBenchmarks(gungraunData);
                } else {
                    document.getElementById('gungraun-content').innerHTML = '<div class="error">No memory profiling data available</div>';
                }

            } catch (error) {
                document.getElementById('loading').innerHTML = `<div class="error">Error loading benchmarks: ${error.message}</div>`;
            }
        }

        function displayPerformanceBenchmarks(data) {
            const metrics = data.metrics || {};
            const benchmarks = data.benchmarks || [];

            document.getElementById('perf-timestamp').textContent = new Date(data.timestamp).toLocaleString();

            let html = `
                <div class="metric">
                    <div class="metric-label">Total Benchmarks</div>
                    <div class="metric-value">${metrics.total_benchmarks || 0}</div>
                </div>
                <div class="metrics-grid">
                    <div class="mini-metric">
                        <div class="mini-metric-label">Avg Time</div>
                        <div class="mini-metric-value">${(metrics.avg_time_ms || 0).toFixed(2)} ms</div>
                    </div>
                    <div class="mini-metric">
                        <div class="mini-metric-label">Total Time</div>
                        <div class="mini-metric-value">${(metrics.total_time_ms || 0).toFixed(2)} ms</div>
                    </div>
                </div>
            `;

            if (benchmarks.length > 0) {
                html += '<div style="margin-top: 1rem; max-height: 200px; overflow-y: auto;">';
                benchmarks.slice(0, 10).forEach(bench => {
                    html += `
                        <div class="mini-metric" style="margin-bottom: 0.5rem;">
                            <div class="mini-metric-label">${bench.name}</div>
                            <div class="mini-metric-value">${(bench.mean_ms || 0).toFixed(3)} ms</div>
                        </div>
                    `;
                });
                html += '</div>';
            }

            document.getElementById('perf-content').innerHTML = html;

            // Draw chart if history available
            if (data.history && data.history.length > 1) {
                drawPerformanceChart(data.history);
            }
        }

        function displayQualityBenchmarks(data) {
            const metrics = data.metrics || {};

            document.getElementById('quality-timestamp').textContent = new Date(data.timestamp).toLocaleString();

            const getStatus = (value, target) => {
                const percent = (value / target) * 100;
                if (percent >= 90) return 'status-good';
                if (percent >= 70) return 'status-warning';
                return 'status-bad';
            };

            let html = `
                <div class="metric">
                    <div class="metric-label">Test Cases</div>
                    <div class="metric-value">${metrics.test_cases || 0}</div>
                </div>
                <div class="metrics-grid">
                    <div class="mini-metric">
                        <div class="mini-metric-label">Precision@3 (target: 60%)</div>
                        <div class="mini-metric-value">
                            ${(metrics.precision_at_3 || 0).toFixed(1)}%
                            <span class="status-badge ${getStatus(metrics.precision_at_3 || 0, 60)}">
                                ${(metrics.precision_at_3 || 0) >= 54 ? '✓' : '⚠'}
                            </span>
                        </div>
                    </div>
                    <div class="mini-metric">
                        <div class="mini-metric-label">Recall@10 (target: 70%)</div>
                        <div class="mini-metric-value">
                            ${(metrics.recall_at_10 || 0).toFixed(1)}%
                            <span class="status-badge ${getStatus(metrics.recall_at_10 || 0, 70)}">
                                ${(metrics.recall_at_10 || 0) >= 63 ? '✓' : '⚠'}
                            </span>
                        </div>
                    </div>
                    <div class="mini-metric">
                        <div class="mini-metric-label">MRR (target: 0.700)</div>
                        <div class="mini-metric-value">
                            ${(metrics.mrr || 0).toFixed(3)}
                            <span class="status-badge ${getStatus((metrics.mrr || 0) * 100, 70)}">
                                ${(metrics.mrr || 0) >= 0.63 ? '✓' : '⚠'}
                            </span>
                        </div>
                    </div>
                    <div class="mini-metric">
                        <div class="mini-metric-label">NDCG@10 (target: 0.750)</div>
                        <div class="mini-metric-value">
                            ${(metrics.ndcg_at_10 || 0).toFixed(3)}
                            <span class="status-badge ${getStatus((metrics.ndcg_at_10 || 0) * 100, 75)}">
                                ${(metrics.ndcg_at_10 || 0) >= 0.675 ? '✓' : '⚠'}
                            </span>
                        </div>
                    </div>
                </div>
            `;

            document.getElementById('quality-content').innerHTML = html;

            // Draw chart if history available
            if (data.history && data.history.length > 1) {
                drawQualityChart(data.history);
            }
        }

        function displayGungraunBenchmarks(data) {
            const metrics = data.metrics || {};
            const benchmarks = data.benchmarks || [];

            document.getElementById('gungraun-timestamp').textContent = new Date(data.timestamp).toLocaleString();

            let html = `
                <div class="metric">
                    <div class="metric-label">Total Benchmarks</div>
                    <div class="metric-value">${metrics.total_benchmarks || 0}</div>
                </div>
                <div class="metrics-grid">
                    <div class="mini-metric">
                        <div class="mini-metric-label">Avg Instructions</div>
                        <div class="mini-metric-value">${formatNumber(metrics.avg_instructions || 0)}</div>
                    </div>
                    <div class="mini-metric">
                        <div class="mini-metric-label">Avg Cycles</div>
                        <div class="mini-metric-value">${formatNumber(metrics.avg_cycles || 0)}</div>
                    </div>
                </div>
            `;

            if (benchmarks.length > 0) {
                html += '<div style="margin-top: 1rem; max-height: 200px; overflow-y: auto;">';
                benchmarks.slice(0, 10).forEach(bench => {
                    html += `
                        <div class="mini-metric" style="margin-bottom: 0.5rem;">
                            <div class="mini-metric-label">${bench.name}</div>
                            <div class="mini-metric-value">${formatNumber(bench.instructions || 0)} inst</div>
                        </div>
                    `;
                });
                html += '</div>';
            }

            document.getElementById('gungraun-content').innerHTML = html;
        }

        function formatNumber(num) {
            if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
            if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
            return num.toString();
        }

        function drawPerformanceChart(history) {
            const ctx = document.getElementById('perf-chart');
            if (!ctx) return;

            const labels = history.map(h => new Date(h.timestamp).toLocaleDateString());
            const avgTimes = history.map(h => h.avg_time_ms || 0);

            new Chart(ctx, {
                type: 'line',
                data: {
                    labels: labels,
                    datasets: [{
                        label: 'Average Time (ms)',
                        data: avgTimes,
                        borderColor: '#58a6ff',
                        backgroundColor: 'rgba(88, 166, 255, 0.1)',
                        tension: 0.1
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {
                        legend: {
                            labels: { color: '#c9d1d9' }
                        }
                    },
                    scales: {
                        y: {
                            ticks: { color: '#8b949e' },
                            grid: { color: '#21262d' }
                        },
                        x: {
                            ticks: { color: '#8b949e' },
                            grid: { color: '#21262d' }
                        }
                    }
                }
            });
        }

        function drawQualityChart(history) {
            const ctx = document.getElementById('quality-chart');
            if (!ctx) return;

            const labels = history.map(h => new Date(h.timestamp).toLocaleDateString());
            const precisionData = history.map(h => h.precision_at_3 || 0);
            const recallData = history.map(h => h.recall_at_10 || 0);
            const mrrData = history.map(h => (h.mrr || 0) * 100);

            new Chart(ctx, {
                type: 'line',
                data: {
                    labels: labels,
                    datasets: [
                        {
                            label: 'Precision@3 (%)',
                            data: precisionData,
                            borderColor: '#58a6ff',
                            backgroundColor: 'rgba(88, 166, 255, 0.1)',
                            tension: 0.1
                        },
                        {
                            label: 'Recall@10 (%)',
                            data: recallData,
                            borderColor: '#3fb950',
                            backgroundColor: 'rgba(63, 185, 80, 0.1)',
                            tension: 0.1
                        },
                        {
                            label: 'MRR (scaled %)',
                            data: mrrData,
                            borderColor: '#d29922',
                            backgroundColor: 'rgba(210, 153, 34, 0.1)',
                            tension: 0.1
                        }
                    ]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {
                        legend: {
                            labels: { color: '#c9d1d9' }
                        }
                    },
                    scales: {
                        y: {
                            ticks: { color: '#8b949e' },
                            grid: { color: '#21262d' }
                        },
                        x: {
                            ticks: { color: '#8b949e' },
                            grid: { color: '#21262d' }
                        }
                    }
                }
            });
        }

        // Load data when page loads
        loadBenchmarkData();
    </script>
</body>
</html>
EOF

echo "✅ Dashboard generated at $OUTPUT_DIR/index.html"
