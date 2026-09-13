# DiskClone Design Decisions

## 1. Storage Engine I/O Architecture

### Block Reading & Writing: `pread` / `pwrite`
- Rather than relying on standard stream abstraction (`std::io::Read` / `Write`), we use positional I/O (`pread64` / `pwrite64`).
- **Rationale**:
  - Independent offsets eliminate race conditions when read and write pipelines operate asynchronously across worker threads.
  - Aligned block transfers prevent unaligned sector penalty on modern Advanced Format (4Kn / 512e) drives and NVMe storage.
  - Allows direct restartability at any exact sector offset during resume operations.

### Aligned Memory Buffer Pool
- A fixed-capacity pool of pre-allocated, page-aligned buffers (typically 4MB - 16MB each) is initialized before cloning starts.
- **Rationale**:
  - Eliminates heap allocation during high-speed I/O loops.
  - Buffer reuse prevents memory fragmentation and eliminates OS page fault latency in tight I/O loops.

### Checksum Algorithm: xxHash64 + Optional SHA256
- **xxHash64**: Executed on every block chunk during cloning. Yields throughput exceeding 10 GB/s per core, leaving CPU time free for disk saturation without bottle-necking NVMe drives.
- **SHA256**: Available as an optional cryptographic verification pass for forensic, compliance, or backup integrity standards.

---

## 2. Safety Defense Hierarchy

```
Layer 1: Device Validation
  ├─ Source must exist and be readable
  ├─ Destination must exist and be writable
  ├─ Destination must not be mounted
  └─ Destination capacity must be >= source capacity

Layer 2: Visual Confirmation
  ├─ Show SOURCE device:
  │   - Model, manufacturer, capacity
  │   - Mounted partitions (if any)
  │   - Serial number (unique identification)
  └─ Show DESTINATION device:
      - Model, manufacturer, capacity
      - Mounted partitions (if any)
      - Serial number

Layer 3: Explicit Text Confirmation
  ├─ Display: "ALL DATA ON [DEVICE] WILL BE PERMANENTLY ERASED"
  ├─ Require exact typing: "CLONE TO THIS DISK"
  └─ Single checkbox is NOT sufficient

Layer 4: System Disk Detection
  ├─ Detect if destination == root device (/)
  ├─ Detect if destination contains /boot or /boot/efi
  ├─ If true: HARD BLOCK operation
  └─ Never allow without explicit developer debug overrides
```

---

## 3. Resume & Journal Architecture

```
When resuming a clone:
1. Load resume journal (device serials, offsets, checksums)
2. Scan current devices
3. Match by:
   - Serial number (primary)
   - Model + capacity (fallback)
4. If match uncertain → require user to re-confirm
5. Verify first block checksums match
6. Only then proceed with resume
```

The journal file is updated synchronously at checkpoint intervals (e.g., every 500MB or on pause).

---

## 4. Privilege Architecture

```
User Desktop Session (unprivileged)
         │
         ├─→ [Tauri Frontend UI]
         │     ↓ (user interactions)
         │     │ (JSON-RPC over IPC)
         │
         └─→ [systemd-user service OR polkit-activated daemon]
               │
               ├─→ [Rust Backend - elevated privileges]
                    ├─ Open /dev/sdX (write access)
                    ├─ ioctl() for device properties
                    └─ pread/pwrite for I/O
```
