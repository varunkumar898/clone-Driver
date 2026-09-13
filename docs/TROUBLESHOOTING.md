# Troubleshooting Guide

## Common Issues & Resolutions

### 1. "Permission Denied" opening block device
- **Cause**: User lacks raw read/write access to `/dev/sdX` or `/dev/nvmeXnY`.
- **Solution**: Run with elevated permissions via polkit or execute with `sudo` during development:
  ```bash
  sudo ./target/debug/diskclone
  ```

### 2. "Destination is mounted" validation error
- **Cause**: Target device has one or more active mounts recorded in `/proc/mounts`.
- **Solution**: Unmount all target partitions before proceeding:
  ```bash
  sudo umount /dev/sdX*
  ```

### 3. "System disk protection triggered"
- **Cause**: Target disk contains root `/`, `/boot`, or `/boot/efi`.
- **Solution**: The application intentionally blocks overwriting the operating system disk. Verify target device node.

### 4. Resuming an interrupted clone
- If a clone was halted, DiskClone looks for `.diskclone-journal.json`. The source and target drive serials and first-block hashes are matched before allowing resumption.
