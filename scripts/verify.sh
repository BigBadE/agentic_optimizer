#!/usr/bin/env bash

set -euo pipefail

# Usage:
#   ./scripts/verify.sh [--no-cloud] [--ollama]
#
# Flags:
#   --no-cloud  Unset cloud provider API keys to prevent running GROQ/OpenRouter/Anthropic tests
#   --ollama    Additionally run tests filtered to "ollama" (requires a local Ollama server)

NO_CLOUD=false
RUN_OLLAMA=false

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
  echo "[verify] Cloud provider tests disabled (--no-cloud)"
fi

# Format, lint, and test
cargo fmt --all -q
cargo clippy --all-targets --all-features -- -D warnings

# Run full workspace tests with coverage
echo "[verify] Running tests with coverage instrumentation..."
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
export MERLIN_FOLDER="${ROOT_DIR}/target/.merlin"
mkdir -p benchmarks/data/coverage

# Run main tests with coverage
cargo llvm-cov --no-report --ignore-filename-regex "test_repositories|benchmarks" --all-features --workspace --no-fail-fast --lib --bins --tests -- --nocapture

# Run ignored env var tests serially with coverage
echo "[verify] Running ignored env var tests (serial)..."
cargo llvm-cov --no-report --ignore-filename-regex "test_repositories|benchmarks" -p merlin-context --lib -- --ignored --test-threads=1

# Generate coverage report
cargo llvm-cov report --lcov --output-path benchmarks/data/coverage/latest.info
git add benchmarks/data/coverage/latest.info
cargo sweep --time 1 -r

# Optionally run Ollama-specific tests by name filter
if [ "$RUN_OLLAMA" = true ]; then
  : "${OLLAMA_HOST:=http://127.0.0.1:11434}"
  export OLLAMA_HOST
  echo "[verify] Running Ollama-filtered tests (OLLAMA_HOST=$OLLAMA_HOST)"
  # Run tests that contain 'ollama' in their name across the workspace.
  # If none exist, this will simply find zero tests and succeed.
  cargo test --workspace ollama -- --nocapture || true
fi