//! Clone operation IPC commands.

/// Starts a disk cloning workflow from source to destination.
#[tauri::command]
pub async fn start_clone(
    _source_path: String,
    _dest_path: String,
    _confirmation_token: String,
) -> Result<(), String> {
    todo!("Phase 3: implement start_clone IPC handler")
}

/// Pauses an ongoing cloning process.
#[tauri::command]
pub async fn pause_clone() -> Result<(), String> {
    todo!("Phase 3: implement pause_clone IPC handler")
}

/// Resumes a paused or interrupted clone operation.
#[tauri::command]
pub async fn resume_clone() -> Result<(), String> {
    todo!("Phase 3: implement resume_clone IPC handler")
}

/// Gracefully cancels the current cloning operation.
#[tauri::command]
pub async fn cancel_clone() -> Result<(), String> {
    todo!("Phase 3: implement cancel_clone IPC handler")
}
