#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running DiskClone Continuous Integration ==="
cd "${ROOT_DIR}"

"${SCRIPT_DIR}/lint.sh"
"${SCRIPT_DIR}/build.sh"
"${SCRIPT_DIR}/test.sh"

echo "=== CI Pipeline Completed Successfully ==="
