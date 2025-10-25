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
cargo fmt -q
cargo clippy --no-deps --all-targets --all-features -- -D warnings

# Run full workspace tests
echo "[verify] Running tests..."
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
export MERLIN_FOLDER="${ROOT_DIR}/target/.merlin"

# Run main tests
cargo nextest run --run-ignored all

# Optionally run Ollama-specific tests by name filter
if [ "$RUN_OLLAMA" = true ]; then
  : "${OLLAMA_HOST:=http://127.0.0.1:11434}"
  export OLLAMA_HOST
  echo "[verify] Running Ollama-filtered tests (OLLAMA_HOST=$OLLAMA_HOST)"
  # Run tests that contain 'ollama' in their name across the workspace.
  # If none exist, this will simply find zero tests and succeed.
  cargo test --workspace ollama -- --nocapture || true
fi