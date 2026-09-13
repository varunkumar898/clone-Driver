# DiskClone Architecture Specification

## 1. System Layers

DiskClone is architected in four distinct operational layers, ensuring clear boundaries between visualization, orchestration, high-speed storage I/O, and low-level Linux kernel primitives.

### Layer 1: UI Layer (TypeScript + React)
- **Host**: Tauri Webview runtime (Linux WebKitGTK).
- **Responsibilities**: Device discovery display, multi-layer confirmation dialogs, real-time progress indicators, interactive throughput graphs, log streams, and verification reports.
- **Security Boundary**: Runs completely unprivileged in the user session without direct filesystem or raw device access.

### Layer 2: Application Core (Rust)
- **Module**: `src/state_machine/`, `src/safety/`, `src/resume/`
- **Responsibilities**:
  - Orchestrates execution through an explicit finite state machine.
  - Implements safety policies (root filesystem protection, mount rejection, size validation).
  - Maintains crash-resilient journal files for clone resumption.
  - Handles lifecycle events (pause, resume, cancellation, error classification).

### Layer 3: Storage Engine (Rust)
- **Module**: `src/storage/`
- **Responsibilities**:
  - Direct block reading and writing (`pread64`/`pwrite64`).
  - Pre-allocated, memory-aligned `BufferPool` to eliminate GC or allocation jitter.
  - Checksum streaming using xxHash64 and optional SHA256.
  - Adaptive I/O chunking (adjusting between 4MB and 16MB based on latency and throughput).
  - Post-clone block verification pass.

### Layer 4: Linux Storage Abstraction (Rust + C)
- **Module**: `src/linux/`, `src/device/`, `src/partition/`, `src/privilege/`
- **Responsibilities**:
  - Device enumeration via `/sys/block` and `udev`.
  - Partition table parsing (GPT and MBR).
  - Filesystem signature and LUKS encrypted volume detection.
  - Mount state detection via `/proc/self/mountinfo`.
  - Safe descriptor opening (`O_RDONLY` for source, `O_WRONLY | O_SYNC` for destination).
  - Privilege management (polkit / helper abstraction).

---

## 2. Finite State Machine Workflow

```
IDLE
  ↓
[User clicks "Clone Disk"]
  ↓
SCANNING_DEVICES
  ├─ Discover /sys/block devices
  ├─ Query device properties
  ├─ Detect mount status
  ├─ Detect encryption (LUKS)
  └─ Report findings to UI
  ↓
AWAITING_SOURCE_SELECTION
  ├─ Display device cards
  └─ User selects source
  ↓
AWAITING_DESTINATION_SELECTION
  ├─ Filter ineligible devices (system disk, mounted)
  ├─ Display remaining candidates
  └─ User selects destination
  ↓
VALIDATING_SELECTION
  ├─ Verify source not system disk
  ├─ Verify destination not mounted
  ├─ Check destination capacity >= source capacity
  ├─ Verify device permissions
  └─ Warn if unsafe situation detected
  ↓
AWAITING_CONFIRMATION
  ├─ Show source/destination summary
  ├─ Show "ALL DATA WILL BE ERASED" warning
  ├─ Require user to type: "CLONE TO THIS DISK"
  └─ Only proceed if exact confirmation provided
  ↓
PREPARING
  ├─ Open source device read-only
  ├─ Open destination device write (with safety checks)
  ├─ Initialize buffer pools
  ├─ Start progress tracking
  └─ Create resume journal
  ↓
CLONING
  ├─ Read source blocks in sequence
  ├─ Compute checksums
  ├─ Write to destination
  ├─ Maintain progress (bytes/speed/ETA)
  ├─ Handle pause/resume
  └─ On error → CLONING_ERROR
  ↓
FLUSH
  ├─ Flush destination buffers to disk
  ├─ Sync filesystem metadata
  └─ Verify write persistence
  ↓
VERIFYING
  ├─ Compare source & destination (block checksums)
  ├─ Verify partition tables match
  ├─ Verify bootloader metadata preserved
  └─ Report pass/fail
  ↓
COMPLETED
  ├─ Show "Clone successful" summary
  ├─ Display verification results
  └─ Allow view logs / eject destination
```

### Error and Cancellation Transitions
```
ANY_STATE → ERROR
  ├─ Log error with full context
  ├─ Close devices safely
  ├─ Save resume state
  ├─ Display actionable error message
  └─ Offer retry / cancel options

CLONING / VERIFYING → CANCELLING
  ├─ Stop I/O gracefully
  ├─ Flush any pending writes
  ├─ Save resume journal
  └─ Return to IDLE (allow resume later)
```

---

## 3. Storage I/O Pipeline

```
┌──────────────┐
│ Source Read  │ ← pread() at offset X, size CHUNK_SIZE
│ (async)      │
└──────┬───────┘
       │ [Buffer: raw bytes]
       ↓
┌──────────────────────┐
│ Checksum Compute     │ ← xxHash64 for fast monitoring
│ (xxHash64)           │
└──────┬───────────────┘
       │ [Checksum: u64]
       ↓
┌──────────────────────┐
│ Destination Write    │ ← pwrite() at offset X, size CHUNK_SIZE
│ (async)              │
└──────┬───────────────┘
       │
       ↓
┌──────────────────────┐
│ Progress Update      │
│ (bytes written)      │
└──────────────────────┘
```

Parallel queues:
- **Read queue**: Prefetches blocks ahead into allocated buffers.
- **Write queue**: Flushes populated buffers sequentially to the target.
- **Verify queue**: Compares block hashes either concurrently or during a dedicated verification pass.
