//! Structured log retrieval IPC command.

use diskclone::logging::format::LogEntry;

/// Retrieves the most recent in-memory log entries.
#[tauri::command]
pub async fn get_logs() -> Result<Vec<LogEntry>, String> {
    todo!("Phase 2: implement get_logs IPC handler")
}
