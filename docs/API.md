# DiskClone API Reference

## Public Rust Library API

### Module Structure
- `diskclone::state_machine`: State machine lifecycle, state types, transition rules.
- `diskclone::device`: Hardware discovery, block device abstraction, system disk checking.
- `diskclone::partition`: Partition table parsing (GPT and MBR).
- `diskclone::storage`: Storage engine, buffer pool, checksum calculation, verification.
- `diskclone::linux`: Direct Linux syscalls, `/sys/block`, `udev`, mountinfo.
- `diskclone::safety`: Multi-stage validation, confirmation tokens, device identity matching.
- `diskclone::resume`: Journaling, checkpoints, and clone recovery.
- `diskclone::logging`: Tracing setup, structured log formatters.
- `diskclone::privilege`: Privilege escalation and helper process abstraction.
- `diskclone::error`: Comprehensive strongly-typed error enums.

---

## Tauri IPC API Commands

```typescript
// Scan and list eligible block devices
invoke('scan_devices'): Promise<BlockDevice[]>

// Start a new clone job with confirmation token
invoke('start_clone', { sourcePath: string, destPath: string, confirmationToken: string }): Promise<void>

// Pause active cloning job
invoke('pause_clone'): Promise<void>

// Resume interrupted or paused clone job
invoke('resume_clone'): Promise<void>

// Gracefully cancel active clone job
invoke('cancel_clone'): Promise<void>

// Trigger block-level post-clone verification
invoke('verify_clone'): Promise<VerificationReport>

// Retrieve recent in-memory structured log events
invoke('get_logs'): Promise<LogEntry[]>
```
