//! Unit tests for Phase 6 confirmation state machine transitions (Layer 3).
//!
//! These tests exercise the `src/state/mod.rs` `StateMachine` transitions
//! specific to the confirmation flow:
//!   VALIDATING → AWAITING_CONFIRMATION → AWAITING_TEXT_CONFIRMATION
//!   → READY_TO_CLONE → PREPARING

use diskclone::state::{CloneState, StateMachine};

// ── Helper ────────────────────────────────────────────────────────────────────

fn sm_at(state: CloneState) -> StateMachine {
    let mut sm = StateMachine::new();
    sm.state = state;
    sm
}

// ── Individual confirmation transitions ──────────────────────────────────────

#[test]
fn test_validating_to_awaiting_confirmation() {
    let mut sm = sm_at(CloneState::VALIDATING);
    assert!(sm.transition_to_awaiting_confirmation().is_ok());
    assert_eq!(sm.current_state(), CloneState::AWAITING_CONFIRMATION);
}

#[test]
fn test_awaiting_confirmation_to_text_confirmation() {
    let mut sm = sm_at(CloneState::AWAITING_CONFIRMATION);
    assert!(sm.transition_to_awaiting_text_confirmation().is_ok());
    assert_eq!(sm.current_state(), CloneState::AWAITING_TEXT_CONFIRMATION);
}

#[test]
fn test_text_confirmation_to_ready_to_clone() {
    let mut sm = sm_at(CloneState::AWAITING_TEXT_CONFIRMATION);
    assert!(sm.transition_to_ready_to_clone().is_ok());
    assert_eq!(sm.current_state(), CloneState::READY_TO_CLONE);
}

#[test]
fn test_ready_to_clone_to_preparing() {
    let mut sm = sm_at(CloneState::READY_TO_CLONE);
    assert!(sm.transition_to_preparing().is_ok());
    assert_eq!(sm.current_state(), CloneState::PREPARING);
}

// ── Full confirmation sequence ────────────────────────────────────────────────

#[test]
fn test_full_confirmation_sequence() {
    let mut sm = StateMachine::new();

    // Walk to VALIDATING via the normal path.
    sm.transition_to(CloneState::AWAITING_SOURCE).unwrap();
    sm.transition_to(CloneState::AWAITING_DESTINATION).unwrap();
    sm.transition_to(CloneState::VALIDATING).unwrap();

    // Confirmation gate.
    sm.transition_to_awaiting_confirmation().unwrap();
    sm.transition_to_awaiting_text_confirmation().unwrap();
    sm.transition_to_ready_to_clone().unwrap();
    sm.transition_to_preparing().unwrap();

    assert_eq!(sm.current_state(), CloneState::PREPARING);
}

// ── Invalid state rejections ──────────────────────────────────────────────────

#[test]
fn test_cannot_go_to_awaiting_confirmation_from_idle() {
    let mut sm = sm_at(CloneState::IDLE);
    assert!(sm.transition_to_awaiting_confirmation().is_err());
    // State unchanged.
    assert_eq!(sm.current_state(), CloneState::IDLE);
}

#[test]
fn test_cannot_skip_text_confirmation() {
    // From AWAITING_CONFIRMATION jump straight to READY_TO_CLONE (must fail).
    let mut sm = sm_at(CloneState::AWAITING_CONFIRMATION);
    let res = sm.transition_to(CloneState::READY_TO_CLONE);
    assert!(res.is_err());
    assert_eq!(sm.current_state(), CloneState::AWAITING_CONFIRMATION);
}

#[test]
fn test_cannot_skip_confirmation_entirely() {
    // VALIDATING → READY_TO_CLONE must fail (skips both intermediate states).
    let mut sm = sm_at(CloneState::VALIDATING);
    let res = sm.transition_to(CloneState::READY_TO_CLONE);
    assert!(res.is_err());
    assert_eq!(sm.current_state(), CloneState::VALIDATING);
}

#[test]
fn test_invalid_path_does_not_mutate_state() {
    let mut sm = sm_at(CloneState::IDLE);
    let _ = sm.transition_to_awaiting_confirmation();
    let _ = sm.transition_to_awaiting_text_confirmation();
    let _ = sm.transition_to_ready_to_clone();
    let _ = sm.transition_to_preparing();
    // All failed; state must still be IDLE.
    assert_eq!(sm.current_state(), CloneState::IDLE);
}

// ── Error / cancel from confirmation states ───────────────────────────────────

#[test]
fn test_error_from_awaiting_confirmation() {
    let mut sm = sm_at(CloneState::AWAITING_CONFIRMATION);
    sm.fail("cancelled");
    assert_eq!(sm.current_state(), CloneState::ERROR);
}

#[test]
fn test_cancel_from_awaiting_text_confirmation() {
    let mut sm = sm_at(CloneState::AWAITING_TEXT_CONFIRMATION);
    // Can abort back to IDLE via CANCELLING.
    assert!(sm.transition_to(CloneState::CANCELLING).is_ok());
    assert_eq!(sm.current_state(), CloneState::CANCELLING);
}

// ── Retry: text confirmation state allows re-entry ────────────────────────────

#[test]
fn test_awaiting_text_confirmation_allows_retry() {
    let mut sm = sm_at(CloneState::AWAITING_TEXT_CONFIRMATION);
    // Re-entering the same state (user typed wrong and gets another chance) is allowed.
    assert!(
        sm.transition_to(CloneState::AWAITING_TEXT_CONFIRMATION)
            .is_ok(),
        "retry loop should be permitted"
    );
}

// ── Confirmation → Idle (user cancels at the visual step) ────────────────────

#[test]
fn test_awaiting_confirmation_can_abort_to_idle() {
    let mut sm = sm_at(CloneState::AWAITING_CONFIRMATION);
    assert!(sm.transition_to(CloneState::IDLE).is_ok());
    assert_eq!(sm.current_state(), CloneState::IDLE);
}

#[test]
fn test_awaiting_text_confirmation_can_abort_to_idle() {
    let mut sm = sm_at(CloneState::AWAITING_TEXT_CONFIRMATION);
    assert!(sm.transition_to(CloneState::IDLE).is_ok());
    assert_eq!(sm.current_state(), CloneState::IDLE);
}

#[test]
fn test_ready_to_clone_can_abort_to_idle() {
    let mut sm = sm_at(CloneState::READY_TO_CLONE);
    assert!(sm.transition_to(CloneState::IDLE).is_ok());
    assert_eq!(sm.current_state(), CloneState::IDLE);
}
