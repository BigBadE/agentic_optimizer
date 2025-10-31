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
  # Only two exact allow patterns are permitted:
  #
  # 1. #![cfg_attr(
  #        test,
  #        allow(
  #            clippy::tests_outside_test_module,
  #            reason = "Allow for integration tests"
  #        )
  #    )]
  #
  # 2. #[allow(
  #        unsafe_code,
  #        reason = "Arc<dyn Tool> is not Trace, but safe to use as documented above"
  #    )]

  local violations_found=false

  # Find all Rust files with allow annotations
  # Use find + grep since rg may not be in PATH in all environments
  while IFS= read -r file; do
    # Check each file individually for proper validation
    if ! validate_allows_in_file "$file"; then
      violations_found=true
    fi
  done < <(find crates -name '*.rs' -type f -exec grep -l '#\[allow(' {} \; 2>/dev/null)

  if [ "$violations_found" = true ]; then
    return 1
  fi

  return 0
}

validate_allows_in_file() {
  local file="$1"
  local violations=""

  # Read the entire file and check each allow annotation
  local in_allow=false
  local allow_start_line=0
  local allow_content=""
  local line_num=0

  while IFS= read -r line; do
    ((line_num++))

    # Detect start of allow annotation
    if echo "$line" | grep -q '#!\?\[.*allow('; then
      in_allow=true
      allow_start_line=$line_num
      allow_content="$line"

      # Check if it's a single-line allow (contains closing )])
      if echo "$line" | grep -q ')]'; then
        # Process immediately as complete allow
        local normalized
        normalized=$(echo "$allow_content" | tr -d '\n\r' | sed 's/[[:space:]]\+/ /g' | sed 's/^ //; s/ $//')

        # Check against the two exact allowed patterns
        local is_valid=false

        # Pattern 1: cfg_attr test allow (normalized)
        local pattern1='#![cfg_attr( test, allow( clippy::tests_outside_test_module, reason = "Allow for integration tests" ) )]'
        if [ "$normalized" = "$pattern1" ]; then
          is_valid=true
        fi

        # Pattern 2: unsafe_code allow (normalized)
        local pattern2='#[allow( unsafe_code, reason = "Arc<dyn Tool> is not Trace, but safe to use as documented above" )]'
        if [ "$normalized" = "$pattern2" ]; then
          is_valid=true
        fi

        if [ "$is_valid" = false ]; then
          violations="${violations}${file}:${allow_start_line}:${normalized}\n"
        fi

        # Reset for next annotation
        in_allow=false
        allow_content=""
      fi
      continue
    fi

    # Continue collecting allow content (multi-line case)
    if [ "$in_allow" = true ]; then
      allow_content="${allow_content}"$'\n'"${line}"

      # Check if we've reached the end of the allow block
      if echo "$line" | grep -q '^[[:space:]]*)][[:space:]]*$'; then
        # Normalize whitespace for comparison
        local normalized
        normalized=$(echo "$allow_content" | tr -d '\n\r' | sed 's/[[:space:]]\+/ /g' | sed 's/^ //; s/ $//')

        # Check against the two exact allowed patterns
        local is_valid=false

        # Pattern 1: cfg_attr test allow (normalized)
        local pattern1='#![cfg_attr( test, allow( clippy::tests_outside_test_module, reason = "Allow for integration tests" ) )]'
        if [ "$normalized" = "$pattern1" ]; then
          is_valid=true
        fi

        # Pattern 2: unsafe_code allow (normalized)
        local pattern2='#[allow( unsafe_code, reason = "Arc<dyn Tool> is not Trace, but safe to use as documented above" )]'
        if [ "$normalized" = "$pattern2" ]; then
          is_valid=true
        fi

        if [ "$is_valid" = false ]; then
          violations="${violations}${file}:${allow_start_line}:${normalized}\n"
        fi

        # Reset for next annotation
        in_allow=false
        allow_content=""
      fi
    fi
  done < "$file"

  if [ -n "$violations" ]; then
    echo "ERROR: Found disallowed #[allow] annotation in:"
    echo -e "$violations"
    echo ""
    echo "Only these TWO EXACT allow patterns are permitted:"
    echo "  1. #![cfg_attr(test, allow(clippy::tests_outside_test_module, reason = \"Allow for integration tests\"))]"
    echo "  2. #[allow(unsafe_code, reason = \"Arc<dyn Tool> is not Trace, but safe to use as documented above\")]"
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
cargo clippy --no-deps --bins --lib --tests --benches --all-features -- -D warnings

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