#!/usr/bin/env bash

set -euo pipefail

# Usage:
#   ./scripts/coverage.sh
#
# Environment variables:
#   MERLIN_CI   Set to skip clean step in CI environments
#
# This script runs coverage instrumentation and generates a coverage report.
# Run verify.sh first to ensure code quality (format, clippy, file sizes).

# Run workspace tests with coverage (excluding expensive crates)
echo "[coverage] Running tests with coverage instrumentation..."
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"

# Respect CARGO_TARGET_DIR if set externally, otherwise use default
TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT_DIR}/target}"
export MERLIN_FOLDER="${TARGET_DIR}/.merlin"
mkdir -p benchmarks/data/coverage

LLVM_COV_DIR="${TARGET_DIR}/llvm-cov-target"
# Clean any existing prof files for a clean build
rm -f "${LLVM_COV_DIR}/*.profdata" 2>/dev/null
rm -f "${LLVM_COV_DIR}/*.profraw" 2>/dev/null

# Run coverage on all workspace crates
# Excludes benchmark crates and test repositories from instrumentation
echo "[coverage] Running coverage on workspace crates..."

# Set cargo profile with default
CARGO_PROFILE="${CARGO_PROFILE:-dev}"

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

# Keep profraw files for grcov (will be cleaned up later)

# Delete the profraw list file that cargo llvm-cov creates
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

# Run llvm-cov export in parallel (one thread per binary)
echo "[coverage] Running llvm-cov in parallel (${#BINARIES[@]} threads)..."
PIDS=()
for i in "${!BINARIES[@]}"; do
  binary="${BINARIES[$i]}"
  output_file="${TEMP_COV_DIR}/cov_${i}.lcov"

  (
    llvm-cov export \
      -format=lcov \
      -instr-profile="${PROFDATA_FILE}" \
      -ignore-filename-regex="${IGNORE_PATTERN}" \
      "$binary" \
      > "$output_file" 2>/dev/null
  ) &
  PIDS+=($!)
done

# Wait for all parallel llvm-cov jobs
for pid in "${PIDS[@]}"; do
  wait "$pid"
done

# Merge all lcov files
if command -v lcov >/dev/null 2>&1; then
  LCOV_ARGS=()
  for lcovfile in "$TEMP_COV_DIR"/*.lcov; do
    if [ -f "$lcovfile" ] && [ -s "$lcovfile" ]; then
      LCOV_ARGS+=("-a" "$lcovfile")
    fi
  done
  lcov "${LCOV_ARGS[@]}" -o benchmarks/data/coverage/latest.info 2>/dev/null
else
  cat "$TEMP_COV_DIR"/*.lcov > benchmarks/data/coverage/latest.info 2>/dev/null
fi

rm -rf "$TEMP_COV_DIR"
git add benchmarks/data/coverage/latest.info

mkdir -p benchmarks/data/coverage
grcov benchmarks/data/coverage/latest.info \
  -s "${ROOT_DIR}" \
  -t html \
  -o benchmarks/data/coverage 2>/dev/null

REPORT_END=$(date +%s)
REPORT_TIME=$((REPORT_END - REPORT_START))
echo "[coverage] Coverage reports generated in ${REPORT_TIME}s"

# Clean up profraw files now that we're done
echo "[coverage] Cleaning up profraw files..."
find "${LLVM_COV_DIR}" -name "*.profraw" -delete 2>/dev/null
