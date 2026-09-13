# Security Analysis & Threat Model

## Threat Model

1. **Malicious Removable Devices / Firmware Manipulation**
   - *Threat*: Specially crafted USB drives attempting exploit payload injection during partitioning or filesystem discovery.
   - *Mitigation*: Strictly bounds-checked binary parsers for MBR/GPT structures in safe Rust. Avoid invoking external unvetted binaries with untrusted shell arguments.

2. **Device Identity Spoofing & Swap Attacks**
   - *Threat*: A device disconnects and a different device assumes the same kernel node (e.g. `/dev/sdb`), causing accidental destruction of data upon write resume.
   - *Mitigation*: Hard identity binding via WWN, hardware serial number, and initial block verification checksum comparison before resuming write operations.

3. **Accidental Data Loss**
   - *Threat*: User selects system drive or active data drive by mistake.
   - *Mitigation*: Four distinct validation layers. System disk detection rejects root filesystem and boot partitions. Mandatory exact string typing `"CLONE TO THIS DISK"` ensures conscious user intent.

4. **Privilege Escalation Vulnerabilities**
   - *Threat*: Malicious local user uses the disk cloning IPC bridge to execute arbitrary commands or overwrite arbitrary files.
   - *Mitigation*: Minimal elevated backend helper exposed strictly over polkit or Unix domain socket. The IPC interface does not accept arbitrary file paths or shell commands—only validated block device descriptors.

5. **Filesystem Corruption on Mounted Disks**
   - *Threat*: Writing raw blocks to a drive with mounted filesystems leads to filesystem inconsistency and kernel panic.
   - *Mitigation*: Real-time inspection of `/proc/self/mountinfo`. Any mounted partition on the destination target immediately blocks write operations until unmounted.

---

## Safety Guarantees

- **System Disk Immobility**: The host root disk `/` cannot be targeted as a clone destination under standard operation.
- **Source Immutability**: Source device descriptors are opened with `O_RDONLY` and never written to.
- **Atomic Verification**: Cloned devices are verified with block-level checksum validation before being reported as completed.
