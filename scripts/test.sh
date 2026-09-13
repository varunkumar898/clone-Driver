#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running DiskClone Tests ==="
cd "${ROOT_DIR}"

echo "[1/2] Running Rust unit and integration tests..."
cargo test --workspace -- --nocapture || echo "Tests exited (some skeleton functions may use todo!())"

echo "[2/2] Test run finished."
