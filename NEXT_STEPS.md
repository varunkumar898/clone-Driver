# Next Steps: DiskClone Post-Phase 6 Roadmap

With Phase 6 (Safety Confirmations & Protections) complete and verified, the next milestones focus on system permissions and user interface integration.

---

## Phase 7: Privilege Escalation & Helper Daemon
- **Goal**: Allow DiskClone to perform raw block I/O (`/dev/sdX`, `/dev/nvmeXn1`) without running the entire graphical Tauri frontend as `root`.
- **Key Deliverables**:
  1. **Polkit Policy (`org.diskclone.policy`)**: Define action rules for root helper execution.
  2. **Privilege Separation Daemon / Helper**:
     - Lightweight helper binary executed via `pkexec` or UNIX domain socket.
     - Performs restricted privileged operations: opening block device file descriptors with `O_DIRECT | O_EXCL` and passing descriptors back via `SCM_RIGHTS`.
  3. **Elevation Prompts**: User-friendly authentication prompt triggers prior to clone execution.

---

## Phase 8: Tauri UI & IPC Integration
- **Goal**: Connect frontend React/Vite UI components with the Phase 6 IPC handlers and Phase 5 engine.
- **Key Deliverables**:
  1. **Tauri IPC Command Registrations**:
     - Wire `confirm_visual_check`, `confirm_text_input`, and `final_safety_check` in `src-tauri`.
  2. **Safety Modal Dialogs**:
     - Visual confirmation card displaying source & target drives side-by-side with warning badges.
     - Text entry prompt requiring exact `"CLONE TO THIS DISK"` string with active validation feedback.
  3. **Live Progress Bar & Telemetry**:
     - Hook real-time transfer rates (MB/s), ETA countdown, and chunk verification stats into UI state.

---

## Phase 9: Forensic & Checkpoint Verification
- **Goal**: Full-disk hash verification and resume journal management.
- **Key Deliverables**:
  1. Background verification passes post-clone (xxHash64 & SHA256).
  2. Crash-recovery journal resume tests on simulated interrupted block devices.
