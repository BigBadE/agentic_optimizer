// Benchmark Dashboard JavaScript

let charts = {};
let currentData = { quality: null, perf: null, gungraun: null };
let selectedBenchmark = { quality: null, perf: null, gungraun: null };

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
            fetch('data/quality/latest.json').then(r => r.ok ? r.json() : null).catch(() => null),
            fetch('data/criterion/latest.json').then(r => r.ok ? r.json() : null).catch(() => null),
            fetch('data/gungraun/latest.json').then(r => r.ok ? r.json() : null).catch(() => null)
        ]);

        // Store data globally for filtering
        currentData.quality = qualityData;
        currentData.perf = perfData;
        currentData.gungraun = gungraunData;

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

    // Quality metrics - use actual field names from parse script
    if (quality && quality.metrics) {
        stats.push({
            label: 'Precision@3',
            value: formatPercent(quality.metrics.precision_at_3 || 0),
            change: quality.metrics.precision_at_3_change || 0,
            inverse: false
        });
        stats.push({
            label: 'Test Cases',
            value: quality.metrics.test_cases || 0,
            change: quality.metrics.test_cases_change || 0,
            inverse: false
        });
    }

    // Performance metrics - use actual field names
    if (perf && perf.metrics) {
        stats.push({
            label: 'Avg Time',
            value: (perf.metrics.avg_time_ms || 0).toFixed(3) + ' ms',
            change: perf.metrics.avg_time_ms_change || 0,
            inverse: true
        });
        stats.push({
            label: 'Total Benchmarks',
            value: perf.metrics.total_benchmarks || 0,
            change: perf.metrics.total_benchmarks_change || 0,
            inverse: false
        });
    }

    // Gungraun metrics - use actual field names
    if (gungraun && gungraun.metrics) {
        stats.push({
            label: 'Avg Instructions',
            value: formatNumber(gungraun.metrics.avg_instructions || 0),
            change: gungraun.metrics.avg_instructions_change || 0,
            inverse: true
        });
        stats.push({
            label: 'Avg Cycles',
            value: formatNumber(gungraun.metrics.avg_cycles || 0),
            change: gungraun.metrics.avg_cycles_change || 0,
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
    // Quality chart - use actual field names
    if (quality && quality.history && quality.history.length > 0) {
        createChart('qualityChart', 'Quality Benchmarks', quality.history, [
            { label: 'Precision@3 (%)', data: 'precision_at_3', color: '#667eea' },
            { label: 'Recall@10 (%)', data: 'recall_at_10', color: '#764ba2' },
            { label: 'MRR', data: 'mrr', color: '#f093fb', yAxisID: 'y1' }
        ]);
    }

    // Performance chart - use actual field names
    if (perf && perf.history && perf.history.length > 0) {
        createChart('performanceChart', 'Performance Benchmarks', perf.history, [
            { label: 'Avg Time (ms)', data: 'avg_time_ms', color: '#f093fb' },
            { label: 'Total Time (ms)', data: 'total_time_ms', color: '#4facfe', yAxisID: 'y1' }
        ]);
    }

    // Gungraun chart - use actual field names
    if (gungraun && gungraun.history && gungraun.history.length > 0) {
        createChart('gungraunChart', 'Memory & Instructions', gungraun.history, [
            { label: 'Avg Instructions', data: 'avg_instructions', color: '#43e97b' },
            { label: 'Avg Cycles', data: 'avg_cycles', color: '#fa709a' }
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
            { metric: 'Test Cases', current: quality.metrics.test_cases || 0, previous: quality.metrics.prev_test_cases || 0 },
            { metric: 'Precision@3', current: formatPercent(quality.metrics.precision_at_3 || 0), previous: formatPercent(quality.metrics.prev_precision_at_3 || 0) },
            { metric: 'Precision@10', current: formatPercent(quality.metrics.precision_at_10 || 0), previous: formatPercent(quality.metrics.prev_precision_at_10 || 0) },
            { metric: 'Recall@10', current: formatPercent(quality.metrics.recall_at_10 || 0), previous: formatPercent(quality.metrics.prev_recall_at_10 || 0) },
            { metric: 'MRR', current: (quality.metrics.mrr || 0).toFixed(4), previous: (quality.metrics.prev_mrr || 0).toFixed(4) },
            { metric: 'NDCG@10', current: (quality.metrics.ndcg_at_10 || 0).toFixed(4), previous: (quality.metrics.prev_ndcg_at_10 || 0).toFixed(4) },
            { metric: 'Critical in Top-3', current: formatPercent(quality.metrics.critical_in_top_3 || 0), previous: formatPercent(quality.metrics.prev_critical_in_top_3 || 0) }
        ]);
    }

    if (perf && perf.metrics) {
        updateTable('performanceTableBody', [
            { metric: 'Total Benchmarks', current: perf.metrics.total_benchmarks || 0, previous: perf.metrics.prev_total_benchmarks || 0 },
            { metric: 'Avg Time', current: (perf.metrics.avg_time_ms || 0).toFixed(3) + ' ms', previous: (perf.metrics.prev_avg_time_ms || 0).toFixed(3) + ' ms' },
            { metric: 'Total Time', current: (perf.metrics.total_time_ms || 0).toFixed(3) + ' ms', previous: (perf.metrics.prev_total_time_ms || 0).toFixed(3) + ' ms' }
        ]);
    }

    if (gungraun && gungraun.metrics) {
        updateTable('gungraunTableBody', [
            { metric: 'Total Benchmarks', current: gungraun.metrics.total_benchmarks || 0, previous: gungraun.metrics.prev_total_benchmarks || 0 },
            { metric: 'Avg Instructions', current: formatNumber(gungraun.metrics.avg_instructions || 0), previous: formatNumber(gungraun.metrics.prev_avg_instructions || 0) },
            { metric: 'Total Instructions', current: formatNumber(gungraun.metrics.total_instructions || 0), previous: formatNumber(gungraun.metrics.prev_total_instructions || 0) },
            { metric: 'Avg Cycles', current: formatNumber(gungraun.metrics.avg_cycles || 0), previous: formatNumber(gungraun.metrics.prev_avg_cycles || 0) },
            { metric: 'Total Cycles', current: formatNumber(gungraun.metrics.total_cycles || 0), previous: formatNumber(gungraun.metrics.prev_total_cycles || 0) }
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

    // Performance benchmarks - show ALL with search/filter
    if (perf && perf.benchmarks && perf.benchmarks.length > 0) {
        html += '<div class="benchmark-table-section">';
        html += '<h3>Performance Benchmarks <span class="badge">' + perf.benchmarks.length + ' tests</span></h3>';
        html += '<input type="text" id="perfFilter" class="filter-input" placeholder="Search benchmarks..." onkeyup="filterBenchmarks(\'perf\')">';
        html += '<table class="metric-table clickable-table" id="perfBenchTable"><thead><tr>';
        html += '<th onclick="sortTable(\'perf\', 0)">Benchmark ▼</th>';
        html += '<th onclick="sortTable(\'perf\', 1)">Mean Time ▼</th>';
        html += '<th onclick="sortTable(\'perf\', 2)">Median Time ▼</th>';
        html += '<th onclick="sortTable(\'perf\', 3)">Std Dev ▼</th>';
        html += '</tr></thead><tbody id="perfBenchBody">';

        perf.benchmarks.forEach((bench, idx) => {
            html += `<tr class="clickable-row" onclick="selectBenchmark('perf', ${idx})" data-name="${bench.name.toLowerCase()}">`;
            html += `<td><strong>${bench.name}</strong></td>`;
            html += `<td>${bench.mean_ms.toFixed(3)} ms</td>`;
            html += `<td>${bench.median_ms.toFixed(3)} ms</td>`;
            html += `<td>±${bench.std_dev_ms.toFixed(3)} ms</td>`;
            html += '</tr>';
        });

        html += '</tbody></table></div>';
    } else if (perf) {
        html += '<div class="benchmark-table-section">';
        html += '<h3>Performance Benchmarks</h3>';
        html += '<p class="no-data">No performance benchmark data available. Total benchmarks: ' + (perf.metrics.total_benchmarks || 0) + '</p>';
        html += '<p class="info">Run <code>cargo bench --workspace</code> to generate performance benchmark data.</p>';
        html += '</div>';
    }

    // Gungraun benchmarks - show ALL with search/filter
    if (gungraun && gungraun.benchmarks && gungraun.benchmarks.length > 0) {
        html += '<div class="benchmark-table-section">';
        html += '<h3>Memory & Instructions (Gungraun) <span class="badge">' + gungraun.benchmarks.length + ' tests</span></h3>';
        html += '<input type="text" id="gungraunFilter" class="filter-input" placeholder="Search benchmarks..." onkeyup="filterBenchmarks(\'gungraun\')">';
        html += '<table class="metric-table clickable-table" id="gungraunBenchTable"><thead><tr>';
        html += '<th onclick="sortTable(\'gungraun\', 0)">Benchmark ▼</th>';
        html += '<th onclick="sortTable(\'gungraun\', 1)">Instructions ▼</th>';
        html += '<th onclick="sortTable(\'gungraun\', 2)">Cycles ▼</th>';
        html += '<th onclick="sortTable(\'gungraun\', 3)">L1 Accesses ▼</th>';
        html += '<th onclick="sortTable(\'gungraun\', 4)">L2 Accesses ▼</th>';
        html += '<th onclick="sortTable(\'gungraun\', 5)">RAM Accesses ▼</th>';
        html += '</tr></thead><tbody id="gungraunBenchBody">';

        gungraun.benchmarks.forEach((bench, idx) => {
            html += `<tr class="clickable-row" onclick="selectBenchmark('gungraun', ${idx})" data-name="${bench.name.toLowerCase()}">`;
            html += `<td><strong>${bench.name}</strong></td>`;
            html += `<td>${formatNumber(bench.instructions)}</td>`;
            html += `<td>${formatNumber(bench.estimated_cycles)}</td>`;
            html += `<td>${formatNumber(bench.l1_accesses)}</td>`;
            html += `<td>${formatNumber(bench.l2_accesses)}</td>`;
            html += `<td>${formatNumber(bench.ram_accesses)}</td>`;
            html += '</tr>';
        });

        html += '</tbody></table></div>';
    } else if (gungraun) {
        html += '<div class="benchmark-table-section">';
        html += '<h3>Memory & Instructions (Gungraun)</h3>';
        html += '<p class="no-data">No gungraun benchmark data available. Total benchmarks: ' + (gungraun.metrics.total_benchmarks || 0) + '</p>';
        html += '<p class="info">Gungraun benchmarks require specific setup. Check project documentation.</p>';
        html += '</div>';
    }

    // Quality metrics summary
    if (quality && quality.metrics) {
        html += '<div class="benchmark-table-section">';
        html += '<h3>Quality Metrics Summary <span class="badge">' + (quality.metrics.test_cases || 0) + ' tests</span></h3>';
        html += '<table class="metric-table"><thead><tr>';
        html += '<th>Metric</th><th>Value</th><th>Description</th>';
        html += '</tr></thead><tbody>';
        html += `<tr><td><strong>Test Cases</strong></td><td>${quality.metrics.test_cases || 0}</td><td>Total number of quality tests</td></tr>`;
        html += `<tr><td><strong>Precision@3</strong></td><td>${formatPercent(quality.metrics.precision_at_3 || 0)}</td><td>Accuracy of top 3 results</td></tr>`;
        html += `<tr><td><strong>Precision@10</strong></td><td>${formatPercent(quality.metrics.precision_at_10 || 0)}</td><td>Accuracy of top 10 results</td></tr>`;
        html += `<tr><td><strong>Recall@10</strong></td><td>${formatPercent(quality.metrics.recall_at_10 || 0)}</td><td>Coverage in top 10 results</td></tr>`;
        html += `<tr><td><strong>MRR</strong></td><td>${(quality.metrics.mrr || 0).toFixed(4)}</td><td>Mean Reciprocal Rank</td></tr>`;
        html += `<tr><td><strong>NDCG@10</strong></td><td>${(quality.metrics.ndcg_at_10 || 0).toFixed(4)}</td><td>Normalized Discounted Cumulative Gain</td></tr>`;
        html += `<tr><td><strong>Critical in Top-3</strong></td><td>${formatPercent(quality.metrics.critical_in_top_3 || 0)}</td><td>Critical issues found in top 3</td></tr>`;
        html += '</tbody></table></div>';
    }

    if (!html) {
        html = '<p class="no-data">No benchmark data available. Run benchmarks to populate this dashboard.</p>';
    }

    container.innerHTML = html;
}

// Filter benchmarks by search text
function filterBenchmarks(type) {
    const input = document.getElementById(type + 'Filter');
    const filter = input.value.toLowerCase();
    const table = document.getElementById(type + 'BenchTable');
    const rows = table.getElementsByTagName('tr');

    for (let i = 1; i < rows.length; i++) {
        const row = rows[i];
        const name = row.getAttribute('data-name');
        if (name && name.includes(filter)) {
            row.style.display = '';
        } else {
            row.style.display = 'none';
        }
    }
}

// Select a benchmark to highlight and show details
function selectBenchmark(type, index) {
    const tbody = document.getElementById(type + 'BenchBody');
    const rows = tbody.getElementsByTagName('tr');

    // Remove previous selection
    for (let row of rows) {
        row.classList.remove('selected');
    }

    // Add selection to clicked row
    rows[index].classList.add('selected');

    // Store selection
    selectedBenchmark[type] = index;

    // Could add modal or side panel with more details here
}

// Sort table by column
function sortTable(type, columnIndex) {
    const table = document.getElementById(type + 'BenchTable');
    const tbody = document.getElementById(type + 'BenchBody');
    const rows = Array.from(tbody.getElementsByTagName('tr'));

    rows.sort((a, b) => {
        const aText = a.getElementsByTagName('td')[columnIndex].textContent;
        const bText = b.getElementsByTagName('td')[columnIndex].textContent;

        // Try numeric sort
        const aNum = parseFloat(aText.replace(/[^0-9.-]/g, ''));
        const bNum = parseFloat(bText.replace(/[^0-9.-]/g, ''));

        if (!isNaN(aNum) && !isNaN(bNum)) {
            return bNum - aNum; // Descending
        }

        // Text sort
        return aText.localeCompare(bText);
    });

    // Re-append rows in sorted order
    rows.forEach(row => tbody.appendChild(row));
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
