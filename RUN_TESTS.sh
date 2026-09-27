#!/usr/bin/env bash
set -e

echo "=========================================="
echo " Running DiskClone Test & Validation Suite"
echo "=========================================="

echo "==> Running Safety Unit & Integration Tests..."
cargo test --test integration_safety_checks --test unit_system_disk_detection --test unit_state_confirmation

echo "==> Running Phase 7 Resume/Recovery Tests..."
cargo test --test unit_resume_recovery
cargo test --lib resume

echo "==> Running Phase 8 UI & IPC State Tests..."
cargo test --lib ui

echo "==> Running Safety Module Tests..."
cargo test --lib device::safety::tests
cargo test --lib confirm::confirmation::tests
cargo test --lib state::tests
cargo test --lib commands::confirm::tests

echo "==> Running Full Test Suite..."
cargo test

echo "==> Running Clippy Checks..."
cargo clippy --lib --tests -- -D warnings

echo "==> Checking Formatting..."
cargo fmt --check

echo "==> Building Frontend UI..."
npm --prefix ui run build

echo "=========================================="
echo " ✅ All DiskClone Tests & Checks Passed!"
echo "=========================================="
