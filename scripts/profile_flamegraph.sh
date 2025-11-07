#!/usr/bin/env bash
# Run flamegraph profiling on fixture tests

set -euo pipefail

cd "$(dirname "$0")/.."

echo "=== Flamegraph Profiling ==="
echo ""
echo "Running fixture tests with flamegraph profiling..."
echo "This will generate a flamegraph.svg file showing CPU usage breakdown"
echo ""
echo "Requirements:"
echo "  - Linux/WSL: Uses 'perf' for sampling"
echo "  - Windows: Requires admin privileges or use 'samply' instead"
echo ""

# Run flamegraph on the fixture tests with release profile
# --release is needed for proper profiling and to enable LTO
cargo flamegraph --release --test fixture_tests --package integration-tests -- test_all_fixtures --exact --nocapture

echo ""
echo "✓ Flamegraph generated: flamegraph.svg"
echo "Open this file in a web browser to view the interactive flamegraph"
