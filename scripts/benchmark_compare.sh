#!/bin/bash
# Benchmark comparison and regression detection script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CRITERION_DIR="$PROJECT_ROOT/target/criterion"
RESULTS_DIR="$PROJECT_ROOT/benchmark_results"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Regression threshold (percentage)
REGRESSION_THRESHOLD=15

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -b, --baseline NAME    Save current results as baseline NAME"
    echo "  -c, --compare NAME     Compare current run against baseline NAME"
    echo "  -l, --list             List available baselines"
    echo "  -r, --run              Run benchmarks before comparison"
    echo "  -h, --help             Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --run --baseline main           # Run benchmarks and save as 'main'"
    echo "  $0 --run --compare main            # Run and compare against 'main'"
    echo "  $0 --compare feature-x             # Compare existing results against feature-x"
}

run_benchmarks() {
    echo -e "${BLUE}Running benchmarks...${NC}"
    cargo bench --workspace --message-format=json > "$RESULTS_DIR/latest.json" 2>&1
    echo -e "${GREEN}✓ Benchmarks complete${NC}"
}

save_baseline() {
    local name=$1
    mkdir -p "$RESULTS_DIR/baselines"

    if [ -d "$CRITERION_DIR" ]; then
        echo -e "${BLUE}Saving baseline: $name${NC}"
        cp -r "$CRITERION_DIR" "$RESULTS_DIR/baselines/$name"
        echo -e "${GREEN}✓ Baseline '$name' saved${NC}"
    else
        echo -e "${RED}Error: No benchmark results found. Run benchmarks first.${NC}"
        exit 1
    fi
}

list_baselines() {
    echo -e "${BLUE}Available baselines:${NC}"
    if [ -d "$RESULTS_DIR/baselines" ]; then
        ls -1 "$RESULTS_DIR/baselines" | while read baseline; do
            date=$(stat -c %y "$RESULTS_DIR/baselines/$baseline" 2>/dev/null || stat -f %Sm "$RESULTS_DIR/baselines/$baseline" 2>/dev/null || echo "Unknown")
            echo "  - $baseline (saved: ${date%% *})"
        done
    else
        echo "  No baselines found."
    fi
}

compare_results() {
    local baseline=$1
    local baseline_dir="$RESULTS_DIR/baselines/$baseline"

    if [ ! -d "$baseline_dir" ]; then
        echo -e "${RED}Error: Baseline '$baseline' not found${NC}"
        list_baselines
        exit 1
    fi

    if [ ! -d "$CRITERION_DIR" ]; then
        echo -e "${RED}Error: No current results found. Run benchmarks first.${NC}"
        exit 1
    fi

    echo -e "${BLUE}Comparing against baseline: $baseline${NC}"
    echo ""

    local regressions=0
    local improvements=0
    local unchanged=0

    # Find all benchmark directories
    find "$CRITERION_DIR" -name "base" -type d | while read current_base; do
        local bench_name=$(dirname "$current_base" | sed "s|$CRITERION_DIR/||")
        local baseline_estimates="$baseline_dir/$bench_name/base/estimates.json"
        local current_estimates="$current_base/estimates.json"

        if [ -f "$baseline_estimates" ] && [ -f "$current_estimates" ]; then
            # Extract mean times (in nanoseconds)
            local baseline_mean=$(jq -r '.mean.point_estimate' "$baseline_estimates")
            local current_mean=$(jq -r '.mean.point_estimate' "$current_estimates")

            # Calculate percentage change
            local change=$(echo "scale=2; (($current_mean - $baseline_mean) / $baseline_mean) * 100" | bc)
            local abs_change=$(echo $change | tr -d '-')

            # Categorize change
            if (( $(echo "$abs_change < 5" | bc -l) )); then
                status="${NC}➡️  No change"
                unchanged=$((unchanged + 1))
            elif (( $(echo "$change < 0" | bc -l) )); then
                status="${GREEN}✓ Improved"
                improvements=$((improvements + 1))
            elif (( $(echo "$abs_change >= $REGRESSION_THRESHOLD" | bc -l) )); then
                status="${RED}⚠️  REGRESSION"
                regressions=$((regressions + 1))
            else
                status="${YELLOW}⚠  Slower"
                regressions=$((regressions + 1))
            fi

            # Format times for display
            local baseline_ms=$(echo "scale=2; $baseline_mean / 1000000" | bc)
            local current_ms=$(echo "scale=2; $current_mean / 1000000" | bc)

            printf "  %-50s %s (%.2f ms → %.2f ms, %+.1f%%)${NC}\n" \
                "$bench_name" "$status" "$baseline_ms" "$current_ms" "$change"
        fi
    done

    echo ""
    echo -e "${BLUE}Summary:${NC}"
    echo -e "  ${GREEN}Improvements: $improvements${NC}"
    echo -e "  ${YELLOW}Regressions: $regressions${NC}"
    echo -e "  ${NC}Unchanged: $unchanged${NC}"

    if [ $regressions -gt 0 ]; then
        echo ""
        echo -e "${RED}⚠️  Performance regressions detected!${NC}"
        echo -e "Review the benchmarks above and investigate changes."
        return 1
    else
        echo ""
        echo -e "${GREEN}✓ No significant regressions detected${NC}"
        return 0
    fi
}

generate_report() {
    local baseline=$1
    local output="$RESULTS_DIR/comparison_report_$(date +%Y%m%d_%H%M%S).md"

    echo "# Benchmark Comparison Report" > "$output"
    echo "" >> "$output"
    echo "**Date**: $(date)" >> "$output"
    echo "**Baseline**: $baseline" >> "$output"
    echo "" >> "$output"
    echo "## Results" >> "$output"
    echo "" >> "$output"
    echo "| Benchmark | Baseline | Current | Change | Status |" >> "$output"
    echo "|-----------|----------|---------|--------|--------|" >> "$output"

    find "$CRITERION_DIR" -name "base" -type d | while read current_base; do
        local bench_name=$(dirname "$current_base" | sed "s|$CRITERION_DIR/||")
        local baseline_estimates="$RESULTS_DIR/baselines/$baseline/$bench_name/base/estimates.json"
        local current_estimates="$current_base/estimates.json"

        if [ -f "$baseline_estimates" ] && [ -f "$current_estimates" ]; then
            local baseline_mean=$(jq -r '.mean.point_estimate' "$baseline_estimates")
            local current_mean=$(jq -r '.mean.point_estimate' "$current_estimates")
            local change=$(echo "scale=2; (($current_mean - $baseline_mean) / $baseline_mean) * 100" | bc)

            local baseline_ms=$(echo "scale=2; $baseline_mean / 1000000" | bc)
            local current_ms=$(echo "scale=2; $current_mean / 1000000" | bc)

            local status
            if (( $(echo "$change < -5" | bc -l) )); then
                status="✅ Improved"
            elif (( $(echo "$change > $REGRESSION_THRESHOLD" | bc -l) )); then
                status="❌ Regression"
            elif (( $(echo "$change > 5" | bc -l) )); then
                status="⚠️ Slower"
            else
                status="➡️ Unchanged"
            fi

            echo "| $bench_name | ${baseline_ms}ms | ${current_ms}ms | ${change}% | $status |" >> "$output"
        fi
    done

    echo "" >> "$output"
    echo "---" >> "$output"
    echo "Generated by benchmark_compare.sh" >> "$output"

    echo -e "${GREEN}✓ Report generated: $output${NC}"
}

# Parse arguments
RUN_BENCH=false
BASELINE=""
COMPARE=""
LIST=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -b|--baseline)
            BASELINE="$2"
            shift 2
            ;;
        -c|--compare)
            COMPARE="$2"
            shift 2
            ;;
        -l|--list)
            LIST=true
            shift
            ;;
        -r|--run)
            RUN_BENCH=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Create results directory
mkdir -p "$RESULTS_DIR"

# Execute actions
if [ "$LIST" = true ]; then
    list_baselines
    exit 0
fi

if [ "$RUN_BENCH" = true ]; then
    run_benchmarks
fi

if [ -n "$BASELINE" ]; then
    save_baseline "$BASELINE"
fi

if [ -n "$COMPARE" ]; then
    compare_results "$COMPARE"
    generate_report "$COMPARE"
fi

if [ "$RUN_BENCH" = false ] && [ -z "$BASELINE" ] && [ -z "$COMPARE" ]; then
    usage
    exit 1
fi
