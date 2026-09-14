# DiskClone Desktop User Manual

Welcome to **DiskClone**, a safety-critical Linux disk cloning application designed for high-performance block-level drive cloning, system migrations, and forensic verification.

---

## Table of Contents

1. [Prerequisites & System Requirements](#1-prerequisites--system-requirements)
2. [Launching the Desktop Application](#2-launching-the-desktop-application)
3. [Step-by-Step Desktop Cloning Guide](#3-step-by-step-desktop-cloning-guide)
   - [Step 1: Device Discovery](#step-1-device-discovery)
   - [Step 2: Source Drive Selection](#step-2-source-drive-selection)
   - [Step 3: Destination Drive Selection](#step-3-destination-drive-selection)
   - [Step 4: Safety & Compatibility Verification](#step-4-safety--compatibility-verification)
   - [Step 5: Explicit Text Confirmation](#step-5-explicit-text-confirmation)
   - [Step 6: Live Progress & Throughput Monitoring](#step-6-live-progress--throughput-monitoring)
   - [Step 7: Post-Clone Verification Pass](#step-7-post-clone-verification-pass)
4. [Handling Errors & Interrupted Jobs](#4-handling-errors--interrupted-jobs)
5. [CLI / Headless Usage (Alternative)](#5-cli--headless-usage-alternative)

---

## 1. Prerequisites & System Requirements

- **Operating System**: Linux 64-bit (x86_64, aarch64) with Linux Kernel 5.4+ (Arch, Ubuntu 22.04+, Fedora, Debian).
- **Desktop Environment**: GNOME, KDE Plasma, XFCE, Hyprland, or any X11/Wayland desktop with WebKitGTK 4.1 support.
- **Permissions**: Raw block device write operations require elevated privileges. Run DiskClone using `sudo` or via Polkit elevation.

---

## 2. Launching the Desktop Application

### Option A: Running in Development Mode
```bash
# Clone repository & launch with live-reload
npm run dev
# OR launch with Tauri CLI
cargo tauri dev
```

### Option B: Building & Running the Production Desktop App
```bash
# Build the release bundle
./scripts/build.sh

# Run elevated backend executable
sudo ./dist/diskclone
```

---

## 3. Step-by-Step Desktop Cloning Guide

### Step 1: Device Discovery
When DiskClone opens, click **Scan Devices** in the top right header. DiskClone queries `/sys/block` and `udev` to identify all attached SATA, NVMe, USB, and virtual drives.

> [!NOTE]
> System disks containing your active operating system (`/`, `/boot`, `/boot/efi`) will be tagged with a red `[SYSTEM DISK]` badge. Drives with active mounted filesystems will display a yellow `[MOUNTED]` badge.

### Step 2: Source Drive Selection
In the **1. Source Device (Read-Only)** panel:
- Click on the drive containing the data you wish to clone.
- DiskClone opens source devices strictly in **Read-Only** mode (`O_RDONLY`), guaranteeing your original drive is never modified or overwritten.

### Step 3: Destination Drive Selection
In the **2. Destination Device (Target)** panel:
- Select the target drive that will receive the cloned raw blocks.
- **Safety Filters**: DiskClone automatically disables ineligible targets:
  - You cannot select the same drive as both source and target.
  - You cannot target the host operating system disk.
  - You cannot target a drive with active mounted filesystems.
  - The destination drive capacity must be greater than or equal to the source drive capacity.

### Step 4: Safety & Compatibility Verification
Once both drives are selected, click **Initiate Clone Sequence**. DiskClone executes an automated 4-tier safety check:
1. Re-verifies `/proc/self/mountinfo` to prevent corrupting active filesystems.
2. Compares serial numbers, hardware models, and capacities.
3. Checks sector alignment (512e / 4Kn) to optimize write block sizes.

### Step 5: Explicit Text Confirmation
A modal warning dialog will appear:

> [!CAUTION]
> **CRITICAL WARNING**: ALL DATA ON DESTINATION DEVICE `/dev/sdX` WILL BE PERMANENTLY ERASED.

To unlock the **Erase & Clone** button, you must type the exact confirmation phrase into the text field:
```text
CLONE TO THIS DISK
```
*(Single checkboxes or accidental clicks are intentionally insufficient to prevent accidental data destruction).*

### Step 6: Live Progress & Throughput Monitoring
During cloning:
- **Transferred Bytes**: Shows exact current bytes written vs. total drive size.
- **Throughput Speed**: Real-time MB/s transfer rate with smoothed moving average.
- **ETA**: Remaining duration calculation updated dynamically.
- **Adaptive Chunking**: Automatically scales block chunks between 4MB and 16MB based on drive I/O latency.

### Step 7: Post-Clone Verification Pass
Upon reaching 100%, DiskClone initiates an online verification pass:
- Computes xxHash64 block hashes across the destination drive and verifies match against the source drive.
- Ensures partition tables (GPT / MBR) and bootloader metadata are byte-for-byte identical.
- Displays a green **✓ Clone Verification Passed** summary badge upon completion.

---

## 4. Handling Errors & Interrupted Jobs

### Unmounting Devices Before Cloning
If a target drive displays `[MOUNTED]`, open your terminal and unmount all active partitions before scanning again:
```bash
sudo umount /dev/sdX*
```

### Resuming Interrupted Clones
If power is disconnected or the app is stopped mid-clone:
1. DiskClone maintains a persistent write-ahead journal (`.diskclone-journal.json`).
2. Re-attach the exact same drive serials.
3. Launch DiskClone and select **Resume Interrupted Job**. DiskClone verifies the first-block hashes and resumes seamlessly from the last saved offset.

---

## 5. CLI / Headless Usage (Alternative)

For headless servers or automated scripts, DiskClone provides a CLI interface:

```bash
# Scan and list system drives
cargo run -- scan

# View CLI version
cargo run -- version

# Print CLI help
cargo run -- help
```
