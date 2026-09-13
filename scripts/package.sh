#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Packaging DiskClone ==="
cd "${ROOT_DIR}"

mkdir -p "${ROOT_DIR}/dist"
cargo build --release --bin diskclone
cp "${ROOT_DIR}/target/release/diskclone" "${ROOT_DIR}/dist/"

echo "Packaged binary located at: ${ROOT_DIR}/dist/diskclone"
