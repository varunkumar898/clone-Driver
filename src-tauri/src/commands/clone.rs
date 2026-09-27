//! Clone operation IPC commands.

use diskclone::ui::handlers;
use diskclone::ui::state::{UiState, UiStateManager};
use std::path::Path;
use std::sync::OnceLock;

static UI_MANAGER: OnceLock<UiStateManager> = OnceLock::new();

pub fn get_ui_manager() -> &'static UiStateManager {
    UI_MANAGER.get_or_init(UiStateManager::new)
}

/// Starts a disk cloning workflow from source to destination.
#[tauri::command]
pub async fn start_clone(
    _source_path: String,
    _dest_path: String,
    _confirmation_token: String,
) -> Result<(), String> {
    let mgr = get_ui_manager();
    handlers::handle_start_clone(mgr).map(|_| ())
}

/// Pauses an ongoing cloning process.
#[tauri::command]
pub async fn pause_clone() -> Result<(), String> {
    let mgr = get_ui_manager();
    handlers::handle_pause_clone(mgr).map(|_| ())
}

/// Resumes a paused or interrupted clone operation.
#[tauri::command]
pub async fn resume_clone() -> Result<(), String> {
    let mgr = get_ui_manager();
    handlers::handle_resume_clone(mgr).map(|_| ())
}

/// Gracefully cancels the current cloning operation.
#[tauri::command]
pub async fn cancel_clone() -> Result<(), String> {
    let mgr = get_ui_manager();
    handlers::handle_cancel_clone(mgr).map(|_| ())
}

/// IPC command: Visual inspection confirmation.
#[tauri::command]
pub async fn confirm_visual_check() -> Result<UiState, String> {
    let mgr = get_ui_manager();
    handlers::handle_confirm_visual(mgr)
}

/// IPC command: Text phrase confirmation.
#[tauri::command]
pub async fn confirm_text_input(typed_token: String) -> Result<UiState, String> {
    let mgr = get_ui_manager();
    handlers::handle_confirm_text(mgr, &typed_token)
}

/// IPC command: Final irreversible safety check.
#[tauri::command]
pub async fn final_safety_check(acknowledged: bool) -> Result<UiState, String> {
    let mgr = get_ui_manager();
    handlers::handle_confirm_final(mgr, acknowledged)
}

/// IPC command: Check recovery checkpoint on disk.
#[tauri::command]
pub async fn check_recovery_checkpoint(
    journal_path: String,
) -> Result<diskclone::ui::RecoveryInfo, String> {
    let mgr = get_ui_manager();
    handlers::handle_check_recovery(mgr, Path::new(&journal_path))
}
