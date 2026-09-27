//! IPC command handlers for the Phase 6 safety confirmation flow.
//!
//! These handlers encapsulate the pure confirmation logic and state-machine
//! transitions.  They are Tauri-agnostic so they can be unit-tested without
//! a running Tauri context; the `src-tauri` crate wraps them with
//! `#[tauri::command]` annotations.

use crate::confirm::{ConfirmationManager, ConfirmationRequest, ConfirmationResponse};
use crate::device::safety::SystemDiskDetector;
use crate::state::{CloneState, StateMachine};

/// Validates that the state machine is in `expected` and transitions to `next`.
/// Returns a descriptive error string on failure (for IPC responses).
fn require_state_and_advance(
    sm: &mut StateMachine,
    expected: CloneState,
    next: CloneState,
) -> std::result::Result<(), String> {
    if sm.current_state() != expected {
        return Err(format!(
            "Expected state {:?}, found {:?}",
            expected,
            sm.current_state()
        ));
    }
    sm.transition_to(next)
        .map_err(|e| format!("State transition error: {e}"))
}

// ── Handler: request visual confirmation ─────────────────────────────────────

/// Builds a `ConfirmationRequest` from source/dest device info and advances the
/// state machine from `VALIDATING` → `AWAITING_CONFIRMATION`.
///
/// In the Tauri integration layer this is wrapped by `#[tauri::command]`.
#[allow(clippy::too_many_arguments)]
pub fn handle_confirm_visual_check(
    source_model: String,
    source_capacity_gb: f64,
    source_serial: String,
    dest_model: String,
    dest_capacity_gb: f64,
    dest_serial: String,
    sm: &mut StateMachine,
    mgr: &mut ConfirmationManager,
) -> std::result::Result<ConfirmationRequest, String> {
    require_state_and_advance(
        sm,
        CloneState::VALIDATING,
        CloneState::AWAITING_CONFIRMATION,
    )?;

    let req = ConfirmationRequest {
        source_model,
        source_capacity_gb,
        source_serial,
        dest_model,
        dest_capacity_gb,
        dest_serial,
    };
    mgr.request = Some(req.clone());
    Ok(req)
}

// ── Handler: validate text confirmation ──────────────────────────────────────

/// Validates the exact typed phrase and advances the state machine
/// `AWAITING_CONFIRMATION` → `AWAITING_TEXT_CONFIRMATION` → `READY_TO_CLONE`.
///
/// We step through the two intermediate states to let the UI track progress
/// granularly; the transition from `AWAITING_TEXT_CONFIRMATION` to `READY_TO_CLONE`
/// is done atomically here once the phrase passes validation.
pub fn handle_confirm_text_input(
    text: String,
    sm: &mut StateMachine,
    mgr: &mut ConfirmationManager,
) -> std::result::Result<bool, String> {
    // Must be in AWAITING_CONFIRMATION (user clicked "I understand").
    require_state_and_advance(
        sm,
        CloneState::AWAITING_CONFIRMATION,
        CloneState::AWAITING_TEXT_CONFIRMATION,
    )?;

    // Validate phrase — step back to AWAITING_CONFIRMATION on failure so UI can retry.
    ConfirmationManager::validate_text_confirmation(&text).map_err(|e| {
        let _ = sm.transition_to(CloneState::AWAITING_CONFIRMATION);
        format!("Text validation failed: {e}")
    })?;

    // Phrase is correct → persist response and advance to READY_TO_CLONE.
    mgr.response = Some(ConfirmationResponse {
        visual_confirmed: true,
        text_confirmed: true,
        text_provided: text,
    });

    sm.transition_to(CloneState::READY_TO_CLONE)
        .map_err(|e| format!("State transition error: {e}"))?;

    Ok(true)
}

// ── Handler: final defense-in-depth safety check ──────────────────────────────

/// Re-verifies system disk status (defense-in-depth) and advances
/// `READY_TO_CLONE` → `PREPARING`.
///
/// Called immediately before handing off to the storage engine.
pub fn handle_final_safety_check(
    dest_path: &str,
    source_size: u64,
    dest_size: u64,
    sm: &mut StateMachine,
) -> std::result::Result<bool, String> {
    if sm.current_state() != CloneState::READY_TO_CLONE {
        return Err(format!(
            "Expected READY_TO_CLONE, found {:?}",
            sm.current_state()
        ));
    }

    // Defense-in-depth: re-check system disk.
    let is_system = SystemDiskDetector::is_system_disk(dest_path)
        .map_err(|e| format!("System disk check failed: {e}"))?;
    if is_system {
        sm.fail("Final safety check: destination is the system disk");
        return Err(
            "Destination appears to be the system disk. Clone blocked for safety.".to_string(),
        );
    }

    // Defense-in-depth: re-check capacity.
    if dest_size < source_size {
        sm.fail("Final safety check: insufficient destination capacity");
        return Err(format!(
            "Insufficient capacity. Source: {source_size} bytes, Dest: {dest_size} bytes"
        ));
    }

    sm.transition_to(CloneState::PREPARING)
        .map_err(|e| format!("State transition error: {e}"))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_sm_at(state: CloneState) -> StateMachine {
        let mut sm = StateMachine::new();
        sm.state = state;
        sm
    }

    // ── visual confirmation ───────────────────────────────────────────────

    #[test]
    fn test_visual_check_advances_state() {
        let mut sm = fresh_sm_at(CloneState::VALIDATING);
        let mut mgr = ConfirmationManager::new();

        let result = handle_confirm_visual_check(
            "Samsung EVO".into(),
            500.0,
            "SN1".into(),
            "WD Blue".into(),
            1000.0,
            "SN2".into(),
            &mut sm,
            &mut mgr,
        );

        assert!(result.is_ok());
        assert_eq!(sm.current_state(), CloneState::AWAITING_CONFIRMATION);
        assert_eq!(result.unwrap().source_model, "Samsung EVO");
    }

    #[test]
    fn test_visual_check_rejects_wrong_state() {
        let mut sm = fresh_sm_at(CloneState::IDLE);
        let mut mgr = ConfirmationManager::new();

        let result = handle_confirm_visual_check(
            "X".into(),
            0.0,
            "".into(),
            "Y".into(),
            0.0,
            "".into(),
            &mut sm,
            &mut mgr,
        );
        assert!(result.is_err());
        assert_eq!(sm.current_state(), CloneState::IDLE);
    }

    // ── text confirmation ─────────────────────────────────────────────────

    #[test]
    fn test_text_input_correct_phrase_advances_to_ready() {
        let mut sm = fresh_sm_at(CloneState::AWAITING_CONFIRMATION);
        let mut mgr = ConfirmationManager::new();

        let result = handle_confirm_text_input("CLONE TO THIS DISK".into(), &mut sm, &mut mgr);

        assert!(result.is_ok());
        assert_eq!(sm.current_state(), CloneState::READY_TO_CLONE);
        assert!(mgr.response.as_ref().unwrap().text_confirmed);
    }

    #[test]
    fn test_text_input_wrong_phrase_returns_error() {
        let mut sm = fresh_sm_at(CloneState::AWAITING_CONFIRMATION);
        let mut mgr = ConfirmationManager::new();

        let result = handle_confirm_text_input(
            "clone to this disk".into(), // wrong case
            &mut sm,
            &mut mgr,
        );

        assert!(result.is_err());
        // State rolled back to AWAITING_CONFIRMATION so user can retry.
        assert_eq!(sm.current_state(), CloneState::AWAITING_CONFIRMATION);
    }

    #[test]
    fn test_text_input_wrong_state_returns_error() {
        let mut sm = fresh_sm_at(CloneState::IDLE);
        let mut mgr = ConfirmationManager::new();

        let result = handle_confirm_text_input("CLONE TO THIS DISK".into(), &mut sm, &mut mgr);
        assert!(result.is_err());
    }

    // ── final safety check ────────────────────────────────────────────────

    #[test]
    fn test_final_safety_check_advances_to_preparing_on_safe_device() {
        let mut sm = fresh_sm_at(CloneState::READY_TO_CLONE);
        // /dev/null is definitely not the system disk.
        let result = handle_final_safety_check("/dev/null", 1000, 2000, &mut sm);
        assert!(result.is_ok());
        assert_eq!(sm.current_state(), CloneState::PREPARING);
    }

    #[test]
    fn test_final_safety_check_blocks_insufficient_capacity() {
        let mut sm = fresh_sm_at(CloneState::READY_TO_CLONE);
        let result = handle_final_safety_check("/dev/null", 2000, 1000, &mut sm);
        assert!(result.is_err());
        assert_eq!(sm.current_state(), CloneState::ERROR);
    }

    #[test]
    fn test_final_safety_check_wrong_state_returns_error() {
        let mut sm = fresh_sm_at(CloneState::IDLE);
        let result = handle_final_safety_check("/dev/null", 1000, 2000, &mut sm);
        assert!(result.is_err());
    }

    // ── full confirmation flow ────────────────────────────────────────────

    #[test]
    fn test_full_confirmation_flow() {
        let mut sm = fresh_sm_at(CloneState::VALIDATING);
        let mut mgr = ConfirmationManager::new();

        // Step 1: visual check
        handle_confirm_visual_check(
            "Src".into(),
            500.0,
            "S1".into(),
            "Dst".into(),
            1000.0,
            "D1".into(),
            &mut sm,
            &mut mgr,
        )
        .unwrap();
        assert_eq!(sm.current_state(), CloneState::AWAITING_CONFIRMATION);

        // Step 2: text confirmation
        handle_confirm_text_input("CLONE TO THIS DISK".into(), &mut sm, &mut mgr).unwrap();
        assert_eq!(sm.current_state(), CloneState::READY_TO_CLONE);

        // Step 3: final check
        handle_final_safety_check("/dev/null", 500, 1000, &mut sm).unwrap();
        assert_eq!(sm.current_state(), CloneState::PREPARING);
    }
}
