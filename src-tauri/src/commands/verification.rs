//! Post-clone verification IPC command.

use diskclone::storage::verification::VerificationReport;

/// Performs block-level verification between source and destination.
#[tauri::command]
pub async fn verify_clone(
    _source_path: String,
    _dest_path: String,
) -> Result<VerificationReport, String> {
    todo!("Phase 4: implement verify_clone IPC handler")
}
