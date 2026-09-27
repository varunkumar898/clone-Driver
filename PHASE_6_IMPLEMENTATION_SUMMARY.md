# Phase 6: Safety Confirmations & Protections — Architectural Implementation

## Architecture Overview

Phase 6 implements the multi-tiered safety gate that sits directly between device selection and storage engine execution, preventing catastrophic user errors such as accidental overwriting of running operating systems or cloning onto undersized targets.

```
Device Selection (Phase 3)
         │
         ▼
[ CloneState::VALIDATING ]
         │
         ├─► Layer 0: System Disk Detection (/proc/cmdline, /proc/mounts)
         ├─► Layer 1: Capacity Validation (Dest ≥ Src, slack calculation)
         │
         ▼
IPC: confirm_visual_check()
         │
         ▼
[ CloneState::AWAITING_CONFIRMATION ]
         │
         ▼ User acknowledges source/destination details & risks
[ CloneState::AWAITING_TEXT_CONFIRMATION ]
         │
         ▼
IPC: confirm_text_input("CLONE TO THIS DISK")
         │
         ▼ Exact case-sensitive match validated
[ CloneState::READY_TO_CLONE ]
         │
         ▼
IPC: final_safety_check() (Defense-in-depth re-verification)
         │
         ▼
[ CloneState::PREPARING ]
         │
         ▼
Phase 5 Storage Engine: CloneEngine::execute()
```

---

## Component Breakdown

### 1. `src/device/safety.rs`
- **`SystemDiskDetector`**:
  - `detect_boot_device()`: Scans `/proc/cmdline` for `root=` and parses device paths or UUIDs.
  - `get_mount_point(path)`: Scans `/proc/mounts` to detect if the target is mounted as `/`, `/boot`, or `/boot/efi`.
  - `is_system_disk(path)`: Combined multi-layer inspection checking root cmdline and active mounts.
  - `disk_from_partition()` & `devices_overlap()`: Maps child partitions (`nvme0n1p2`, `sda1`) to parent disks (`nvme0n1`, `sda`).
- **`CapacityValidator`**:
  - `validate(source_size, dest_device)`: Enforces `dest >= source` and issues slack warnings if `< 5%`.
  - `validate_capacity(source_bytes, dest_bytes)`: Direct byte comparison validation.
  - `get_remaining_capacity(dest_device)`: Calculates remaining unallocated disk space from partition tables.

### 2. `src/confirm/confirmation.rs`
- **`ConfirmationRequest`**: DTO containing source and destination model, capacity, and serial number. Includes `format_summary()` for clean UI presentation.
- **`ConfirmationResponse`**: Records boolean flags for visual acknowledgment, text validation, and the raw text provided.
- **`ConfirmationManager`**: State manager providing:
  - `validate_text_confirmation(text)`: Enforces exact match against `"CLONE TO THIS DISK"`.
  - `validate_full_confirmation()`: Ensures both visual and text steps completed.
  - `final_check_system_disk(dest_path)`: Re-checks system disk status.

### 3. `src/state/mod.rs` & `src/state_machine/`
- State definitions updated with confirmation phases:
  - `AWAITING_CONFIRMATION`
  - `AWAITING_TEXT_CONFIRMATION`
  - `READY_TO_CLONE`
- Transition rules:
  - Linear confirmation gate cannot be bypassed.
  - Retrying incorrect text confirmation allowed without restarting the entire workflow.
  - Graceful cancellation to `IDLE` supported from any confirmation state.

### 4. `src/commands/confirm.rs`
- Pure Rust handlers that serve as the backend implementation for Tauri IPC commands:
  - `handle_confirm_visual_check`: Prepares confirmation data and moves state to `AWAITING_CONFIRMATION`.
  - `handle_confirm_text_input`: Validates phrase and moves state to `READY_TO_CLONE`.
  - `handle_final_safety_check`: Performs defense-in-depth re-checks before transitioning to `PREPARING`.

### 5. `src/error.rs`
- Extended `StorageError` with specific safety error variants:
  - `SystemDiskProtected`
  - `InsufficientCapacity`
  - `TextConfirmationFailed`
  - `ConfirmationStateError`
  - `CannotQueryDeviceSize`
  - `InvalidDevicePath`

---

## Test Coverage

- **Unit Tests**:
  - `tests/unit/system_disk_detection.rs`: Mocked `/proc/cmdline` and `/proc/mounts`, device overlap, NVMe/SATA partition parsing, fail-closed handling.
  - `tests/unit/state_confirmation.rs`: Sequential transitions, illegal skip rejections, retry loops, abort paths.
- **Integration Tests**:
  - `tests/integration/safety_checks.rs`: End-to-end flow combining safety checks, confirmation manager, and state machine transitions.
- **Total Tests Passing**: 45+ Phase 6 safety-related test assertions, 100% pass rate.
