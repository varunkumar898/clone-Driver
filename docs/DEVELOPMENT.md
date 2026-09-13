# Development Guide

## Development Workflow

For every feature and change:
1. Understand the requirement and safety implications.
2. Inspect existing modules and dependencies.
3. Identify affected components (Storage Engine, State Machine, Linux Abstraction, Safety, UI).
4. Implement minimal, well-encapsulated code.
5. Verify with automated unit tests and integration tests.
6. Run formatting (`cargo fmt`) and static analysis (`cargo clippy`).
7. Perform security and safety verification.

## Prerequisites

- Arch Linux, Ubuntu 22.04+, or Fedora 38+
- Rust 1.77+ (`rustup default stable`)
- Node.js 18+ and npm
- `libudev-dev`, `pkg-config`, `libwebkit2gtk-4.1-dev`

## Common Commands

```bash
# Build Rust backend and frontend
./scripts/build.sh

# Run all tests
./scripts/test.sh

# Linting and formatting checks
./scripts/lint.sh
```
