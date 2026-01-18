#!/usr/bin/env bash
set -euo pipefail

echo "Running formatting checks..."
cargo fmt --check

echo "Running clippy..."
cargo clippy --all-targets --all-features

echo "Running compilation check..."
cargo check --all-targets --all-features

echo "Running tests..."
cargo test
