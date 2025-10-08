#!/bin/bash
# Script to identify and categorize all remaining clippy warnings

echo "=== Clippy Warning Analysis ==="
echo ""

echo "Total errors:"
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | grep "^error:" | wc -l

echo ""
echo "=== By Category ==="

echo "Missing documentation:"
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | grep "missing documentation" | wc -l

echo "Missing must_use:"
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | grep "must_use" | wc -l

echo "Missing backticks:"
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | grep "missing backticks" | wc -l

echo "Other errors:"
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | grep "^error:" | grep -v "missing documentation" | grep -v "must_use" | grep -v "missing backticks" | wc -l

echo ""
echo "=== Files with most errors ==="
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | grep "\.rs:" | sed 's/-->.*//' | sed 's/.*-->//' | sort | uniq -c | sort -rn | head -20
