#!/usr/bin/env bash

# Get the project root directory (parent of scripts/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

export MERLIN_FOLDER="${PROJECT_ROOT}/.merlin"
cargo run -- -p benchmarks/crates/quality/test_repositories/valor --context-dump