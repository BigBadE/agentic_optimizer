#!/usr/bin/env bash
# Test runner with enhanced output formatting and package-level reporting

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
TEST_FLAGS="--all-features --no-fail-fast --lib --bins --tests"
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMP_DIR="${WORKSPACE_ROOT}/target/test-output"
START_TIME=$(date +%s)

# Parse arguments
SPECIFIC_PACKAGE=""
NO_CLOUD=false

print_usage() {
    echo "Usage: $0 [OPTIONS] [PACKAGE_NAME]"
    echo ""
    echo "OPTIONS:"
    echo "  --no-cloud    Skip cloud provider tests"
    echo "  -h, --help    Show this help message"
    echo ""
    echo "PACKAGE_NAME:"
    echo "  Specific package to run tests for with --nocapture (e.g., 'merlin-core')"
    echo ""
    echo "Examples:"
    echo "  $0                          # Run all tests"
    echo "  $0 --no-cloud               # Run all tests except cloud"
    echo "  $0 merlin-core              # Run tests for merlin-core with output"
    echo "  $0 --no-cloud merlin-core   # Run merlin-core tests, skip cloud"
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --no-cloud)
            NO_CLOUD=true
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        -*)
            echo "Unknown option: $1"
            print_usage
            exit 1
            ;;
        *)
            SPECIFIC_PACKAGE="$1"
            shift
            ;;
    esac
done

# Setup
mkdir -p "$TEMP_DIR"
cd "$WORKSPACE_ROOT"

# Handle --no-cloud flag
if [[ "$NO_CLOUD" == true ]]; then
    unset GROQ_API_KEY || true
    unset OPENROUTER_API_KEY || true
    unset ANTHROPIC_API_KEY || true
    echo -e "${YELLOW}Cloud provider tests disabled (--no-cloud)${NC}\n"
fi

# Live build progress indicator
show_build_progress() {
    local build_log="$1"
    local spinner=('⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧' '⠇' '⠏')
    local spin_idx=0
    local iterations=0
    local max_iterations=600  # 2 minutes max (600 * 0.2s)

    # Trap to clean up on kill
    trap 'printf "\r\033[K"; exit 0' SIGTERM SIGINT

    # Wait for file to exist
    while [[ ! -f "$build_log" ]] && [[ $iterations -lt $max_iterations ]]; do
        sleep 0.1
        ((iterations++))
    done

    # Monitor build progress
    while [[ $iterations -lt $max_iterations ]]; do
        if [[ -f "$build_log" ]] && [[ -s "$build_log" ]]; then
            # Check if build finished or tests started (check early)
            if grep -q "Finished" "$build_log" 2>/dev/null; then
                # Count compiled crates for final message
                local count=$(grep -c "^[[:space:]]*Compiling" "$build_log" 2>/dev/null)
                if [[ -z "$count" ]] || [[ ! "$count" =~ ^[0-9]+$ ]]; then
                    count=0
                fi
                # Always clear the line and move to next line
                printf "\r%b" $'\033[K'
                if [[ $count -gt 0 ]]; then
                    printf "  ${GREEN}✓${NC} Built ${count} crates\n"
                else
                    printf "\n"  # Just move to next line if nothing was built
                fi
                break
            fi

            # Count compiled crates
            local count=$(grep -c "^[[:space:]]*Compiling" "$build_log" 2>/dev/null)
            if [[ -z "$count" ]] || [[ ! "$count" =~ ^[0-9]+$ ]]; then
                count=0
            fi

            # Get latest crate name
            local latest=""
            if [[ $count -gt 0 ]]; then
                latest=$(grep "^[[:space:]]*Compiling" "$build_log" 2>/dev/null | tail -1 | awk '{print $2}')
            fi

            # Update display with proper clearing
            if [[ -n "$latest" ]]; then
                printf "\r%b  ${CYAN}${spinner[$spin_idx]}${NC} Building ${latest} (${count} crates)..." $'\033[K'
            else
                printf "\r%b  ${CYAN}${spinner[$spin_idx]}${NC} Building..." $'\033[K'
            fi

            spin_idx=$(( (spin_idx + 1) % ${#spinner[@]} ))
        fi
        sleep 0.2
        ((iterations++))
    done
}

# Get list of workspace packages
get_test_packages() {
    # Use cargo tree to list workspace packages
    cargo tree --workspace --depth 0 2>/dev/null | \
        awk '{print $1}' | \
        grep -v '^\[' | \
        sort -u
}

# Run specific package with --nocapture
run_specific_package() {
    local pkg="$1"
    echo -e "${BOLD}${CYAN}=== Merlin Test Runner ===${NC}"
    echo -e "${BOLD}Testing package: ${YELLOW}$pkg${NC}"

    PKG_START=$(date +%s)
    OUTPUT_FILE="$TEMP_DIR/${pkg}.log"

    # Build test command with --nocapture
    TEST_CMD="cargo test -p $pkg $TEST_FLAGS -- --nocapture"

    echo ""

    # Start build progress indicator in background
    show_build_progress "$OUTPUT_FILE" &
    progress_pid=$!

    # Run tests and capture output
    if $TEST_CMD > "$OUTPUT_FILE" 2>&1; then
        RESULT="passed"
    else
        RESULT="failed"
    fi

    # Stop progress indicator and ensure it's finished
    kill $progress_pid 2>/dev/null || true
    wait $progress_pid 2>/dev/null || true
    sleep 0.05

    echo -e "  ${CYAN}Running tests...${NC}"
    echo ""

    # Parse results per test file
    declare -A FILE_RESULTS
    TOTAL_PASSED=0
    TOTAL_FAILED=0
    FAILED_TESTS=()

    # Extract per-file results
    current_file=""
    while IFS= read -r line; do
        # Match "Running unittests src\lib.rs" or "Running tests\foo.rs" (with optional leading whitespace)
        if [[ $line =~ Running[[:space:]]+(unittests|tests)[[:space:]\\\\]+([^[:space:]]+) ]]; then
            raw_file="${BASH_REMATCH[2]}"
            # Clean up the file path - remove .rs extension and path prefixes
            current_file="${raw_file%.rs}"
            # Remove src\ and tests\ prefixes (with backslashes on Windows)
            current_file="${current_file#src\\\\}"
            current_file="${current_file#tests\\\\}"
            # Also handle forward slashes for cross-platform compatibility
            current_file="${current_file#src/}"
            current_file="${current_file#tests/}"
        # Match test result line
        elif [[ $line =~ test[[:space:]]+result:[[:space:]]+ok\.[[:space:]]+([0-9]+)[[:space:]]+passed.*finished[[:space:]]+in[[:space:]]+([0-9.]+)s ]]; then
            passed_count="${BASH_REMATCH[1]}"
            test_duration="${BASH_REMATCH[2]}"
            TOTAL_PASSED=$((TOTAL_PASSED + passed_count))
            if [[ -n "$current_file" && $passed_count -gt 0 ]]; then
                echo -e "  ${GREEN}✓${NC} ${current_file} - ${passed_count} passed (${test_duration}s)"
                FILE_RESULTS[$current_file]="passed"
            fi
        elif [[ $line =~ test[[:space:]]+result:[[:space:]]+FAILED\.[[:space:]]+([0-9]+)[[:space:]]+passed\;[[:space:]]+([0-9]+)[[:space:]]+failed.*finished[[:space:]]+in[[:space:]]+([0-9.]+)s ]]; then
            passed_count="${BASH_REMATCH[1]}"
            failed_count="${BASH_REMATCH[2]}"
            test_duration="${BASH_REMATCH[3]}"
            TOTAL_PASSED=$((TOTAL_PASSED + passed_count))
            TOTAL_FAILED=$((TOTAL_FAILED + failed_count))
            if [[ -n "$current_file" ]]; then
                echo -e "  ${RED}✗${NC} ${current_file} - ${passed_count} passed, ${failed_count} failed (${test_duration}s)"
                FILE_RESULTS[$current_file]="failed"
            fi
        fi
    done < "$OUTPUT_FILE"

    # Extract failed test names
    while IFS= read -r line; do
        if [[ $line =~ test[[:space:]]([^[:space:]]+)[[:space:]]\.\.\.\ FAILED ]]; then
            FAILED_TESTS+=("${BASH_REMATCH[1]}")
        fi
    done < "$OUTPUT_FILE"

    PKG_END=$(date +%s)
    DURATION=$((PKG_END - PKG_START))

    # Summary - compact format
    echo ""
    echo -e "${BOLD}${CYAN}=== Test Summary ===${NC}"
    echo -e "${BOLD}Package:${NC} ${YELLOW}$pkg${NC} | ${BOLD}Total:${NC} $((TOTAL_PASSED + TOTAL_FAILED)) | ${GREEN}Passed:${NC} $TOTAL_PASSED | ${RED}Failed:${NC} $TOTAL_FAILED | ${BOLD}Duration:${NC} ${DURATION}s"

    # Failed tests detail
    if [[ ${#FAILED_TESTS[@]} -gt 0 ]]; then
        echo ""
        echo -e "${BOLD}${RED}Failed Tests:${NC}"
        for test in "${FAILED_TESTS[@]}"; do
            echo -e "  ${RED}✗${NC} $test"
        done

        # Show failure details
        echo ""
        echo -e "${BOLD}Failure Details:${NC}"
        sed -n '/^failures:$/,/^test result: FAILED/p' "$OUTPUT_FILE"

        exit 1
    else
        echo -e "${GREEN}${BOLD}All tests passed!${NC}"
        exit 0
    fi
}

# If specific package requested, run it and exit
if [[ -n "$SPECIFIC_PACKAGE" ]]; then
    run_specific_package "$SPECIFIC_PACKAGE"
fi

# Run all tests
echo -e "${BOLD}Discovering test packages...${NC}"
PACKAGES=($(get_test_packages))
TOTAL_PACKAGES=${#PACKAGES[@]}

if [[ $TOTAL_PACKAGES -eq 0 ]]; then
    echo -e "${YELLOW}No test packages found${NC}"
    exit 0
fi

echo -e "Found ${BOLD}$TOTAL_PACKAGES${NC} packages with tests\n"

# Results tracking
declare -A PACKAGE_RESULTS
declare -A PACKAGE_DURATIONS
declare -A PACKAGE_PASSED
declare -A PACKAGE_FAILED
FAILED_TESTS=()
TOTAL_PASSED=0
TOTAL_FAILED=0

# Run tests for each package
for pkg in "${PACKAGES[@]}"; do
    echo -e "${BOLD}${BLUE}Testing $pkg...${NC}"

    PKG_START=$(date +%s)
    OUTPUT_FILE="$TEMP_DIR/${pkg}.log"

    # Build test command
    TEST_CMD="cargo test -p $pkg $TEST_FLAGS --"

    # Start build progress indicator in background
    show_build_progress "$OUTPUT_FILE" &
    progress_pid=$!

    # Run tests and capture output
    if $TEST_CMD > "$OUTPUT_FILE" 2>&1; then
        # Stop progress indicator and ensure it's finished
        kill $progress_pid 2>/dev/null || true
        wait $progress_pid 2>/dev/null || true
        sleep 0.05

        echo -e "  ${CYAN}Running tests...${NC}"
        echo ""

        PACKAGE_RESULTS[$pkg]="passed"

        # Parse per-file results
        PASSED=0
        FAILED=0
        current_file=""
        declare -A file_results

        while IFS= read -r line; do
            # Match "Running unittests src\lib.rs" or "Running tests\foo.rs" (with optional leading whitespace)
            if [[ $line =~ Running[[:space:]]+(unittests|tests)[[:space:]\\\\]+([^[:space:]]+) ]]; then
                raw_file="${BASH_REMATCH[2]}"
                # Clean up the file path - remove .rs extension and path prefixes
                current_file="${raw_file%.rs}"
                # Remove src\ and tests\ prefixes (with backslashes on Windows)
                current_file="${current_file#src\\\\}"
                current_file="${current_file#tests\\\\}"
                # Also handle forward slashes for cross-platform compatibility
                current_file="${current_file#src/}"
                current_file="${current_file#tests/}"
            # Match test result line
            elif [[ $line =~ test[[:space:]]+result:[[:space:]]+ok\.[[:space:]]+([0-9]+)[[:space:]]+passed.*finished[[:space:]]+in[[:space:]]+([0-9.]+)s ]]; then
                passed_count="${BASH_REMATCH[1]}"
                test_duration="${BASH_REMATCH[2]}"
                PASSED=$((PASSED + passed_count))
                if [[ -n "$current_file" && $passed_count -gt 0 ]]; then
                    file_results[$current_file]="${GREEN}✓${NC} ${passed_count} passed (${test_duration}s)"
                fi
            fi
        done < "$OUTPUT_FILE"

        # Display per-file results
        for file in $(echo "${!file_results[@]}" | tr ' ' '\n' | sort); do
            echo -e "  ${file}: ${file_results[$file]}"
        done

        PACKAGE_PASSED[$pkg]=$PASSED
        PACKAGE_FAILED[$pkg]=$FAILED
        TOTAL_PASSED=$((TOTAL_PASSED + PASSED))

        PKG_END=$(date +%s)
        DURATION=$((PKG_END - PKG_START))
        PACKAGE_DURATIONS[$pkg]=$DURATION

        echo -e "  ${GREEN}✓${NC} ${BOLD}Package total:${NC} $PASSED tests (${DURATION}s)"
    else
        # Stop progress indicator and ensure it's finished
        kill $progress_pid 2>/dev/null || true
        wait $progress_pid 2>/dev/null || true
        sleep 0.05

        echo -e "  ${CYAN}Running tests...${NC}"
        echo ""

        PACKAGE_RESULTS[$pkg]="failed"

        # Parse per-file results
        PASSED=0
        FAILED=0
        current_file=""
        declare -A file_results

        while IFS= read -r line; do
            # Match "Running unittests src\lib.rs" or "Running tests\foo.rs" (with optional leading whitespace)
            if [[ $line =~ Running[[:space:]]+(unittests|tests)[[:space:]\\\\]+([^[:space:]]+) ]]; then
                raw_file="${BASH_REMATCH[2]}"
                # Clean up the file path - remove .rs extension and path prefixes
                current_file="${raw_file%.rs}"
                # Remove src\ and tests\ prefixes (with backslashes on Windows)
                current_file="${current_file#src\\\\}"
                current_file="${current_file#tests\\\\}"
                # Also handle forward slashes for cross-platform compatibility
                current_file="${current_file#src/}"
                current_file="${current_file#tests/}"
            # Match test result line
            elif [[ $line =~ test[[:space:]]+result:[[:space:]]+ok\.[[:space:]]+([0-9]+)[[:space:]]+passed.*finished[[:space:]]+in[[:space:]]+([0-9.]+)s ]]; then
                passed_count="${BASH_REMATCH[1]}"
                test_duration="${BASH_REMATCH[2]}"
                PASSED=$((PASSED + passed_count))
                if [[ -n "$current_file" && $passed_count -gt 0 ]]; then
                    file_results[$current_file]="${GREEN}✓${NC} ${passed_count} passed (${test_duration}s)"
                fi
            elif [[ $line =~ test[[:space:]]+result:[[:space:]]+FAILED\.[[:space:]]+([0-9]+)[[:space:]]+passed\;[[:space:]]+([0-9]+)[[:space:]]+failed.*finished[[:space:]]+in[[:space:]]+([0-9.]+)s ]]; then
                passed_count="${BASH_REMATCH[1]}"
                failed_count="${BASH_REMATCH[2]}"
                test_duration="${BASH_REMATCH[3]}"
                PASSED=$((PASSED + passed_count))
                FAILED=$((FAILED + failed_count))
                if [[ -n "$current_file" ]]; then
                    file_results[$current_file]="${RED}✗${NC} ${passed_count} passed, ${failed_count} failed (${test_duration}s)"
                fi
            fi
        done < "$OUTPUT_FILE"

        # Display per-file results
        for file in $(echo "${!file_results[@]}" | tr ' ' '\n' | sort); do
            echo -e "  ${file}: ${file_results[$file]}"
        done

        PACKAGE_PASSED[$pkg]=$PASSED
        PACKAGE_FAILED[$pkg]=$FAILED
        TOTAL_PASSED=$((TOTAL_PASSED + PASSED))
        TOTAL_FAILED=$((TOTAL_FAILED + FAILED))

        # Extract failed test names
        while IFS= read -r line; do
            if [[ $line =~ test[[:space:]]([^[:space:]]+)[[:space:]]\.\.\.\ FAILED ]]; then
                FAILED_TESTS+=("$pkg::${BASH_REMATCH[1]}")
            fi
        done < "$OUTPUT_FILE"

        PKG_END=$(date +%s)
        DURATION=$((PKG_END - PKG_START))
        PACKAGE_DURATIONS[$pkg]=$DURATION

        echo -e "  ${RED}✗${NC} ${BOLD}Package total:${NC} $PASSED passed, $FAILED failed (${DURATION}s)"
    fi

    echo ""
done

# Summary
END_TIME=$(date +%s)
TOTAL_DURATION=$((END_TIME - START_TIME))

echo -e "${BOLD}${CYAN}=== Test Summary ===${NC}\n"

# Package breakdown
echo -e "${BOLD}Package Results:${NC}"
for pkg in "${PACKAGES[@]}"; do
    result="${PACKAGE_RESULTS[$pkg]}"
    passed="${PACKAGE_PASSED[$pkg]}"
    failed="${PACKAGE_FAILED[$pkg]}"
    duration="${PACKAGE_DURATIONS[$pkg]}"

    if [[ "$result" == "passed" ]]; then
        echo -e "  ${GREEN}✓${NC} ${BOLD}$pkg${NC}: $passed passed (${duration}s)"
    else
        echo -e "  ${RED}✗${NC} ${BOLD}$pkg${NC}: $passed passed, $failed failed (${duration}s)"
    fi
done

echo ""
echo -e "${BOLD}Overall:${NC}"
echo -e "  Total tests: $((TOTAL_PASSED + TOTAL_FAILED))"
echo -e "  ${GREEN}Passed: $TOTAL_PASSED${NC}"
echo -e "  ${RED}Failed: $TOTAL_FAILED${NC}"
echo -e "  Duration: ${TOTAL_DURATION}s"

# Failed tests detail
if [[ ${#FAILED_TESTS[@]} -gt 0 ]]; then
    echo -e "\n${BOLD}${RED}Failed Tests:${NC}"
    for test in "${FAILED_TESTS[@]}"; do
        echo -e "  ${RED}✗${NC} $test"
    done
    echo ""
    echo -e "${YELLOW}To run a specific failed test with output:${NC}"
    echo -e "  $0 ${FAILED_TESTS[0]##*::}"
    echo ""

    # Show failure details
    echo -e "${BOLD}Failure Details:${NC}\n"
    for pkg in "${PACKAGES[@]}"; do
        if [[ "${PACKAGE_RESULTS[$pkg]}" == "failed" ]]; then
            echo -e "${BOLD}${RED}--- $pkg ---${NC}"
            # Extract and show failure section from log
            sed -n '/^failures:$/,/^test result: FAILED/p' "$TEMP_DIR/${pkg}.log"
            echo ""
        fi
    done

    exit 1
else
    echo -e "\n${GREEN}${BOLD}All tests passed!${NC}"
    exit 0
fi
