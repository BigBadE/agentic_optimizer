#!/usr/bin/env bash

set -euo pipefail

# Usage:
#   ./scripts/verify.sh [--cov]
#
# Flags:
#   --cov       Run coverage testing (delegates to coverage.sh)
#
# Environment variables:
#   MERLIN_CI   Set to skip clean step in CI environments

check_file_sizes() {
  local max_lines=500
  local violations=()

  # Find all Rust source files
  while IFS= read -r file; do
    local line_count=$(wc -l < "$file")
    if [ "$line_count" -gt "$max_lines" ]; then
      violations+=("$file: $line_count lines")
    fi
  done < <(find crates -name "*.rs" -type f)

  if [ "${#violations[@]}" -gt 0 ]; then
    echo "ERROR: The following files exceed $max_lines lines:"
    printf '%s\n' "${violations[@]}"
    return 1
  fi

  return 0
}

RUN_COVERAGE=false

for arg in "$@"; do
  case "$arg" in
    --cov)
      RUN_COVERAGE=true
      shift
      ;;
    *)
      # ignore unknown args (forward compatibility)
      ;;
  esac
done

# Check file sizes
echo "[verify] Checking file sizes..."
check_file_sizes

# Format, lint, and test
cargo fmt -q
# Check libs and tests only (not bins) to avoid false-positive dead code warnings
# for feature-gated test utilities that are only used by integration tests
cargo clippy --no-deps --lib --tests --all-features -- -D warnings

# Delegate to coverage.sh if --cov is passed (skip normal tests)
if [ "$RUN_COVERAGE" = true ]; then
  exec "$(dirname "${BASH_SOURCE[0]}")/coverage.sh"
fi

# Run full workspace tests
echo "[verify] Running tests..."
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
export MERLIN_FOLDER="${ROOT_DIR}/target/.merlin"

# Run main tests
cargo nextest run --run-ignored all

if [ -z "${MERLIN_CI:-}" ]; then
  echo "[coverage] Cleaning old build artifacts..."
  cargo sweep --time 1 -r
else
  echo "[coverage] Skipping clean step (MERLIN_CI set)"
fi