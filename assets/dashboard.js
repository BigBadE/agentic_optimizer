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
