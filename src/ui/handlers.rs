//! Pure IPC command handlers for Tauri frontend bridge.
//!
//! Provides business logic and state machine validation for frontend interactions:
//! - Device selection and safety verification
//! - Sequential multi-stage confirmation gates
//! - Clone lifecycle operations (start, pause, resume, cancel)
//! - Recovery checkpoint detection and application

use super::state::{
    ConfirmationStep, LiveProgress, RecoveryInfo, UiScreen, UiState, UiStateManager,
};

use crate::device::BlockDevice;
use crate::resume::{RecoveryCoordinator, ResumeJournal};
use std::path::Path;

/// Handler to retrieve the current UI state snapshot.
pub fn handle_get_state(manager: &UiStateManager) -> UiState {
    manager.get_state()
}

/// Handler to select source and destination devices.
pub fn handle_select_devices(
    manager: &UiStateManager,
    source: BlockDevice,
    destination: BlockDevice,
) -> Result<UiState, String> {
    manager.select_devices(source, destination)?;
    Ok(manager.get_state())
}

/// Handler to begin the multi-step confirmation flow from selection screen.
pub fn handle_start_confirmation(manager: &UiStateManager) -> Result<UiState, String> {
    let state = manager.get_state();
    if state.source_device.is_none() || state.dest_device.is_none() {
        return Err("Both source and destination devices must be selected".to_string());
    }

    manager.set_confirmation_step(ConfirmationStep::Visual);
    Ok(manager.get_state())
}

/// Handler for approving Step 1 (Visual Confirmation).
pub fn handle_confirm_visual(manager: &UiStateManager) -> Result<UiState, String> {
    let state = manager.get_state();
    if state.confirmation_step != ConfirmationStep::Visual {
        return Err(format!(
            "Invalid confirmation step: expected Visual, found {:?}",
            state.confirmation_step
        ));
    }

    manager.set_confirmation_step(ConfirmationStep::TextInput);
    Ok(manager.get_state())
}

/// Handler for validating and approving Step 2 (Text Confirmation).
pub fn handle_confirm_text(
    manager: &UiStateManager,
    typed_phrase: &str,
) -> Result<UiState, String> {
    let state = manager.get_state();
    if state.confirmation_step != ConfirmationStep::TextInput {
        return Err(format!(
            "Invalid confirmation step: expected TextInput, found {:?}",
            state.confirmation_step
        ));
    }

    let is_valid = manager.update_confirmation_input(typed_phrase);
    if !is_valid {
        return Err(format!(
            "Typed phrase did not match required token '{}'",
            UiStateManager::CONFIRMATION_PHRASE
        ));
    }

    manager.set_confirmation_step(ConfirmationStep::FinalCheck);
    Ok(manager.get_state())
}

/// Handler for executing Step 3 (Final Irrevocable Check).
pub fn handle_confirm_final(
    manager: &UiStateManager,
    acknowledgement_checked: bool,
) -> Result<UiState, String> {
    let state = manager.get_state();
    if state.confirmation_step != ConfirmationStep::FinalCheck {
        return Err(format!(
            "Invalid confirmation step: expected FinalCheck, found {:?}",
            state.confirmation_step
        ));
    }

    if !acknowledgement_checked {
        return Err("User must explicitly check the destruction acknowledgement".to_string());
    }

    manager.set_confirmation_step(ConfirmationStep::Approved);
    Ok(manager.get_state())
}

/// Handler to initiate the clone operation after confirmation.
pub fn handle_start_clone(manager: &UiStateManager) -> Result<UiState, String> {
    let state = manager.get_state();
    if state.confirmation_step != ConfirmationStep::Approved {
        return Err("Cannot start clone: confirmation flow is not approved".to_string());
    }

    manager.set_screen(UiScreen::Cloning);
    manager.update_progress(LiveProgress::default());
    Ok(manager.get_state())
}

/// Handler to pause an ongoing clone.
pub fn handle_pause_clone(manager: &UiStateManager) -> Result<UiState, String> {
    let state = manager.get_state();
    if state.current_screen != UiScreen::Cloning {
        return Err(format!(
            "Cannot pause: not in Cloning state (current: {:?})",
            state.current_screen
        ));
    }

    manager.set_screen(UiScreen::Paused);
    Ok(manager.get_state())
}

/// Handler to resume a paused clone.
pub fn handle_resume_clone(manager: &UiStateManager) -> Result<UiState, String> {
    let state = manager.get_state();
    if state.current_screen != UiScreen::Paused {
        return Err(format!(
            "Cannot resume: not in Paused state (current: {:?})",
            state.current_screen
        ));
    }

    manager.set_screen(UiScreen::Cloning);
    Ok(manager.get_state())
}

/// Handler to cancel a running or paused clone.
pub fn handle_cancel_clone(manager: &UiStateManager) -> Result<UiState, String> {
    let state = manager.get_state();
    if state.current_screen != UiScreen::Cloning && state.current_screen != UiScreen::Paused {
        return Err(format!(
            "Cannot cancel: clone is not active (current: {:?})",
            state.current_screen
        ));
    }

    manager.reset();
    Ok(manager.get_state())
}

/// Handler to check for interrupted clone recovery checkpoint on disk.
pub fn handle_check_recovery(
    manager: &UiStateManager,
    journal_path: &Path,
) -> Result<RecoveryInfo, String> {
    if !journal_path.exists() {
        let info = RecoveryInfo {
            available: false,
            journal_path: journal_path.display().to_string(),
            plan: None,
            message: "No existing clone journal found".to_string(),
        };
        return Ok(info);
    }

    let record =
        ResumeJournal::load(journal_path).map_err(|e| format!("Failed to read journal: {e}"))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if RecoveryCoordinator::is_checkpoint_expired(
        &record,
        crate::resume::MAX_CHECKPOINT_AGE_SECS,
        Some(now),
    ) {
        let info = RecoveryInfo {
            available: false,
            journal_path: journal_path.display().to_string(),
            plan: None,
            message: "Existing checkpoint has expired (> 24 hours)".to_string(),
        };
        return Ok(info);
    }

    let plan = crate::resume::RecoveryPlan {
        source_serial: record.source_serial,
        dest_serial: record.dest_serial,
        total_bytes: record.source_size_bytes,
        resumed_offset: record.last_completed_offset,
        bytes_remaining: record
            .source_size_bytes
            .saturating_sub(record.last_completed_offset),
        percent_complete: if record.source_size_bytes == 0 {
            0.0
        } else {
            (record.last_completed_offset as f64 / record.source_size_bytes as f64 * 100.0)
                .clamp(0.0, 100.0)
        },
        chunk_size: record.chunk_size,
        checkpoint_age_secs: now.saturating_sub(record.timestamp_epoch_secs),
        first_block_xxhash: record.first_block_xxhash,
    };

    let info = RecoveryInfo {
        available: true,
        journal_path: journal_path.display().to_string(),
        plan: Some(plan),
        message: "Interrupted clone checkpoint available for resumption".to_string(),
    };

    manager.set_recovery(info.clone());
    Ok(info)
}

/// Handler to accept and apply a recovery plan, moving directly to cloning state.
pub fn handle_apply_recovery(
    manager: &UiStateManager,
    plan: crate::resume::RecoveryPlan,
) -> Result<UiState, String> {
    manager.set_screen(UiScreen::Cloning);
    let progress = LiveProgress {
        bytes_copied: plan.resumed_offset,
        total_bytes: plan.total_bytes,
        percent: plan.percent_complete,
        speed_mbps: 0.0,
        eta_secs: 0,
        current_chunk: plan.chunks_completed(),
        total_chunks: plan.chunks_completed() + plan.chunks_remaining(),
    };
    manager.update_progress(progress);
    Ok(manager.get_state())
}

/// Handler to dismiss an active error and return to a safe screen.
pub fn handle_dismiss_error(manager: &UiStateManager, fallback_screen: UiScreen) -> UiState {
    manager.clear_error(fallback_screen);
    manager.get_state()
}

/// Handler to reset the entire UI session to the beginning.
pub fn handle_reset_flow(manager: &UiStateManager) -> UiState {
    manager.reset();
    manager.get_state()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::state::UiError;

    use crate::device::properties::DeviceProperties;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn mock_device(name: &str, size_bytes: u64) -> BlockDevice {
        BlockDevice {
            path: PathBuf::from(format!("/dev/{name}")),
            sysfs_name: name.to_string(),
            size_bytes,
            properties: DeviceProperties {
                serial_number: format!("SN_{name}"),
                model: format!("Model_{name}"),
                ..Default::default()
            },
            is_system_disk: false,
            is_mounted: false,
            partition_table: None,
        }
    }

    #[test]
    fn test_handle_get_state() {
        let mgr = UiStateManager::new();
        let state = handle_get_state(&mgr);
        assert_eq!(state.current_screen, UiScreen::DiskSelection);
    }

    #[test]
    fn test_handle_select_devices_success() {
        let mgr = UiStateManager::new();
        let s = mock_device("sda", 1_000_000);
        let d = mock_device("sdb", 2_000_000);

        let state = handle_select_devices(&mgr, s, d).unwrap();
        assert!(state.source_device.is_some());
        assert!(state.dest_device.is_some());
    }

    #[test]
    fn test_handle_start_confirmation_requires_both_devices() {
        let mgr = UiStateManager::new();
        assert!(handle_start_confirmation(&mgr).is_err());
    }

    #[test]
    fn test_handle_start_confirmation_success() {
        let mgr = UiStateManager::new();
        mgr.select_devices(mock_device("sda", 100), mock_device("sdb", 200))
            .unwrap();
        let state = handle_start_confirmation(&mgr).unwrap();
        assert_eq!(state.current_screen, UiScreen::VisualConfirmation);
        assert_eq!(state.confirmation_step, ConfirmationStep::Visual);
    }

    #[test]
    fn test_handle_confirm_visual_advances_to_text() {
        let mgr = UiStateManager::new();
        mgr.select_devices(mock_device("sda", 100), mock_device("sdb", 200))
            .unwrap();
        handle_start_confirmation(&mgr).unwrap();

        let state = handle_confirm_visual(&mgr).unwrap();
        assert_eq!(state.current_screen, UiScreen::TextConfirmation);
        assert_eq!(state.confirmation_step, ConfirmationStep::TextInput);
    }

    #[test]
    fn test_handle_confirm_text_rejects_wrong_input() {
        let mgr = UiStateManager::new();
        mgr.select_devices(mock_device("sda", 100), mock_device("sdb", 200))
            .unwrap();
        handle_start_confirmation(&mgr).unwrap();
        handle_confirm_visual(&mgr).unwrap();

        let res = handle_confirm_text(&mgr, "wrong token");
        assert!(res.is_err());
    }

    #[test]
    fn test_handle_confirm_text_accepts_exact_input() {
        let mgr = UiStateManager::new();
        mgr.select_devices(mock_device("sda", 100), mock_device("sdb", 200))
            .unwrap();
        handle_start_confirmation(&mgr).unwrap();
        handle_confirm_visual(&mgr).unwrap();

        let state = handle_confirm_text(&mgr, UiStateManager::CONFIRMATION_PHRASE).unwrap();
        assert_eq!(state.current_screen, UiScreen::FinalSafetyCheck);
        assert_eq!(state.confirmation_step, ConfirmationStep::FinalCheck);
    }

    #[test]
    fn test_handle_confirm_final_requires_checkbox() {
        let mgr = UiStateManager::new();
        mgr.select_devices(mock_device("sda", 100), mock_device("sdb", 200))
            .unwrap();
        handle_start_confirmation(&mgr).unwrap();
        handle_confirm_visual(&mgr).unwrap();
        handle_confirm_text(&mgr, UiStateManager::CONFIRMATION_PHRASE).unwrap();

        assert!(handle_confirm_final(&mgr, false).is_err());
    }

    #[test]
    fn test_handle_confirm_final_approves() {
        let mgr = UiStateManager::new();
        mgr.select_devices(mock_device("sda", 100), mock_device("sdb", 200))
            .unwrap();
        handle_start_confirmation(&mgr).unwrap();
        handle_confirm_visual(&mgr).unwrap();
        handle_confirm_text(&mgr, UiStateManager::CONFIRMATION_PHRASE).unwrap();

        let state = handle_confirm_final(&mgr, true).unwrap();
        assert_eq!(state.confirmation_step, ConfirmationStep::Approved);
    }

    #[test]
    fn test_handle_start_clone_requires_approval() {
        let mgr = UiStateManager::new();
        assert!(handle_start_clone(&mgr).is_err());
    }

    #[test]
    fn test_handle_start_clone_after_approval() {
        let mgr = UiStateManager::new();
        mgr.select_devices(mock_device("sda", 100), mock_device("sdb", 200))
            .unwrap();
        handle_start_confirmation(&mgr).unwrap();
        handle_confirm_visual(&mgr).unwrap();
        handle_confirm_text(&mgr, UiStateManager::CONFIRMATION_PHRASE).unwrap();
        handle_confirm_final(&mgr, true).unwrap();

        let state = handle_start_clone(&mgr).unwrap();
        assert_eq!(state.current_screen, UiScreen::Cloning);
    }

    #[test]
    fn test_handle_pause_and_resume_clone() {
        let mgr = UiStateManager::new();
        mgr.set_confirmation_step(ConfirmationStep::Approved);
        handle_start_clone(&mgr).unwrap();

        let paused = handle_pause_clone(&mgr).unwrap();
        assert_eq!(paused.current_screen, UiScreen::Paused);

        let resumed = handle_resume_clone(&mgr).unwrap();
        assert_eq!(resumed.current_screen, UiScreen::Cloning);
    }

    #[test]
    fn test_handle_cancel_clone() {
        let mgr = UiStateManager::new();
        mgr.set_confirmation_step(ConfirmationStep::Approved);
        handle_start_clone(&mgr).unwrap();

        let cancelled = handle_cancel_clone(&mgr).unwrap();
        assert_eq!(cancelled.current_screen, UiScreen::DiskSelection);
    }

    #[test]
    fn test_handle_check_recovery_missing_journal() {
        let mgr = UiStateManager::new();
        let res = handle_check_recovery(&mgr, Path::new("/path/that/does/not/exist.json")).unwrap();
        assert!(!res.available);
    }

    #[test]
    fn test_handle_check_recovery_valid_journal() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("resume.json");
        let rec = crate::resume::ResumeRecord::new("S1", "D1", 1000, 100, 0);
        ResumeJournal::save(&path, &rec).unwrap();

        let mgr = UiStateManager::new();
        let res = handle_check_recovery(&mgr, &path).unwrap();
        assert!(res.available);
        assert!(res.plan.is_some());
    }

    #[test]
    fn test_handle_dismiss_error() {
        let mgr = UiStateManager::new();
        mgr.set_error(UiError {
            title: "Err".into(),
            message: "msg".into(),
            is_recoverable: true,
            suggestion: None,
        });
        let state = handle_dismiss_error(&mgr, UiScreen::DiskSelection);
        assert_eq!(state.current_screen, UiScreen::DiskSelection);
        assert!(state.error.is_none());
    }

    #[test]
    fn test_handle_reset_flow() {
        let mgr = UiStateManager::new();
        mgr.set_screen(UiScreen::Cloning);
        let state = handle_reset_flow(&mgr);
        assert_eq!(state.current_screen, UiScreen::DiskSelection);
    }
}
