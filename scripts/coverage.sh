#!/usr/bin/env bash

set -euo pipefail

# Usage:
#   ./scripts/coverage.sh [--fixture]
#
# Flags:
#   --fixture   Run only fixture tests with coverage, other tests normally
#
# Environment variables:
#   MERLIN_CI   Set to skip clean step in CI environments
#
# This script runs coverage instrumentation and generates a coverage report.
# Run verify.sh first to ensure code quality (format, clippy, file sizes).

# Parse arguments
FIXTURE_ONLY=false
for arg in "$@"; do
  case "$arg" in
    --fixture)
      FIXTURE_ONLY=true
      shift
      ;;
    *)
      # ignore unknown args (forward compatibility)
      ;;
  esac
done

# Run workspace tests with coverage (excluding expensive crates)
echo "[coverage] Running tests with coverage instrumentation..."
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"

# Respect CARGO_TARGET_DIR if set externally, otherwise use default
TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT_DIR}/target}"
export MERLIN_FOLDER="${TARGET_DIR}/.merlin"
mkdir -p benchmarks/data/coverage

LLVM_COV_DIR="${TARGET_DIR}/llvm-cov-target"
# Clean any existing prof files and temp directories for a clean build
rm -f "${LLVM_COV_DIR}"/*.profdata 2>/dev/null
rm -f "${LLVM_COV_DIR}"/*.profraw 2>/dev/null
rm -rf "${LLVM_COV_DIR}"/temp_lcov_* 2>/dev/null

# Run coverage on all workspace crates
# Excludes benchmark crates and test repositories from instrumentation
# Set cargo profile with default
CARGO_PROFILE="${CARGO_PROFILE:-dev}"

if [ "$FIXTURE_ONLY" = true ]; then
  echo "[coverage] Running fixture tests with coverage, other tests normally..."

  # First run fixture tests with coverage
  echo "[coverage] Running fixture tests with coverage instrumentation..."
  LLVM_PROFILE_FILE_NAME="merlin-%m.profraw" \
  cargo llvm-cov \
    --no-report \
    --ignore-filename-regex "test_repositories|.cargo|.rustup" \
    --all-features \
    --package integration-tests \
    --lib --tests \
    --no-fail-fast \
    --cargo-profile "${CARGO_PROFILE}" \
    nextest

  COV_EXIT=$?
  echo "[coverage] Fixture tests completed with exit code: $COV_EXIT"

  # Then run all other tests normally (without coverage)
  echo "[coverage] Running non-fixture tests without coverage..."
  cargo nextest run \
    --workspace \
    --exclude integration-tests \
    --run-ignored all

  NORMAL_EXIT=$?
  echo "[coverage] Non-fixture tests completed with exit code: $NORMAL_EXIT"

  # Exit with failure if either failed
  if [ $COV_EXIT -ne 0 ] || [ $NORMAL_EXIT -ne 0 ]; then
    echo "[coverage] Tests failed (cov: $COV_EXIT, normal: $NORMAL_EXIT)"
    exit 1
  fi
else
  echo "[coverage] Running all tests with coverage instrumentation..."
  LLVM_PROFILE_FILE_NAME="merlin-%m.profraw" \
  cargo llvm-cov \
    --no-report \
    --ignore-filename-regex "test_repositories|.cargo|.rustup" \
    --all-features \
    --workspace \
    --exclude merlin-benchmarks-criterion \
    --exclude merlin-benchmarks-gungraun \
    --exclude merlin-benchmarks-quality \
    --lib --tests \
    --no-fail-fast \
    --cargo-profile "${CARGO_PROFILE}" \
    nextest

  echo "[coverage] Nextest completed with exit code: $?"
fi

# Check profraw files and disk usage
PROFRAW_COUNT=$(find "${LLVM_COV_DIR}" -name "*.profraw" 2>/dev/null | wc -l || echo "0")
PROFRAW_SIZE=$(find "${LLVM_COV_DIR}" -name "*.profraw" -exec du -ch {} + 2>/dev/null | tail -1 | awk '{print $1}' || echo "unknown")
LLVM_COV_SIZE=$(du -sh "${LLVM_COV_DIR}" 2>/dev/null | awk '{print $1}' || echo "unknown")
echo "[coverage] Found ${PROFRAW_COUNT} profraw files (${PROFRAW_SIZE}), llvm-cov-target total: ${LLVM_COV_SIZE}"

# Merge profraw files into the expected profdata location
# cargo llvm-cov expects: ${LLVM_COV_DIR}/${project_name}.profdata
PROJECT_NAME="agentic_optimizer"
PROFDATA_FILE="${LLVM_COV_DIR}/${PROJECT_NAME}.profdata"
echo "[coverage] Merging profraw files into ${PROFDATA_FILE}..."
MERGE_START=$(date +%s)
llvm-profdata merge -sparse \
  -o "${PROFDATA_FILE}" \
  $(find "${LLVM_COV_DIR}" -name "*.profraw" 2>/dev/null)
MERGE_END=$(date +%s)
MERGE_TIME=$((MERGE_END - MERGE_START))
PROFDATA_SIZE=$(du -sh "${PROFDATA_FILE}" 2>/dev/null | awk '{print $1}' || echo "unknown")
echo "[coverage] Profraw files merged in ${MERGE_TIME}s (profdata size: ${PROFDATA_SIZE})"

# Delete the prof files that cargo llvm-cov creates
rm -f "${LLVM_COV_DIR}/*.profraw" 2>/dev/null
rm -f "${LLVM_COV_DIR}/${PROJECT_NAME}-profraw-list" 2>/dev/null

# Generate coverage reports using llvm-cov in parallel
echo "[coverage] Generating coverage reports..."
REPORT_START=$(date +%s)

# Find all instrumented test binaries
BINARIES=()
while IFS= read -r binary; do
  BINARIES+=("$binary")
done < <(find "${LLVM_COV_DIR}/debug/deps" -type f -name "*.exe" 2>/dev/null | sort)

echo "[coverage] Found ${#BINARIES[@]} instrumented binaries"

if [ "${#BINARIES[@]}" -eq 0 ]; then
  echo "[coverage] ERROR: No instrumented binaries found"
  exit 1
fi

# Ignore patterns matching cargo llvm-cov behavior
IGNORE_PATTERN="test_repositories|\.cargo|\.rustup|/tests/|\\\\rustc\\\\|\\\\target\\\\llvm-cov-target|\\\\cargo\\\\(registry|git)|\\\\rustup\\\\toolchains"

# Create temporary directory for parallel lcov generation
TEMP_COV_DIR="${LLVM_COV_DIR}/temp_lcov_$$"
mkdir -p "$TEMP_COV_DIR"

# Convert paths to Windows format if on MSYS/Windows (llvm-cov is a native Windows tool)
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
  PROFDATA_FILE_NATIVE="$(cygpath -w "${PROFDATA_FILE}")"
else
  PROFDATA_FILE_NATIVE="${PROFDATA_FILE}"
fi

# Run llvm-cov export in parallel (one thread per binary)
echo "[coverage] Running llvm-cov in parallel (${#BINARIES[@]} threads)..."
PIDS=()
for i in "${!BINARIES[@]}"; do
  binary="${BINARIES[$i]}"
  # Convert binary path to Windows format if needed
  if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    binary_native="$(cygpath -w "${binary}")"
  else
    binary_native="${binary}"
  fi
  output_file="${TEMP_COV_DIR}/cov_${i}.lcov"

  (
    error_file="${TEMP_COV_DIR}/error_${i}.txt"
    if llvm-cov export \
      -format=lcov \
      -instr-profile="${PROFDATA_FILE_NATIVE}" \
      -ignore-filename-regex="${IGNORE_PATTERN}" \
      "$binary_native" \
      > "$output_file" 2>"$error_file"; then
      # Success
      exit 0
    fi

    # Failed - check if it's due to missing coverage data (benign)
    if grep -q "no coverage data found" "$error_file" 2>/dev/null; then
      # Create empty file to avoid merge issues
      echo -n "" > "$output_file"
      exit 0
    fi

    # Real error
    echo "[coverage] ERROR: llvm-cov export failed for binary $i ($(basename "$binary")):" >&2
    cat "$error_file" >&2
    exit 1
  ) &
  PIDS+=($!)
done

# Wait for all parallel llvm-cov jobs and collect failures
FAILED=0
for pid in "${PIDS[@]}"; do
  if ! wait "$pid"; then
    FAILED=$((FAILED + 1))
  fi
done

if [ $FAILED -gt 0 ]; then
  echo "[coverage] ERROR: $FAILED llvm-cov export jobs failed" >&2
  exit 1
fi

echo "[coverage] Merging reports..."
# Merge all lcov files using simple concatenation
# This is valid for lcov format and avoids lcov tool dependencies
cat "$TEMP_COV_DIR"/*.lcov > benchmarks/data/coverage/latest.info 2>/dev/null

rm -rf "$TEMP_COV_DIR"
git add benchmarks/data/coverage/latest.info

mkdir -p benchmarks/data/coverage
if ! grcov benchmarks/data/coverage/latest.info \
  -s "${ROOT_DIR}" \
  -t html \
  -o benchmarks/data/coverage 2>&1; then
  echo "[coverage] ERROR: grcov failed to generate HTML report" >&2
  exit 1
fi

REPORT_END=$(date +%s)
REPORT_TIME=$((REPORT_END - REPORT_START))
echo "[coverage] Coverage reports generated in ${REPORT_TIME}s"
