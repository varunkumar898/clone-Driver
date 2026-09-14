# DiskClone

DiskClone is a high-performance, safety-critical Linux disk cloning application engineered with a layered architecture: a Rust core storage and validation engine, an IPC bridge via Tauri v2, and a modern TypeScript/React desktop UI.

```
┌─────────────────────────────────────────────────────────────┐
│                    UI LAYER (TypeScript)                    │
│           Tauri Window + React Components                   │
│  (Disk Selection, Progress, Warnings, Logs, Verification)   │
└────────────────────────┬────────────────────────────────────┘
                         │
                    IPC Bridge (Tauri v2)
                         │
┌────────────────────────▼────────────────────────────────────┐
│           APPLICATION CORE (Rust Backend)                   │
│  • Job orchestration & state machine                        │
│  • Validation & safety checks                               │
│  • Clone workflow coordination                              │
│  • Error recovery & resume logic                            │
│  • Device discovery abstraction                             │
└────────────────────────┬────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────┐
│               STORAGE ENGINE (Rust)                         │
│  • Block reading/writing                                    │
│  • Buffer pool management                                   │
│  • Checksumming (xxHash64 + optional SHA256)                │
│  • Verification engine                                      │
│  • I/O queueing & adaptive chunking                         │
│  • Performance measurement                                  │
└────────────────────────┬────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────┐
│         LINUX STORAGE ABSTRACTION (Rust + C)                │
│  • Block device discovery (/sys/block, udev)                │
│  • Partition table parsing (GPT, MBR)                       │
│  • Filesystem detection (ext4, XFS, Btrfs, etc.)            │
│  • Device properties & metadata                             │
│  • Mount state detection                                    │
│  • Safe device opening (read-only source)                   │
│  • System disk detection                                    │
└────────────────────────┬────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────┐
│      KERNEL / BLOCK DEVICES (/dev/sdX, /dev/nvmeXnY)        │
└─────────────────────────────────────────────────────────────┘
```

## Features

- **4-Layer Safety Defense**:
  1. *Device Validation*: Rejects mounted targets, source/destination equality, and undersized targets.
  2. *Visual Confirmation*: Presents vendor, model, capacity, partition layout, and serial number.
  3. *Explicit Text Confirmation*: Requires typing exact confirmation string: `CLONE TO THIS DISK`.
  4. *System Disk Protection*: Hard-blocks operations targeting the root filesystem `/` or `/boot` partition.
- **High-Throughput I/O Engine**: Sequential aligned block streaming using `pread`/`pwrite` with adaptive chunking (4MB - 16MB).
- **Online & Post-Clone Verification**: Real-time xxHash64 block-level checksumming and optional cryptographic SHA256 verification.
- **Crash Recovery & Resumption**: Persistent write-ahead resume journal ensuring safe recovery after power loss or accidental disconnection.
- **Privilege Separation**: Unprivileged UI communicates across an audited IPC boundary to the elevated Rust backend.

## Quick Start

### Prerequisites

- Linux (x86_64, aarch64) with Kernel 5.4+
- Rust 1.77+ (`cargo`, `rustc`)
- Node.js 18+ and `npm`
- System development libraries: `libudev-dev`, `pkg-config`, `libwebkit2gtk-4.1-dev` (or distro equivalent)

### Building

```bash
# Build both Rust core and UI
./scripts/build.sh
```

### Running Tests

```bash
# Run unit and integration tests
./scripts/test.sh
```

### Running the CLI Engine (Dry-run / Headless)

```bash
cargo run -- --help
cargo run -- scan
```

## Documentation

- [USER_MANUAL.md](docs/USER_MANUAL.md): Complete desktop user guide and step-by-step instructions.
- [ARCHITECTURE.md](ARCHITECTURE.md): Detailed architectural layers, state machine, and data flow.
- [DESIGN.md](DESIGN.md): Technical decisions, chunk sizing, and buffer pool strategy.
- [SECURITY.md](SECURITY.md): Threat model, safety guarantees, and privilege model.
- [TESTING.md](TESTING.md): Testing strategy, mock fixtures, and test automation.
- [DEVELOPMENT.md](docs/DEVELOPMENT.md): Developer setup and contribution guide.
- [TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md): Common error recovery and debugging.
- [PERFORMANCE_TUNING.md](docs/PERFORMANCE_TUNING.md): I/O chunking and buffer tuning.
- [API.md](docs/API.md): Public Rust API and Tauri IPC command reference.
