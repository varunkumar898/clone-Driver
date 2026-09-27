//! UI state management for the Tauri desktop application.
//!
//! Provides thread-safe, reactive state tracking for:
//! - Multi-screen navigation (Selection, Confirmation, Progress, Recovery, Error)
//! - Multi-step confirmation flow and token validation
//! - Live clone progress and verification telemetry
//! - Interrupted clone recovery detection

use crate::device::BlockDevice;
use crate::resume::RecoveryPlan;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Active view screen in the Tauri user interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UiScreen {
    /// Screen 1: Scan and select source and destination disks.
    #[default]
    DiskSelection,
    /// Screen 2A: Visual inspection of source vs destination.
    VisualConfirmation,
    /// Screen 2B: Strict manual text entry safety gate.
    TextConfirmation,
    /// Screen 2C: Final irreversible warning and authorization.
    FinalSafetyCheck,
    /// Screen 3: Live cloning progress, transfer speed, and telemetry.
    Cloning,
    /// Clone operation paused by user or system.
    Paused,
    /// Screen 4: Post-clone hash verification (xxHash64 / SHA-256).
    Verification,
    /// Operation successfully completed.
    Completed,
    /// Fatal or recoverable error screen.
    Error,
    /// Interrupted clone recovery prompt screen.
    Recovery,
}

/// Progress across the multi-step safety confirmation flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConfirmationStep {
    /// Initial visual review of device models, capacities, and serials.
    #[default]
    Visual,
    /// Explicit typing of confirmation phrase ("CLONE TO THIS DISK").
    TextInput,
    /// Final irrevocable commitment check.
    FinalCheck,
    /// All confirmation steps cleared; ready to invoke clone engine.
    Approved,
}

/// Real-time cloning progress statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveProgress {
    /// Bytes copied so far.
    pub bytes_copied: u64,
    /// Total source size in bytes.
    pub total_bytes: u64,
    /// Percentage completed [0.0, 100.0].
    pub percent: f64,
    /// Current transfer speed in Megabytes per second (MB/s).
    pub speed_mbps: f64,
    /// Estimated time remaining in seconds.
    pub eta_secs: u64,
    /// Current block chunk index.
    pub current_chunk: u64,
    /// Total chunks to copy.
    pub total_chunks: u64,
}

impl Default for LiveProgress {
    fn default() -> Self {
        Self {
            bytes_copied: 0,
            total_bytes: 0,
            percent: 0.0,
            speed_mbps: 0.0,
            eta_secs: 0,
            current_chunk: 0,
            total_chunks: 0,
        }
    }
}

/// Comprehensive UI error payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiError {
    /// Short error title or category.
    pub title: String,
    /// Detailed diagnostic message.
    pub message: String,
    /// Whether user can retry or dismiss.
    pub is_recoverable: bool,
    /// Suggested user remediation action.
    pub suggestion: Option<String>,
}

/// Recovery state information when an interrupted clone checkpoint is detected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RecoveryInfo {
    /// Whether an interrupted checkpoint is available.
    pub available: bool,
    /// Path to the detected journal file.
    pub journal_path: String,
    /// Recovery plan details if validated.
    pub plan: Option<RecoveryPlan>,
    /// Informational message for the user.
    pub message: String,
}

/// Complete UI snapshot synchronized across IPC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiState {
    /// Currently displayed screen.
    pub current_screen: UiScreen,
    /// Selected source block device (read-only).
    pub source_device: Option<BlockDevice>,
    /// Selected destination block device (target to overwrite).
    pub dest_device: Option<BlockDevice>,
    /// Current stage of confirmation.
    pub confirmation_step: ConfirmationStep,
    /// Real-time text typed into the confirmation gate.
    pub confirmation_input: String,
    /// Whether the typed confirmation matches "CLONE TO THIS DISK".
    pub is_token_valid: bool,
    /// Live progress metrics during cloning.
    pub progress: LiveProgress,
    /// Active error details if any.
    pub error: Option<UiError>,
    /// Recovery information if an interrupted clone was found.
    pub recovery: RecoveryInfo,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            current_screen: UiScreen::DiskSelection,
            source_device: None,
            dest_device: None,
            confirmation_step: ConfirmationStep::Visual,
            confirmation_input: String::new(),
            is_token_valid: false,
            progress: LiveProgress::default(),
            error: None,
            recovery: RecoveryInfo::default(),
        }
    }
}

/// Thread-safe manager for desktop UI state.
#[derive(Debug, Clone)]
pub struct UiStateManager {
    state: Arc<Mutex<UiState>>,
}

impl Default for UiStateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UiStateManager {
    /// The mandatory confirmation phrase required before cloning.
    pub const CONFIRMATION_PHRASE: &'static str = "CLONE TO THIS DISK";

    /// Creates a new manager with default idle state.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(UiState::default())),
        }
    }

    /// Obtains a clone of the current UI state snapshot.
    pub fn get_state(&self) -> UiState {
        self.state.lock().expect("UiState lock poisoned").clone()
    }

    /// Sets the active UI screen.
    pub fn set_screen(&self, screen: UiScreen) {
        let mut state = self.state.lock().expect("UiState lock poisoned");
        state.current_screen = screen;
    }

    /// Selects source and destination devices with initial validation.
    pub fn select_devices(
        &self,
        source: BlockDevice,
        destination: BlockDevice,
    ) -> Result<(), String> {
        if source.path == destination.path {
            return Err("Source and destination cannot be the same device".to_string());
        }

        if destination.is_system_disk {
            return Err("Destination disk contains the host running operating system".to_string());
        }

        if destination.is_mounted {
            return Err("Destination disk has active mounted partitions".to_string());
        }

        if destination.size_bytes < source.size_bytes {
            return Err(format!(
                "Destination capacity ({} B) is smaller than source ({} B)",
                destination.size_bytes, source.size_bytes
            ));
        }

        let mut state = self.state.lock().expect("UiState lock poisoned");
        state.source_device = Some(source);
        state.dest_device = Some(destination);
        state.confirmation_step = ConfirmationStep::Visual;
        state.confirmation_input.clear();
        state.is_token_valid = false;
        Ok(())
    }

    /// Advances confirmation step after safety checks pass.
    pub fn set_confirmation_step(&self, step: ConfirmationStep) {
        let mut state = self.state.lock().expect("UiState lock poisoned");
        state.confirmation_step = step;
        match step {
            ConfirmationStep::Visual => state.current_screen = UiScreen::VisualConfirmation,
            ConfirmationStep::TextInput => state.current_screen = UiScreen::TextConfirmation,
            ConfirmationStep::FinalCheck => state.current_screen = UiScreen::FinalSafetyCheck,
            ConfirmationStep::Approved => {}
        }
    }

    /// Updates typed confirmation input and checks validity in real time.
    pub fn update_confirmation_input(&self, input: &str) -> bool {
        let mut state = self.state.lock().expect("UiState lock poisoned");
        state.confirmation_input = input.to_string();
        state.is_token_valid = input == Self::CONFIRMATION_PHRASE;
        state.is_token_valid
    }

    /// Updates live progress telemetry.
    pub fn update_progress(&self, progress: LiveProgress) {
        let mut state = self.state.lock().expect("UiState lock poisoned");
        state.progress = progress;
    }

    /// Sets error state and switches screen to `UiScreen::Error`.
    pub fn set_error(&self, error: UiError) {
        let mut state = self.state.lock().expect("UiState lock poisoned");
        state.error = Some(error);
        state.current_screen = UiScreen::Error;
    }

    /// Clears any active error and restores the previous screen.
    pub fn clear_error(&self, return_screen: UiScreen) {
        let mut state = self.state.lock().expect("UiState lock poisoned");
        state.error = None;
        state.current_screen = return_screen;
    }

    /// Sets recovery information and switches to `UiScreen::Recovery`.
    pub fn set_recovery(&self, recovery: RecoveryInfo) {
        let mut state = self.state.lock().expect("UiState lock poisoned");
        state.recovery = recovery;
        state.current_screen = UiScreen::Recovery;
    }

    /// Resets the UI state back to initial clean selection state.
    pub fn reset(&self) {
        let mut state = self.state.lock().expect("UiState lock poisoned");
        *state = UiState::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::properties::DeviceProperties;
    use std::path::PathBuf;

    fn make_device(name: &str, size_bytes: u64, is_system: bool, is_mounted: bool) -> BlockDevice {
        BlockDevice {
            path: PathBuf::from(format!("/dev/{name}")),
            sysfs_name: name.to_string(),
            size_bytes,
            properties: DeviceProperties {
                serial_number: format!("SN_{name}"),
                model: format!("Model_{name}"),
                ..Default::default()
            },
            is_system_disk: is_system,
            is_mounted,
            partition_table: None,
        }
    }

    #[test]
    fn test_ui_initial_state() {
        let mgr = UiStateManager::new();
        let state = mgr.get_state();
        assert_eq!(state.current_screen, UiScreen::DiskSelection);
        assert_eq!(state.confirmation_step, ConfirmationStep::Visual);
        assert!(state.source_device.is_none());
        assert!(state.dest_device.is_none());
        assert!(!state.is_token_valid);
    }

    #[test]
    fn test_ui_set_screen() {
        let mgr = UiStateManager::new();
        mgr.set_screen(UiScreen::Cloning);
        assert_eq!(mgr.get_state().current_screen, UiScreen::Cloning);
    }

    #[test]
    fn test_ui_select_devices_valid() {
        let mgr = UiStateManager::new();
        let src = make_device("sda", 1_000_000_000, false, false);
        let dst = make_device("sdb", 2_000_000_000, false, false);

        assert!(mgr.select_devices(src, dst).is_ok());
        let state = mgr.get_state();
        assert_eq!(state.source_device.unwrap().sysfs_name, "sda");
        assert_eq!(state.dest_device.unwrap().sysfs_name, "sdb");
    }

    #[test]
    fn test_ui_select_devices_reject_same() {
        let mgr = UiStateManager::new();
        let src = make_device("sda", 1_000_000_000, false, false);
        let dst = make_device("sda", 1_000_000_000, false, false);

        let res = mgr.select_devices(src, dst);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("same device"));
    }

    #[test]
    fn test_ui_select_devices_reject_system_disk() {
        let mgr = UiStateManager::new();
        let src = make_device("sda", 1_000_000_000, false, false);
        let dst = make_device("sdb", 2_000_000_000, true, false);

        let res = mgr.select_devices(src, dst);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("operating system"));
    }

    #[test]
    fn test_ui_select_devices_reject_mounted() {
        let mgr = UiStateManager::new();
        let src = make_device("sda", 1_000_000_000, false, false);
        let dst = make_device("sdb", 2_000_000_000, false, true);

        let res = mgr.select_devices(src, dst);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("mounted"));
    }

    #[test]
    fn test_ui_select_devices_reject_too_small() {
        let mgr = UiStateManager::new();
        let src = make_device("sda", 2_000_000_000, false, false);
        let dst = make_device("sdb", 1_000_000_000, false, false);

        let res = mgr.select_devices(src, dst);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("smaller than source"));
    }

    #[test]
    fn test_ui_token_validation_exact_match() {
        let mgr = UiStateManager::new();
        assert!(!mgr.update_confirmation_input("clone to this disk")); // lower case fails
        assert!(!mgr.update_confirmation_input("CLONE TO THIS DISK ")); // extra space fails
        assert!(mgr.update_confirmation_input("CLONE TO THIS DISK")); // exact match passes
        assert!(mgr.get_state().is_token_valid);
    }

    #[test]
    fn test_ui_confirmation_step_progression() {
        let mgr = UiStateManager::new();
        mgr.set_confirmation_step(ConfirmationStep::TextInput);
        assert_eq!(mgr.get_state().current_screen, UiScreen::TextConfirmation);

        mgr.set_confirmation_step(ConfirmationStep::FinalCheck);
        assert_eq!(mgr.get_state().current_screen, UiScreen::FinalSafetyCheck);
    }

    #[test]
    fn test_ui_progress_updates() {
        let mgr = UiStateManager::new();
        let progress = LiveProgress {
            bytes_copied: 500_000_000,
            total_bytes: 1_000_000_000,
            percent: 50.0,
            speed_mbps: 125.5,
            eta_secs: 4,
            current_chunk: 125,
            total_chunks: 250,
        };
        mgr.update_progress(progress.clone());
        assert_eq!(mgr.get_state().progress, progress);
    }

    #[test]
    fn test_ui_error_handling() {
        let mgr = UiStateManager::new();
        let err = UiError {
            title: "Write Error".to_string(),
            message: "I/O error at sector 4096".to_string(),
            is_recoverable: false,
            suggestion: Some("Check cable connection".to_string()),
        };
        mgr.set_error(err.clone());
        let state = mgr.get_state();
        assert_eq!(state.current_screen, UiScreen::Error);
        assert_eq!(state.error, Some(err));

        mgr.clear_error(UiScreen::DiskSelection);
        let cleared = mgr.get_state();
        assert_eq!(cleared.current_screen, UiScreen::DiskSelection);
        assert!(cleared.error.is_none());
    }

    #[test]
    fn test_ui_recovery_setting() {
        let mgr = UiStateManager::new();
        let rec = RecoveryInfo {
            available: true,
            journal_path: "/tmp/resume.json".to_string(),
            plan: None,
            message: "Interrupted clone found".to_string(),
        };
        mgr.set_recovery(rec.clone());
        let state = mgr.get_state();
        assert_eq!(state.current_screen, UiScreen::Recovery);
        assert_eq!(state.recovery, rec);
    }

    #[test]
    fn test_ui_reset() {
        let mgr = UiStateManager::new();
        mgr.set_screen(UiScreen::Cloning);
        mgr.reset();
        assert_eq!(mgr.get_state().current_screen, UiScreen::DiskSelection);
    }

    #[test]
    fn test_ui_concurrent_access() {
        let mgr = UiStateManager::new();
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let m = mgr.clone();
                std::thread::spawn(move || {
                    m.set_screen(if i % 2 == 0 {
                        UiScreen::Cloning
                    } else {
                        UiScreen::DiskSelection
                    });
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
        let screen = mgr.get_state().current_screen;
        assert!(screen == UiScreen::Cloning || screen == UiScreen::DiskSelection);
    }

    #[test]
    fn test_ui_live_progress_default() {
        let prog = LiveProgress::default();
        assert_eq!(prog.bytes_copied, 0);
        assert_eq!(prog.percent, 0.0);
    }
}
