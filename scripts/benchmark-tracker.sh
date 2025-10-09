#!/usr/bin/env bash
# Benchmark Tracker - Aggregates and visualizes benchmark results from CI
# Tracks quality-bench, performance benchmarks, and gungraun-bench over time

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BENCH_DATA_DIR="$PROJECT_ROOT/.benchmark-data"
REPORT_FILE="$PROJECT_ROOT/benchmark-report.md"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create benchmark data directory if it doesn't exist
mkdir -p "$BENCH_DATA_DIR"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to fetch latest benchmark results from gh-pages
fetch_benchmark_data() {
    local bench_type=$1
    local output_dir="$BENCH_DATA_DIR/$bench_type"
    
    print_status "Fetching $bench_type benchmark data..."
    
    mkdir -p "$output_dir"
    
    # Try to fetch from gh-pages branch
    if git show-ref --verify --quiet refs/heads/gh-pages; then
        git show gh-pages:${bench_type}/latest.md > "$output_dir/latest.md" 2>/dev/null || {
            print_warning "No latest.md found for $bench_type"
            return 1
        }
        
        # Get historical data (last 10 runs)
        git log --all --pretty=format:"%H %ai" -- "gh-pages/${bench_type}/*.md" | head -10 > "$output_dir/history.txt" || true
        
        print_success "Fetched $bench_type data"
        return 0
    else
        print_warning "gh-pages branch not found. Run benchmarks first."
        return 1
    fi
}

# Function to parse benchmark results and extract metrics
parse_benchmark_metrics() {
    local bench_file=$1
    local bench_type=$2
    local timestamp=${3:-$(date +%s)}
    
    if [ ! -f "$bench_file" ]; then
        print_warning "Benchmark file not found: $bench_file"
        return 1
    fi
    
    local metrics_file="$BENCH_DATA_DIR/${bench_type}/metrics.json"
    
    print_status "Parsing metrics from $bench_file..."
    
    # Extract metrics based on benchmark type
    case "$bench_type" in
        "quality-bench")
            # Parse quality metrics (success rate, avg score, etc.)
            python3 - <<EOF
import json
import re
import sys

metrics = {
    "timestamp": $timestamp,
    "type": "$bench_type",
    "metrics": {}
}

try:
    with open("$bench_file", "r") as f:
        content = f.read()
        
    # Extract success rate
    success_match = re.search(r'Success Rate:\s*(\d+\.?\d*)%', content)
    if success_match:
        metrics["metrics"]["success_rate"] = float(success_match.group(1))
    
    # Extract average score
    score_match = re.search(r'Average Score:\s*(\d+\.?\d*)', content)
    if score_match:
        metrics["metrics"]["avg_score"] = float(score_match.group(1))
    
    # Extract total tests
    tests_match = re.search(r'Total Tests:\s*(\d+)', content)
    if tests_match:
        metrics["metrics"]["total_tests"] = int(tests_match.group(1))
    
    print(json.dumps(metrics, indent=2))
except Exception as e:
    print(f"Error parsing quality benchmarks: {e}", file=sys.stderr)
    sys.exit(1)
EOF
            ;;
            
        "perf-bench")
            # Parse performance metrics (latency, throughput, etc.)
            python3 - <<EOF
import json
import re
import sys

metrics = {
    "timestamp": $timestamp,
    "type": "$bench_type",
    "metrics": {}
}

try:
    with open("$bench_file", "r") as f:
        content = f.read()
    
    # Extract average latency
    latency_match = re.search(r'Average Latency:\s*(\d+\.?\d*)\s*ms', content)
    if latency_match:
        metrics["metrics"]["avg_latency_ms"] = float(latency_match.group(1))
    
    # Extract throughput
    throughput_match = re.search(r'Throughput:\s*(\d+\.?\d*)\s*req/s', content)
    if throughput_match:
        metrics["metrics"]["throughput"] = float(throughput_match.group(1))
    
    # Extract p95 latency
    p95_match = re.search(r'P95 Latency:\s*(\d+\.?\d*)\s*ms', content)
    if p95_match:
        metrics["metrics"]["p95_latency_ms"] = float(p95_match.group(1))
    
    print(json.dumps(metrics, indent=2))
except Exception as e:
    print(f"Error parsing performance benchmarks: {e}", file=sys.stderr)
    sys.exit(1)
EOF
            ;;
            
        "gungraun-bench")
            # Parse gungraun metrics (memory, instructions, etc.)
            python3 - <<EOF
import json
import re
import sys

metrics = {
    "timestamp": $timestamp,
    "type": "$bench_type",
    "metrics": {}
}

try:
    with open("$bench_file", "r") as f:
        content = f.read()
    
    # Extract total instructions
    instr_match = re.search(r'Total Instructions:\s*([\d,]+)', content)
    if instr_match:
        metrics["metrics"]["total_instructions"] = int(instr_match.group(1).replace(',', ''))
    
    # Extract peak memory
    mem_match = re.search(r'Peak Memory:\s*(\d+\.?\d*)\s*MB', content)
    if mem_match:
        metrics["metrics"]["peak_memory_mb"] = float(mem_match.group(1))
    
    # Extract cache misses
    cache_match = re.search(r'Cache Misses:\s*([\d,]+)', content)
    if cache_match:
        metrics["metrics"]["cache_misses"] = int(cache_match.group(1).replace(',', ''))
    
    print(json.dumps(metrics, indent=2))
except Exception as e:
    print(f"Error parsing gungraun benchmarks: {e}", file=sys.stderr)
    sys.exit(1)
EOF
            ;;
    esac
}

# Function to generate comparison report
generate_comparison_report() {
    print_status "Generating benchmark comparison report..."
    
    cat > "$REPORT_FILE" <<'HEADER'
# Benchmark Tracking Report

**Generated:** $(date '+%Y-%m-%d %H:%M:%S')

This report tracks benchmark results across all three benchmark suites:
- **Quality Benchmarks**: Task success rate and quality scores
- **Performance Benchmarks**: Latency and throughput metrics
- **Gungraun Benchmarks**: Memory usage and instruction counts

---

HEADER

    # Add Quality Benchmarks section
    if [ -f "$BENCH_DATA_DIR/quality-bench/latest.md" ]; then
        cat >> "$REPORT_FILE" <<EOF
## Quality Benchmarks

### Latest Results
\`\`\`
$(head -30 "$BENCH_DATA_DIR/quality-bench/latest.md")
\`\`\`

EOF
    fi
    
    # Add Performance Benchmarks section
    if [ -f "$BENCH_DATA_DIR/perf-bench/latest.md" ]; then
        cat >> "$REPORT_FILE" <<EOF
## Performance Benchmarks

### Latest Results
\`\`\`
$(head -30 "$BENCH_DATA_DIR/perf-bench/latest.md")
\`\`\`

EOF
    fi
    
    # Add Gungraun Benchmarks section
    if [ -f "$BENCH_DATA_DIR/gungraun-bench/latest.md" ]; then
        cat >> "$REPORT_FILE" <<EOF
## Gungraun Benchmarks

### Latest Results
\`\`\`
$(head -30 "$BENCH_DATA_DIR/gungraun-bench/latest.md")
\`\`\`

EOF
    fi
    
    # Add trend analysis section
    cat >> "$REPORT_FILE" <<EOF
## Trend Analysis

### Historical Data
- Quality benchmarks: $(find "$BENCH_DATA_DIR/quality-bench" -name "*.md" 2>/dev/null | wc -l) runs tracked
- Performance benchmarks: $(find "$BENCH_DATA_DIR/perf-bench" -name "*.md" 2>/dev/null | wc -l) runs tracked
- Gungraun benchmarks: $(find "$BENCH_DATA_DIR/gungraun-bench" -name "*.md" 2>/dev/null | wc -l) runs tracked

### Commands
- View this report: \`cat benchmark-report.md\`
- Fetch latest data: \`./scripts/benchmark-tracker.sh fetch\`
- Generate new report: \`./scripts/benchmark-tracker.sh report\`

---

*Generated by benchmark-tracker.sh*
EOF

    print_success "Report generated: $REPORT_FILE"
}

# Function to show benchmark trends
show_trends() {
    print_status "Analyzing benchmark trends..."
    
    echo ""
    echo "=== Benchmark Trends ==="
    echo ""
    
    for bench_type in "quality-bench" "perf-bench" "gungraun-bench"; do
        if [ -d "$BENCH_DATA_DIR/$bench_type" ]; then
            echo "📊 $bench_type:"
            local count=$(find "$BENCH_DATA_DIR/$bench_type" -name "*.md" 2>/dev/null | wc -l)
            echo "   Total runs: $count"
            
            if [ -f "$BENCH_DATA_DIR/$bench_type/latest.md" ]; then
                echo "   Latest: $(stat -c %y "$BENCH_DATA_DIR/$bench_type/latest.md" 2>/dev/null | cut -d' ' -f1 || echo 'unknown')"
            fi
            echo ""
        fi
    done
}

# Main command dispatcher
case "${1:-help}" in
    fetch)
        print_status "Fetching all benchmark data..."
        fetch_benchmark_data "quality-bench" || true
        fetch_benchmark_data "perf-bench" || true
        fetch_benchmark_data "gungraun-bench" || true
        print_success "Fetch complete"
        ;;
        
    report)
        generate_comparison_report
        ;;
        
    trends)
        show_trends
        ;;
        
    all)
        fetch_benchmark_data "quality-bench" || true
        fetch_benchmark_data "perf-bench" || true
        fetch_benchmark_data "gungraun-bench" || true
        generate_comparison_report
        show_trends
        ;;
        
    help|*)
        cat <<EOF
Benchmark Tracker - Track and visualize benchmark results

Usage: $0 <command>

Commands:
    fetch       Fetch latest benchmark data from gh-pages
    report      Generate comparison report
    trends      Show benchmark trends
    all         Run all commands (fetch + report + trends)
    help        Show this help message

Examples:
    $0 fetch    # Fetch latest data
    $0 report   # Generate report
    $0 all      # Do everything

Output:
    - Benchmark data: .benchmark-data/
    - Report: benchmark-report.md
EOF
        ;;
esac
