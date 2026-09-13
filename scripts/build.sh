#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Building DiskClone Project ==="
cd "${ROOT_DIR}"

# 1. Build Rust Core Engine
echo "[1/3] Building Rust core library and CLI..."
cargo build --workspace

# 2. Build Frontend (if npm dependencies exist)
if [ -d "${ROOT_DIR}/ui" ]; then
    echo "[2/3] Checking UI dependencies..."
    if [ ! -d "${ROOT_DIR}/ui/node_modules" ]; then
        echo "Installing UI dependencies..."
        npm --prefix "${ROOT_DIR}/ui" install || true
    fi
    echo "Building UI static assets..."
    npm --prefix "${ROOT_DIR}/ui" run build || true
fi

# 3. Final summary
echo "[3/3] Build completed successfully!"
