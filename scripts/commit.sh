#!/usr/bin/env bash

set -euo pipefail

# Usage:
#   ./scripts/commit.sh [--no-cloud] [--ollama] [--html]
#
# Flags:
#   --no-cloud  Unset cloud provider API keys to prevent running GROQ/OpenRouter/Anthropic tests
#   --ollama    Additionally run tests filtered to "ollama" (requires a local Ollama server)
#   --html      Generate HTML coverage report in addition to lcov
#
# This script runs full verification with coverage instrumentation and generates a coverage report.
# For faster verification without coverage, use ./scripts/verify.sh instead.

NO_CLOUD=false
RUN_OLLAMA=false
GENERATE_HTML=false

for arg in "$@"; do
  case "$arg" in
    --no-cloud)
      NO_CLOUD=true
      shift
      ;;
    --ollama)
      RUN_OLLAMA=true
      shift
      ;;
    --html)
      GENERATE_HTML=true
      shift
      ;;
    *)
      # ignore unknown args (forward compatibility)
      ;;
  esac
done

if [ "$NO_CLOUD" = true ]; then
  # Explicitly unset cloud provider keys to ensure their tests are skipped
  unset GROQ_API_KEY || true
  unset OPENROUTER_API_KEY || true
  unset ANTHROPIC_API_KEY || true
  echo "[commit] Cloud provider tests disabled (--no-cloud)"
fi

# Format, lint, and test
cargo fmt --all -q
cargo clippy --no-deps --all-targets --all-features -- -D warnings

# Run workspace tests with coverage (excluding expensive crates)
echo "[commit] Running tests with coverage instrumentation..."
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
export MERLIN_FOLDER="${ROOT_DIR}/target/.merlin"
mkdir -p benchmarks/data/coverage

# Clean any existing profraw files for a clean build
cargo llvm-cov clean --workspace

# Run coverage on all workspace crates
# Excludes benchmark crates and test repositories from instrumentation
echo "[commit] Running coverage on workspace crates..."
LLVM_PROFILE_FILE="coverage/%p-%m.profraw" \
cargo llvm-cov \
  --no-report \
  --ignore-filename-regex "test_repositories|.cargo|.rustup" \
  --all-features \
  --workspace \
  --exclude merlin-benchmarks-criterion \
  --exclude merlin-benchmarks-gungraun \
  --exclude merlin-benchmarks-quality \
  --lib --tests \
  --no-fail-fast

# Check profraw files and disk usage
LLVM_COV_DIR="${ROOT_DIR}/target/llvm-cov-target"
PROFRAW_COUNT=$(find "${LLVM_COV_DIR}" -name "*.profraw" 2>/dev/null | wc -l)
PROFRAW_SIZE=$(find "${LLVM_COV_DIR}" -name "*.profraw" -exec du -ch {} + 2>/dev/null | tail -1 | awk '{print $1}')
LLVM_COV_SIZE=$(du -sh "${LLVM_COV_DIR}" 2>/dev/null | awk '{print $1}')
echo "[commit] Found ${PROFRAW_COUNT} profraw files (${PROFRAW_SIZE}), llvm-cov-target total: ${LLVM_COV_SIZE}"

# Merge profraw files into single profdata file
PROFDATA_FILE="${ROOT_DIR}/target/coverage.profdata"
echo "[commit] Merging profraw files into ${PROFDATA_FILE}..."
MERGE_START=$(date +%s)
llvm-profdata merge -sparse \
  -o "${PROFDATA_FILE}" \
  $(find "${LLVM_COV_DIR}" -name "*.profraw" 2>/dev/null)
MERGE_END=$(date +%s)
MERGE_TIME=$((MERGE_END - MERGE_START))
PROFDATA_SIZE=$(du -sh "${PROFDATA_FILE}" 2>/dev/null | awk '{print $1}')
echo "[commit] Profraw files merged in ${MERGE_TIME}s (profdata size: ${PROFDATA_SIZE})"

# Delete profraw files to save disk space
echo "[commit] Removing profraw files..."
find "${LLVM_COV_DIR}" -name "*.profraw" -delete 2>/dev/null

# Generate lcov report from merged profdata
echo "[commit] Generating lcov report from merged profdata..."
REPORT_START=$(date +%s)
cargo llvm-cov report \
  --ignore-filename-regex "test_repositories|.cargo|.rustup|tests/" \
  --instr-profile="${PROFDATA_FILE}" \
  --lcov \
  --output-path benchmarks/data/coverage/latest.info
REPORT_END=$(date +%s)
REPORT_TIME=$((REPORT_END - REPORT_START))
echo "[commit] Lcov report generated in ${REPORT_TIME}s"
git add benchmarks/data/coverage/latest.info

# Optionally generate HTML report from merged profdata
if [ "$GENERATE_HTML" = true ]; then
  echo "[commit] Generating HTML report from merged profdata..."
  HTML_START=$(date +%s)
  cargo llvm-cov report \
    --ignore-filename-regex "test_repositories|.cargo|.rustup|tests/" \
    --instr-profile="${PROFDATA_FILE}" \
    --html \
    --output-dir benchmarks/data/coverage/html
  HTML_END=$(date +%s)
  HTML_TIME=$((HTML_END - HTML_START))
  echo "[commit] HTML coverage report generated in ${HTML_TIME}s at benchmarks/data/coverage/html/index.html"
fi

# Delete profdata file after reports are generated
echo "[commit] Removing profdata file..."
rm -f "${PROFDATA_FILE}"
REMAINING_SIZE=$(du -sh "${LLVM_COV_DIR}" 2>/dev/null | awk '{print $1}')
echo "[commit] Coverage artifacts cleaned, instrumented builds remain: ${REMAINING_SIZE}"

echo "[commit] Cleaning old build artifacts..."
cargo sweep --time 1 -r

# Optionally run Ollama-specific tests by name filter
if [ "$RUN_OLLAMA" = true ]; then
  OLLAMA_HOST="${OLLAMA_HOST:-http://127.0.0.1:11434}"
  export OLLAMA_HOST
  echo "[commit] Running Ollama-filtered tests (OLLAMA_HOST=${OLLAMA_HOST})"
  # Run tests that contain 'ollama' in their name across the workspace.
  # If none exist, this will simply find zero tests and succeed.
  cargo test --workspace ollama -- --nocapture || true
fi
