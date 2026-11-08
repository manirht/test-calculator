#!/usr/bin/env sh
# Run every test suite: Rust (fmt, clippy, tests), Python, TypeScript.
set -eu
cd "$(dirname "$0")/.."

echo "== Rust"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

echo "== Python"
(cd python && python3 -m unittest discover tests)

echo "== TypeScript"
(cd ts && npm ci --silent && npm test)
