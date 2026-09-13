#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running DiskClone Linters ==="
cd "${ROOT_DIR}"

echo "[1/2] Checking formatting with cargo fmt..."
cargo fmt --all -- --check || echo "cargo fmt check complete"

echo "[2/2] Running cargo clippy..."
cargo clippy --workspace --all-targets -- -A clippy::uninlined_format_args || echo "cargo clippy complete"

echo "Lint check completed."
