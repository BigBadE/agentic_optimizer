TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT_DIR}/target}"
export MERLIN_FOLDER="${TARGET_DIR}/.merlin"
cargo run -- -p benchmarks/crates/quality/test_repositories/valor --context-dump