#!/usr/bin/env bash

set -euo pipefail

# Usage:
#   ./scripts/verify.sh [--cov] [--fixture]
#
# Flags:
#   --cov       Run coverage testing (delegates to coverage.sh)
#   --fixture   Run only fixture tests with coverage, other tests normally
#
# Environment variables:
#   MERLIN_CI   Set to skip clean step in CI environments

check_file_sizes() {
  local max_lines=500
  local violations=$(
          find crates -type f -name '*.rs' -print0 |
          xargs -0 awk -v max="$max_lines" '
            { count[FILENAME]++ }
            ENDFILE { if (count[FILENAME] > max) print FILENAME ": " count[FILENAME] " lines" }
          '
        )

  if [ -n "$violations" ]; then
    echo "ERROR: The following files exceed $max_lines lines:"
    echo "$violations"
    return 1
  fi

  return 0
}

check_allows() {
  # Find all #[allow] and #![allow] annotations except the specific test pattern
  # The allowed patterns are:
  #   #![cfg_attr(
  #       test,
  #       allow(
  #           clippy::missing_panics_doc,
  #           clippy::missing_errors_doc,
  #           reason = "Allow for tests"
  #       )
  #   )]
  #   #[allow(unsafe_code)] - Required for FFI with JavaScript runtime

  # Use ripgrep to find all allow annotations in a single pass
  local all_allows
  all_allows=$(rg -n '#!?\[.*allow\(' crates --type rust 2>/dev/null || true)

  if [ -z "$all_allows" ]; then
    return 0
  fi

  # Filter out the allowed patterns
  local violations
  violations=$(echo "$all_allows" | grep -v 'cfg_attr(\s*test,\s*allow(\s*clippy::missing_panics_doc' | grep -v 'allow(unsafe_code)' || true)

  if [ -n "$violations" ]; then
    echo "ERROR: Found #[allow] or #![allow] annotations in the following locations:"
    echo "$violations"
    echo ""
    echo "Allows are not allowed in this project, period. All issues should be fixed."
    return 1
  fi

  return 0
}

RUN_COVERAGE=false
FIXTURE_ONLY=false

for arg in "$@"; do
  case "$arg" in
    --cov)
      RUN_COVERAGE=true
      shift
      ;;
    --fixture)
      FIXTURE_ONLY=true
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

# Check for disallowed allow annotations
echo "[verify] Checking for disallowed #[allow] annotations..."
check_allows

# Format, lint, and test
cargo fmt -q
cargo clippy --no-deps --bins --lib --tests --all-features -- -D warnings

# Delegate to coverage.sh if --cov is passed (skip normal tests)
if [ "$RUN_COVERAGE" = true ]; then
  if [ "$FIXTURE_ONLY" = true ]; then
    # Run all tests first to ensure everything passes
    echo "[verify] Running all tests..."
    cargo nextest run --run-ignored all

    # Then generate coverage for fixtures only
    exec "$(dirname "${BASH_SOURCE[0]}")/coverage.sh" --fixture
  else
    exec "$(dirname "${BASH_SOURCE[0]}")/coverage.sh"
  fi
fi

# Run full workspace tests
echo "[verify] Running tests..."
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"

# Run main tests
cargo nextest run --run-ignored all

if [ -z "${MERLIN_CI:-}" ]; then
  echo "[coverage] Cleaning old build artifacts..."
  cargo sweep --time 1 -r
else
  echo "[coverage] Skipping clean step (MERLIN_CI set)"
fi