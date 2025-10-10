#!/usr/bin/env bash
# Local (non-CI) benchmark runner for Merlin
# - Runs Quality Benchmarks -> quality-results.md
# - Runs Performance Benchmarks (Criterion wrapper) -> perf-results.md
# - Leaves gungraun to CI (Linux + Valgrind)

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

# Create benchmarks/data directory if it doesn't exist
mkdir -p benchmarks/data

QUALITY_OUT="benchmarks/data/quality-results.md"

usage() {
  cat <<EOF
Usage: scripts/benchmark.sh [options]

Outputs:
  - $QUALITY_OUT   (quality benchmarks)
  - target/criterion/ (performance benchmarks - raw criterion data + HTML report)

Next steps (CI will publish when pushed to master):
  git add -f $QUALITY_OUT
  git commit -m "chore: add local benchmark results"
  git push origin master
  
Options:
  --debug    Run in the current terminal/process without CPU pinning wrapper. Useful to debug and
             to observe live output directly. This disables the Windows hidden-window runner.
EOF
}

info() { echo -e "\033[1;34m[bench]\033[0m $*"; }
success() { echo -e "\033[1;32m[done]\033[0m $*"; }
warn() { echo -e "\033[1;33m[warn]\033[0m $*"; }
err() { echo -e "\033[1;31m[fail]\033[0m $*"; }

# Flags
DEBUG_MODE=0

# Parse args (only --debug for now)
while [[ $# -gt 0 ]]; do
  case "$1" in
    --debug)
      DEBUG_MODE=1; shift ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      err "Unknown option: $1"; usage; exit 2 ;;
  esac
done

# Deterministic env for more stable runs
export RUST_BACKTRACE=0
export RUST_LOG=off

# Reduce external noise in criterion
export CRITERION_MEASUREMENT_TIME=10
export CRITERION_WARM_UP_TIME=2
export CRITERION_SAMPLE_SIZE=50
export CRITERION_NOISE_THRESHOLD=0.02

# Helper to run with CPU affinity while preserving exit codes and elevating priority
run_with_affinity() {
  # In debug mode run inline to preserve live output and avoid hidden window
  if [[ $DEBUG_MODE -eq 1 ]]; then
    "$@"
    return $?
  fi

  if [[ "${OS:-}" == "Windows_NT" ]]; then
    # Create temporary files and script
    local ps1 out_file err_file
    if ps1=$(mktemp -t merlin_bench_psXXXXXX.ps1 2>/dev/null); then :; else ps1="${TMPDIR:-${TMP:-/tmp}}/merlin_bench_ps$$.ps1"; : >"$ps1"; fi
    out_file=$(mktemp 2>/dev/null || mktemp -t merlin_bench_out)
    err_file=$(mktemp 2>/dev/null || mktemp -t merlin_bench_err)

    cat >"$ps1" <<'PS1'
$outPath = $args[0]
$errPath = $args[1]
$exe = $args[2]
if ($args.Length -gt 3) { $procArgs = $args[3..($args.Length-1)] } else { $procArgs = @() }
$p = Start-Process -FilePath $exe -ArgumentList $procArgs -WindowStyle Hidden -RedirectStandardOutput $outPath -RedirectStandardError $errPath -PassThru
Start-Sleep -Milliseconds 100
try { (Get-Process -Id $p.Id).ProcessorAffinity = 1 } catch {}
try { (Get-Process -Id $p.Id).PriorityClass = 'High' } catch {}
try { (Get-Process -Id $p.Id).PriorityBoostEnabled = $false } catch {}
$p.WaitForExit()
exit $p.ExitCode
PS1

    # Invoke powershell in background so we can live-stream outputs
    local exe="$1"; shift
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$ps1" "$out_file" "$err_file" "$exe" "$@" &
    local ps_pid=$!
    # Stream outputs until process exits
    tail -n +1 -f "$out_file" & local tail_out=$!
    tail -n +1 -f "$err_file" >&2 & local tail_err=$!
    wait $ps_pid; local code=$?
    kill $tail_out >/dev/null 2>&1 || true
    kill $tail_err >/dev/null 2>&1 || true
    rm -f "$ps1" "$out_file" "$err_file" || true
    return $code
  elif command -v taskset >/dev/null 2>&1; then
    if command -v ionice >/dev/null 2>&1; then
      ionice -c2 -n0 taskset -c 0 nice -n -10 "$@"
    else
      taskset -c 0 nice -n -10 "$@"
    fi
  else
    "$@"
  fi
}

# Attempts to normalize results and prevent inconsistent benchmarks
if [[ "${OSTYPE:-}" == msys* || "${OSTYPE:-}" == cygwin* ]]; then
  echo "[*] Setting Windows power mode to High Performance..."
  powershell.exe -NoProfile -Command "powercfg -setactive SCHEME_MIN" || true
fi

# Run Quality Benchmarks (writes quality-results.md)
info "Running quality benchmarks..."
export MERLIN_FOLDER="${ROOT_DIR}/target/.merlin"
run_with_affinity cargo run --release --bin quality-bench -- --output "$QUALITY_OUT"
success "Quality results -> $QUALITY_OUT"

# Run Performance Benchmarks (runs Criterion benchmarks, outputs to target/criterion)
info "Running performance benchmarks..."
run_with_affinity cargo bench -p merlin-benchmarks-criterion
success "Performance benchmarks complete"

# Parse criterion results to JSON
info "Parsing criterion results to JSON..."
if ! command -v python >/dev/null 2>&1; then
  warn "python not found; skipping criterion JSON parsing"
else
  python scripts/parse-benchmarks.py \
    --quality-results "$QUALITY_OUT" \
    --criterion-dir target/criterion \
    --output-dir benchmarks/data
  rm "$QUALITY_OUT"
  success "Criterion JSON -> benchmarks/data/criterion/latest.json"
fi

# Summary
cat <<EOF

========================================
Local benchmarks completed successfully
========================================

Artifacts:
- benchmarks/data/quality/latest.json
- benchmarks/data/criterion/latest.json
- target/criterion/ (raw data + HTML report)

To publish via CI (dashboards update automatically):
  git add -f $QUALITY_OUT benchmarks/data/criterion/latest.json
  git commit -m "chore: add local benchmark results"
  git push origin master

CI will:
- Upload to gh-pages
- Update data/quality/latest.json and data/criterion/latest.json
- Refresh https://bigbade.github.io/agentic_optimizer/
EOF
