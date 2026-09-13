//! Event emission and runtime event handlers for Tauri bridge.

use tauri::AppHandle;

/// Emits progress update event to the frontend UI.
#[allow(dead_code)]
pub fn emit_progress<R: tauri::Runtime>(
    _app: &AppHandle<R>,
    _event_name: &str,
    _payload: &impl serde::Serialize,
) -> Result<(), tauri::Error> {
    todo!("Phase 3: implement emit_progress")
}
