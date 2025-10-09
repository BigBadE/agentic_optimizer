#!/usr/bin/env bash
# Generate web-based benchmark dashboard for gh-pages
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${1:-gh-pages}"

echo "📊 Generating benchmark dashboard..."
echo "Output directory: $OUTPUT_DIR"

# Create directory structure
mkdir -p "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/data"
mkdir -p "$OUTPUT_DIR/assets"

# Generate main dashboard HTML
cat > "$OUTPUT_DIR/index.html" <<'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Merlin Benchmark Dashboard</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
        }
        
        .container {
            max-width: 1400px;
            margin: 0 auto;
        }
        
        header {
            background: white;
            padding: 30px;
            border-radius: 12px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
            margin-bottom: 30px;
        }
        
        h1 {
            color: #333;
            font-size: 2.5em;
            margin-bottom: 10px;
        }
        
        .subtitle {
            color: #666;
            font-size: 1.1em;
        }
        
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        
        .stat-card {
            background: white;
            padding: 25px;
            border-radius: 12px;
            box-shadow: 0 5px 15px rgba(0,0,0,0.1);
            transition: transform 0.3s ease, box-shadow 0.3s ease;
        }
        
        .stat-card:hover {
            transform: translateY(-5px);
            box-shadow: 0 10px 25px rgba(0,0,0,0.15);
        }
        
        .stat-label {
            color: #888;
            font-size: 0.9em;
            text-transform: uppercase;
            letter-spacing: 1px;
            margin-bottom: 10px;
        }
        
        .stat-value {
            color: #333;
            font-size: 2.5em;
            font-weight: bold;
            margin-bottom: 5px;
        }
        
        .stat-change {
            font-size: 0.9em;
            padding: 4px 8px;
            border-radius: 4px;
            display: inline-block;
        }
        
        .stat-change.positive {
            background: #d4edda;
            color: #155724;
        }
        
        .stat-change.negative {
            background: #f8d7da;
            color: #721c24;
        }
        
        .stat-change.neutral {
            background: #e2e3e5;
            color: #383d41;
        }
        
        .benchmark-section {
            background: white;
            padding: 30px;
            border-radius: 12px;
            box-shadow: 0 5px 15px rgba(0,0,0,0.1);
            margin-bottom: 30px;
        }
        
        .section-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 20px;
            padding-bottom: 15px;
            border-bottom: 2px solid #f0f0f0;
        }
        
        .section-title {
            font-size: 1.8em;
            color: #333;
        }
        
        .section-badge {
            background: #667eea;
            color: white;
            padding: 8px 16px;
            border-radius: 20px;
            font-size: 0.9em;
            font-weight: 600;
        }
        
        .chart-container {
            height: 400px;
            margin: 20px 0;
        }
        
        .tabs {
            display: flex;
            gap: 10px;
            margin-bottom: 20px;
            border-bottom: 2px solid #f0f0f0;
        }
        
        .tab {
            padding: 12px 24px;
            background: none;
            border: none;
            cursor: pointer;
            font-size: 1em;
            color: #666;
            border-bottom: 3px solid transparent;
            transition: all 0.3s ease;
        }
        
        .tab:hover {
            color: #667eea;
        }
        
        .tab.active {
            color: #667eea;
            border-bottom-color: #667eea;
            font-weight: 600;
        }
        
        .tab-content {
            display: none;
        }
        
        .tab-content.active {
            display: block;
        }
        
        .metric-table {
            width: 100%;
            border-collapse: collapse;
            margin-top: 20px;
        }
        
        .metric-table th,
        .metric-table td {
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #f0f0f0;
        }
        
        .metric-table th {
            background: #f8f9fa;
            font-weight: 600;
            color: #333;
        }
        
        .metric-table tr:hover {
            background: #f8f9fa;
        }
        
        .loading {
            text-align: center;
            padding: 40px;
            color: #666;
        }
        
        .spinner {
            border: 3px solid #f3f3f3;
            border-top: 3px solid #667eea;
            border-radius: 50%;
            width: 40px;
            height: 40px;
            animation: spin 1s linear infinite;
            margin: 0 auto 20px;
        }
        
        @keyframes spin {
            0% { transform: rotate(0deg); }
            100% { transform: rotate(360deg); }
        }
        
        .error {
            background: #f8d7da;
            color: #721c24;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
        }
        
        footer {
            text-align: center;
            color: white;
            padding: 20px;
            margin-top: 40px;
        }
        
        .timestamp {
            color: rgba(255,255,255,0.8);
            font-size: 0.9em;
        }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>🚀 Merlin Benchmark Dashboard</h1>
            <p class="subtitle">Real-time performance tracking across all benchmark suites</p>
        </header>
        
        <div class="stats-grid" id="statsGrid">
            <div class="loading">
                <div class="spinner"></div>
                <p>Loading benchmark data...</p>
            </div>
        </div>
        
        <div class="benchmark-section">
            <div class="section-header">
                <h2 class="section-title">📈 Performance Trends</h2>
                <span class="section-badge">Last 30 Days</span>
            </div>
            
            <div class="tabs">
                <button class="tab active" onclick="switchTab('quality')">Quality Benchmarks</button>
                <button class="tab" onclick="switchTab('performance')">Performance</button>
                <button class="tab" onclick="switchTab('gungraun')">Gungraun (Memory)</button>
            </div>
            
            <div id="quality" class="tab-content active">
                <div class="chart-container">
                    <canvas id="qualityChart"></canvas>
                </div>
                <table class="metric-table" id="qualityTable">
                    <thead>
                        <tr>
                            <th>Metric</th>
                            <th>Current</th>
                            <th>Previous</th>
                            <th>Change</th>
                        </tr>
                    </thead>
                    <tbody id="qualityTableBody">
                        <tr><td colspan="4" class="loading">Loading...</td></tr>
                    </tbody>
                </table>
            </div>
            
            <div id="performance" class="tab-content">
                <div class="chart-container">
                    <canvas id="performanceChart"></canvas>
                </div>
                <table class="metric-table" id="performanceTable">
                    <thead>
                        <tr>
                            <th>Metric</th>
                            <th>Current</th>
                            <th>Previous</th>
                            <th>Change</th>
                        </tr>
                    </thead>
                    <tbody id="performanceTableBody">
                        <tr><td colspan="4" class="loading">Loading...</td></tr>
                    </tbody>
                </table>
            </div>
            
            <div id="gungraun" class="tab-content">
                <div class="chart-container">
                    <canvas id="gungraunChart"></canvas>
                </div>
                <table class="metric-table" id="gungraunTable">
                    <thead>
                        <tr>
                            <th>Metric</th>
                            <th>Current</th>
                            <th>Previous</th>
                            <th>Change</th>
                        </tr>
                    </thead>
                    <tbody id="gungraunTableBody">
                        <tr><td colspan="4" class="loading">Loading...</td></tr>
                    </tbody>
                </table>
            </div>
        </div>
        
        <div class="benchmark-section">
            <div class="section-header">
                <h2 class="section-title">📊 Detailed Results</h2>
            </div>
            <div id="detailedResults">
                <p class="loading">Loading detailed results...</p>
            </div>
        </div>
    </div>
    
    <footer>
        <p class="timestamp" id="lastUpdated">Last updated: Loading...</p>
        <p>Generated by Merlin Benchmark Tracker</p>
    </footer>
    
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.min.js"></script>
    <script src="assets/dashboard.js"></script>
</body>
</html>
EOF

# Generate dashboard JavaScript
cat > "$OUTPUT_DIR/assets/dashboard.js" <<'EOF'
// Benchmark Dashboard JavaScript

let charts = {};

// Switch between tabs
function switchTab(tabName) {
    // Update tab buttons
    document.querySelectorAll('.tab').forEach(tab => {
        tab.classList.remove('active');
    });
    event.target.classList.add('active');
    
    // Update tab content
    document.querySelectorAll('.tab-content').forEach(content => {
        content.classList.remove('active');
    });
    document.getElementById(tabName).classList.add('active');
}

// Format number with commas
function formatNumber(num) {
    return num.toLocaleString();
}

// Format percentage
function formatPercent(num) {
    return num.toFixed(2) + '%';
}

// Calculate change percentage
function calculateChange(current, previous) {
    if (!previous || previous === 0) return 0;
    return ((current - previous) / previous) * 100;
}

// Get change class (positive/negative/neutral)
function getChangeClass(change, inverse = false) {
    if (Math.abs(change) < 0.1) return 'neutral';
    if (inverse) {
        return change < 0 ? 'positive' : 'negative';
    }
    return change > 0 ? 'positive' : 'negative';
}

// Load benchmark data
async function loadBenchmarkData() {
    try {
        const [qualityData, perfData, gungraunData] = await Promise.all([
            fetch('data/quality-latest.json').then(r => r.ok ? r.json() : null).catch(() => null),
            fetch('data/perf-latest.json').then(r => r.ok ? r.json() : null).catch(() => null),
            fetch('data/gungraun-latest.json').then(r => r.ok ? r.json() : null).catch(() => null)
        ]);
        
        updateStatsGrid(qualityData, perfData, gungraunData);
        updateCharts(qualityData, perfData, gungraunData);
        updateTables(qualityData, perfData, gungraunData);
        updateDetailedResults(qualityData, perfData, gungraunData);
        
        // Update timestamp
        const timestamp = new Date().toLocaleString();
        document.getElementById('lastUpdated').textContent = `Last updated: ${timestamp}`;
        
    } catch (error) {
        console.error('Error loading benchmark data:', error);
        showError('Failed to load benchmark data. Please try again later.');
    }
}

// Update stats grid
function updateStatsGrid(quality, perf, gungraun) {
    const statsGrid = document.getElementById('statsGrid');
    
    const stats = [];
    
    if (quality && quality.metrics) {
        stats.push({
            label: 'Quality Score',
            value: quality.metrics.avg_score || 'N/A',
            change: quality.metrics.change || 0,
            inverse: false
        });
        stats.push({
            label: 'Success Rate',
            value: formatPercent(quality.metrics.success_rate || 0),
            change: quality.metrics.success_rate_change || 0,
            inverse: false
        });
    }
    
    if (perf && perf.metrics) {
        stats.push({
            label: 'Avg Latency',
            value: (perf.metrics.avg_latency_ms || 0).toFixed(2) + 'ms',
            change: perf.metrics.latency_change || 0,
            inverse: true
        });
        stats.push({
            label: 'Throughput',
            value: formatNumber(perf.metrics.throughput || 0) + ' req/s',
            change: perf.metrics.throughput_change || 0,
            inverse: false
        });
    }
    
    if (gungraun && gungraun.metrics) {
        stats.push({
            label: 'Peak Memory',
            value: (gungraun.metrics.peak_memory_mb || 0).toFixed(2) + ' MB',
            change: gungraun.metrics.memory_change || 0,
            inverse: true
        });
        stats.push({
            label: 'Instructions',
            value: formatNumber(gungraun.metrics.total_instructions || 0),
            change: gungraun.metrics.instructions_change || 0,
            inverse: true
        });
    }
    
    if (stats.length === 0) {
        statsGrid.innerHTML = '<div class="loading"><p>No benchmark data available yet. Run benchmarks to populate this dashboard.</p></div>';
        return;
    }
    
    statsGrid.innerHTML = stats.map(stat => `
        <div class="stat-card">
            <div class="stat-label">${stat.label}</div>
            <div class="stat-value">${stat.value}</div>
            <span class="stat-change ${getChangeClass(stat.change, stat.inverse)}">
                ${stat.change > 0 ? '↑' : stat.change < 0 ? '↓' : '→'} ${Math.abs(stat.change).toFixed(2)}%
            </span>
        </div>
    `).join('');
}

// Update charts
function updateCharts(quality, perf, gungraun) {
    // Quality chart
    if (quality && quality.history) {
        createChart('qualityChart', 'Quality Benchmarks', quality.history, [
            { label: 'Success Rate (%)', data: 'success_rate', color: '#667eea' },
            { label: 'Avg Score', data: 'avg_score', color: '#764ba2' }
        ]);
    }
    
    // Performance chart
    if (perf && perf.history) {
        createChart('performanceChart', 'Performance Benchmarks', perf.history, [
            { label: 'Latency (ms)', data: 'avg_latency_ms', color: '#f093fb' },
            { label: 'Throughput (req/s)', data: 'throughput', color: '#4facfe' }
        ]);
    }
    
    // Gungraun chart
    if (gungraun && gungraun.history) {
        createChart('gungraunChart', 'Memory & Instructions', gungraun.history, [
            { label: 'Peak Memory (MB)', data: 'peak_memory_mb', color: '#43e97b' },
            { label: 'Cache Misses', data: 'cache_misses', color: '#fa709a' }
        ]);
    }
}

// Create a chart
function createChart(canvasId, title, history, datasets) {
    const canvas = document.getElementById(canvasId);
    if (!canvas) return;
    
    const ctx = canvas.getContext('2d');
    
    // Destroy existing chart if it exists
    if (charts[canvasId]) {
        charts[canvasId].destroy();
    }
    
    const labels = history.map(h => new Date(h.timestamp).toLocaleDateString());
    
    charts[canvasId] = new Chart(ctx, {
        type: 'line',
        data: {
            labels: labels,
            datasets: datasets.map(ds => ({
                label: ds.label,
                data: history.map(h => h[ds.data]),
                borderColor: ds.color,
                backgroundColor: ds.color + '20',
                tension: 0.4,
                fill: true
            }))
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
                title: {
                    display: true,
                    text: title,
                    font: { size: 16 }
                },
                legend: {
                    display: true,
                    position: 'top'
                }
            },
            scales: {
                y: {
                    beginAtZero: true
                }
            }
        }
    });
}

// Update tables
function updateTables(quality, perf, gungraun) {
    if (quality && quality.metrics) {
        updateTable('qualityTableBody', [
            { metric: 'Success Rate', current: formatPercent(quality.metrics.success_rate || 0), previous: formatPercent(quality.metrics.prev_success_rate || 0) },
            { metric: 'Average Score', current: (quality.metrics.avg_score || 0).toFixed(2), previous: (quality.metrics.prev_avg_score || 0).toFixed(2) },
            { metric: 'Total Tests', current: quality.metrics.total_tests || 0, previous: quality.metrics.prev_total_tests || 0 }
        ]);
    }
    
    if (perf && perf.metrics) {
        updateTable('performanceTableBody', [
            { metric: 'Avg Latency', current: (perf.metrics.avg_latency_ms || 0).toFixed(2) + ' ms', previous: (perf.metrics.prev_avg_latency_ms || 0).toFixed(2) + ' ms' },
            { metric: 'P95 Latency', current: (perf.metrics.p95_latency_ms || 0).toFixed(2) + ' ms', previous: (perf.metrics.prev_p95_latency_ms || 0).toFixed(2) + ' ms' },
            { metric: 'Throughput', current: formatNumber(perf.metrics.throughput || 0) + ' req/s', previous: formatNumber(perf.metrics.prev_throughput || 0) + ' req/s' }
        ]);
    }
    
    if (gungraun && gungraun.metrics) {
        updateTable('gungraunTableBody', [
            { metric: 'Peak Memory', current: (gungraun.metrics.peak_memory_mb || 0).toFixed(2) + ' MB', previous: (gungraun.metrics.prev_peak_memory_mb || 0).toFixed(2) + ' MB' },
            { metric: 'Total Instructions', current: formatNumber(gungraun.metrics.total_instructions || 0), previous: formatNumber(gungraun.metrics.prev_total_instructions || 0) },
            { metric: 'Cache Misses', current: formatNumber(gungraun.metrics.cache_misses || 0), previous: formatNumber(gungraun.metrics.prev_cache_misses || 0) }
        ]);
    }
}

// Update a table
function updateTable(tableBodyId, rows) {
    const tbody = document.getElementById(tableBodyId);
    if (!tbody) return;
    
    tbody.innerHTML = rows.map(row => {
        const currentNum = parseFloat(row.current);
        const previousNum = parseFloat(row.previous);
        const change = calculateChange(currentNum, previousNum);
        
        return `
            <tr>
                <td>${row.metric}</td>
                <td>${row.current}</td>
                <td>${row.previous}</td>
                <td><span class="stat-change ${getChangeClass(change)}">${change > 0 ? '+' : ''}${change.toFixed(2)}%</span></td>
            </tr>
        `;
    }).join('');
}

// Update detailed results
function updateDetailedResults(quality, perf, gungraun) {
    const container = document.getElementById('detailedResults');
    if (!container) return;
    
    let html = '';
    
    if (quality) {
        html += `<h3>Quality Benchmarks</h3><pre>${JSON.stringify(quality, null, 2)}</pre>`;
    }
    if (perf) {
        html += `<h3>Performance Benchmarks</h3><pre>${JSON.stringify(perf, null, 2)}</pre>`;
    }
    if (gungraun) {
        html += `<h3>Gungraun Benchmarks</h3><pre>${JSON.stringify(gungraun, null, 2)}</pre>`;
    }
    
    if (!html) {
        html = '<p>No detailed results available.</p>';
    }
    
    container.innerHTML = html;
}

// Show error message
function showError(message) {
    const statsGrid = document.getElementById('statsGrid');
    statsGrid.innerHTML = `<div class="error">${message}</div>`;
}

// Initialize dashboard
document.addEventListener('DOMContentLoaded', () => {
    loadBenchmarkData();
    
    // Refresh every 5 minutes
    setInterval(loadBenchmarkData, 5 * 60 * 1000);
});
EOF

# Create sample data files
cat > "$OUTPUT_DIR/data/quality-latest.json" <<'EOF'
{
  "timestamp": "2025-01-08T22:00:00Z",
  "metrics": {
    "success_rate": 95.5,
    "prev_success_rate": 94.2,
    "success_rate_change": 1.38,
    "avg_score": 8.7,
    "prev_avg_score": 8.5,
    "change": 2.35,
    "total_tests": 150,
    "prev_total_tests": 145
  },
  "history": [
    {"timestamp": "2025-01-01", "success_rate": 93.0, "avg_score": 8.2},
    {"timestamp": "2025-01-03", "success_rate": 94.2, "avg_score": 8.5},
    {"timestamp": "2025-01-05", "success_rate": 94.8, "avg_score": 8.6},
    {"timestamp": "2025-01-08", "success_rate": 95.5, "avg_score": 8.7}
  ]
}
EOF

cat > "$OUTPUT_DIR/data/perf-latest.json" <<'EOF'
{
  "timestamp": "2025-01-08T22:00:00Z",
  "metrics": {
    "avg_latency_ms": 125.3,
    "prev_avg_latency_ms": 132.1,
    "latency_change": -5.15,
    "p95_latency_ms": 245.7,
    "prev_p95_latency_ms": 258.3,
    "throughput": 1250,
    "prev_throughput": 1180,
    "throughput_change": 5.93
  },
  "history": [
    {"timestamp": "2025-01-01", "avg_latency_ms": 135.2, "throughput": 1100},
    {"timestamp": "2025-01-03", "avg_latency_ms": 132.1, "throughput": 1180},
    {"timestamp": "2025-01-05", "avg_latency_ms": 128.5, "throughput": 1220},
    {"timestamp": "2025-01-08", "avg_latency_ms": 125.3, "throughput": 1250}
  ]
}
EOF

cat > "$OUTPUT_DIR/data/gungraun-latest.json" <<'EOF'
{
  "timestamp": "2025-01-08T22:00:00Z",
  "metrics": {
    "peak_memory_mb": 45.2,
    "prev_peak_memory_mb": 47.8,
    "memory_change": -5.44,
    "total_instructions": 1250000,
    "prev_total_instructions": 1280000,
    "instructions_change": -2.34,
    "cache_misses": 15200,
    "prev_cache_misses": 16100
  },
  "history": [
    {"timestamp": "2025-01-01", "peak_memory_mb": 48.5, "cache_misses": 16500},
    {"timestamp": "2025-01-03", "peak_memory_mb": 47.8, "cache_misses": 16100},
    {"timestamp": "2025-01-05", "peak_memory_mb": 46.3, "cache_misses": 15800},
    {"timestamp": "2025-01-08", "peak_memory_mb": 45.2, "cache_misses": 15200}
  ]
}
EOF

# Create README for gh-pages
cat > "$OUTPUT_DIR/README.md" <<'EOF'
# Merlin Benchmark Dashboard

This directory contains the web-based benchmark tracking dashboard for the Merlin project.

## Structure

- `index.html` - Main dashboard page
- `assets/dashboard.js` - Dashboard JavaScript
- `data/` - Benchmark data in JSON format
  - `quality-latest.json` - Latest quality benchmark results
  - `perf-latest.json` - Latest performance benchmark results
  - `gungraun-latest.json` - Latest gungraun benchmark results
- `quality-bench/` - Historical quality benchmark results
- `perf-bench/` - Historical performance benchmark results
- `gungraun-bench/` - Historical gungraun benchmark results

## Viewing the Dashboard

Visit: https://YOUR_USERNAME.github.io/agentic_optimizer/

## Updating Data

The dashboard automatically updates when new benchmark results are pushed to the gh-pages branch by CI workflows.

## Local Development

To test locally:
```bash
cd gh-pages
python3 -m http.server 8000
# Visit http://localhost:8000
```
EOF

echo "✅ Dashboard generated successfully!"
echo "📁 Output directory: $OUTPUT_DIR"
echo ""
echo "Next steps:"
echo "1. Review the generated files in $OUTPUT_DIR"
echo "2. Push to gh-pages branch: cd $OUTPUT_DIR && git add . && git commit -m 'Add benchmark dashboard' && git push"
echo "3. Enable GitHub Pages in repository settings"
echo "4. Visit https://YOUR_USERNAME.github.io/agentic_optimizer/"
