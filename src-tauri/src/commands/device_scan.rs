//! Device scanning IPC command.

use diskclone::device::BlockDevice;

/// Scans the system for block devices suitable for source and destination cloning.
#[tauri::command]
pub async fn scan_devices() -> Result<Vec<BlockDevice>, String> {
    todo!("Phase 2: implement device scanning IPC handler")
}
