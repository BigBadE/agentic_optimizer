#!/usr/bin/env bash
# Local (non-CI) benchmark runner for Merlin
# - Runs Quality Benchmarks -> quality-results.md
# - Runs Performance Benchmarks (Criterion wrapper) -> perf-results.md
# - Leaves gungraun to CI (Linux + Valgrind)

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

QUALITY_OUT="quality-results.md"
PERF_OUT="perf-results.md"
OPEN_REPORT=0
SKIP_BUILD=0

usage() {
  cat <<EOF
Usage: scripts/benchmark.sh [options]

Options:
  --open-report    Open Criterion HTML report after running (if available)
  --skip-build     Do not run cargo build first
  -h, --help       Show this help

Outputs:
  - $QUALITY_OUT   (quality benchmarks)
  - $PERF_OUT      (performance benchmarks)
  - target/criterion/ (raw criterion data + HTML report)

Next steps (CI will publish when pushed to master):
  git add -f $QUALITY_OUT $PERF_OUT
  git commit -m "chore: add local benchmark results"
  git push origin master
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --open-report) OPEN_REPORT=1; shift;;
    --skip-build)  SKIP_BUILD=1; shift;;
    -h|--help) usage; exit 0;;
    *) echo "Unknown option: $1"; usage; exit 2;;
  esac
done

info() { echo -e "\033[1;34m[bench]\033[0m $*"; }
success() { echo -e "\033[1;32m[done]\033[0m $*"; }
warn() { echo -e "\033[1;33m[warn]\033[0m $*"; }
err() { echo -e "\033[1;31m[fail]\033[0m $*"; }

# 0) Tooling check
if ! command -v cargo >/dev/null 2>&1; then
  err "cargo not found. Install Rust toolchain first: https://rustup.rs"
  exit 1
fi

# 1) Build (optional)
if [[ "$SKIP_BUILD" -eq 0 ]]; then
  info "Building in release mode..."
  cargo build --workspace --release
  success "Build complete"
else
  warn "Skipping build as requested"
fi

# 2) Run Quality Benchmarks (writes quality-results.md)
info "Running quality benchmarks..."
cargo run --release --bin quality-bench -- --output "$QUALITY_OUT"
success "Quality results -> $QUALITY_OUT"

# 3) Run Performance Benchmarks (writes perf-results.md and target/criterion)
# Exclude gungraun benchmarks (Linux/Valgrind only, run in CI)
info "Running performance benchmarks (excluding gungraun)..."
cargo run --release --bin perf-bench -- --output "$PERF_OUT"
success "Performance results -> $PERF_OUT"

# 4) Optionally open Criterion HTML report
REPORT_HTML="target/criterion/report/index.html"
if [[ "$OPEN_REPORT" -eq 1 ]]; then
  if [[ -f "$REPORT_HTML" ]]; then
    info "Opening Criterion report: $REPORT_HTML"
    if command -v xdg-open >/dev/null 2>&1; then xdg-open "$REPORT_HTML" >/dev/null 2>&1 || true; fi
    if command -v open >/dev/null 2>&1; then open "$REPORT_HTML" >/dev/null 2>&1 || true; fi
    if command -v start >/dev/null 2>&1; then start "" "$REPORT_HTML" >/dev/null 2>&1 || true; fi
  else
    warn "Criterion report not found at $REPORT_HTML"
  fi
fi

# 5) Summary
cat <<EOF

========================================
Local benchmarks completed successfully
========================================

Artifacts:
- $QUALITY_OUT
- $PERF_OUT
- target/criterion/ (raw data + HTML report)

To publish via CI (dashboards update automatically):
  git add -f $QUALITY_OUT $PERF_OUT
  git commit -m "chore: add local benchmark results"
  git push origin master

CI will:
- Parse and upload to gh-pages/quality-bench, gh-pages/perf-bench
- Update data/quality-latest.json and data/perf-latest.json
- Refresh https://bigbade.github.io/agentic_optimizer/
EOF
