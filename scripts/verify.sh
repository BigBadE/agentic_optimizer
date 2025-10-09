set -e

cargo fmt --all -q
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --no-fail-fast --lib --bins --tests -- --nocapture