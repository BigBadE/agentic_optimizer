#!/usr/bin/env bash
#
# Comprehensive fixture timing analysis script
#
# Usage:
#   ./scripts/timings.sh [--export FILE.json] [--flamegraph]
#
# Flags:
#   --export FILE       Export timing data to JSON file
#   --flamegraph        Generate flamegraph profile (requires Linux/WSL)
#
# This script runs fixture tests with full timing instrumentation and generates
# a comprehensive report including:
#   - Per-category timing breakdown
#   - Function-level timing breakdown
#   - Slowest individual fixtures
#   - Overall performance metrics
#   - Parallelization efficiency
#   - Optional flamegraph generation
#
# Note: Timing layer is always enabled to capture tracing output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$ROOT_DIR"

# Find working Python executable
PYTHON_CMD=""

# Try python first (more common on Windows)
if command -v python &> /dev/null && python --version &> /dev/null; then
    PYTHON_CMD="python"
# Then try python3
elif command -v python3 &> /dev/null && python3 --version &> /dev/null; then
    PYTHON_CMD="python3"
fi

if [ -z "$PYTHON_CMD" ]; then
    echo "❌ Error: python is required but not found in PATH"
    echo "Please install Python 3.x and ensure it's in your PATH"
    exit 1
fi

# Run the Python timing analyzer with all arguments passed through
exec "$PYTHON_CMD" "$SCRIPT_DIR/timings.py" "$@"
