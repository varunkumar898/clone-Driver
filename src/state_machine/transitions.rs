//! State transition validation rules and state machine coordinator.
//!
//! The transition table enforces the complete Phase 6 confirmation flow:
//! Idle → ScanningDevices → AwaitingSourceSelection → AwaitingDestinationSelection
//!   → ValidatingSelection → AwaitingConfirmation → Preparing → Cloning
//!   → Flush → Verifying → Completed
//!
//! Any state can transition to Error or Cancelling (graceful abort).

use super::context::StateMachineContext;
use super::state::CloneState;
use crate::error::{Result, StateMachineError};

/// State machine coordinator executing transitions and enforcing strict rules.
#[derive(Debug)]
pub struct StateMachine {
    current_state: CloneState,
    context: StateMachineContext,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    /// Creates a new state machine initialized in the `Idle` state.
    pub fn new() -> Self {
        Self {
            current_state: CloneState::Idle,
            context: StateMachineContext::new(),
        }
    }

    /// Returns the current state.
    pub fn state(&self) -> CloneState {
        self.current_state
    }

    /// Returns reference to operational context.
    pub fn context(&self) -> &StateMachineContext {
        &self.context
    }

    /// Returns mutable reference to operational context.
    pub fn context_mut(&mut self) -> &mut StateMachineContext {
        &mut self.context
    }

    /// Validates whether a transition from the *current* state to `to` is legally allowed.
    ///
    /// The `from` parameter is retained for API compatibility with callers that pass both
    /// states explicitly; it must match `self.current_state` for a transition to be valid.
    pub fn can_transition(&self, from: CloneState, to: CloneState) -> bool {
        // Caller-supplied `from` must match reality.
        if from != self.current_state {
            return false;
        }

        // Error and Cancelling are always reachable from any non-terminal state.
        if matches!(to, CloneState::Error | CloneState::Cancelling) {
            return !matches!(
                self.current_state,
                CloneState::Completed | CloneState::Error
            );
        }

        // Cancelling → Idle (reset after abort).
        // Error → Idle (reset after error acknowledgement).
        if matches!(
            self.current_state,
            CloneState::Cancelling | CloneState::Error
        ) && to == CloneState::Idle
        {
            return true;
        }

        // Happy-path transitions (strict linear sequence).
        matches!(
            (self.current_state, to),
            (CloneState::Idle, CloneState::ScanningDevices)
                | (CloneState::ScanningDevices, CloneState::AwaitingSourceSelection)
                | (CloneState::AwaitingSourceSelection, CloneState::AwaitingDestinationSelection)
                | (CloneState::AwaitingDestinationSelection, CloneState::ValidatingSelection)
                | (CloneState::ValidatingSelection, CloneState::AwaitingConfirmation)
                // Confirmation rejected → back to Idle (user can start over).
                | (CloneState::AwaitingConfirmation, CloneState::Idle)
                | (CloneState::AwaitingConfirmation, CloneState::Preparing)
                | (CloneState::Preparing, CloneState::Cloning)
                | (CloneState::Cloning, CloneState::Flush)
                | (CloneState::Flush, CloneState::Verifying)
                | (CloneState::Verifying, CloneState::Completed)
                // Reset from terminal states.
                | (CloneState::Completed, CloneState::Idle)
        )
    }

    /// Attempts to transition the state machine to a target state.
    ///
    /// Returns `Err(StateMachineError::InvalidTransition)` without mutating state
    /// if the transition is not permitted by the transition table.
    pub fn transition_to(&mut self, target: CloneState) -> Result<()> {
        if self.can_transition(self.current_state, target) {
            self.current_state = target;
            Ok(())
        } else {
            Err(StateMachineError::InvalidTransition {
                from: self.current_state.to_string(),
                to: target.to_string(),
            }
            .into())
        }
    }

    /// Transitions to the `Error` state and records a diagnostic message in the context.
    pub fn fail(&mut self, error: impl Into<String>) {
        self.context.error_message = Some(error.into());
        self.current_state = CloneState::Error;
    }

    /// Convenience: transition to `Cancelling`.
    pub fn cancel(&mut self) -> Result<()> {
        self.transition_to(CloneState::Cancelling)
    }

    /// Resets the machine to `Idle`, clearing context. Only valid from terminal states.
    pub fn reset(&mut self) -> Result<()> {
        if matches!(
            self.current_state,
            CloneState::Idle | CloneState::Completed | CloneState::Error | CloneState::Cancelling
        ) {
            self.current_state = CloneState::Idle;
            self.context.reset();
            Ok(())
        } else {
            Err(StateMachineError::InvalidTransition {
                from: self.current_state.to_string(),
                to: "Idle".to_string(),
            }
            .into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_idle() {
        let sm = StateMachine::new();
        assert_eq!(sm.state(), CloneState::Idle);
    }

    #[test]
    fn test_happy_path_transitions() {
        let mut sm = StateMachine::new();
        assert!(sm.transition_to(CloneState::ScanningDevices).is_ok());
        assert!(sm
            .transition_to(CloneState::AwaitingSourceSelection)
            .is_ok());
        assert!(sm
            .transition_to(CloneState::AwaitingDestinationSelection)
            .is_ok());
        assert!(sm.transition_to(CloneState::ValidatingSelection).is_ok());
        assert!(sm.transition_to(CloneState::AwaitingConfirmation).is_ok());
        assert!(sm.transition_to(CloneState::Preparing).is_ok());
        assert!(sm.transition_to(CloneState::Cloning).is_ok());
        assert!(sm.transition_to(CloneState::Flush).is_ok());
        assert!(sm.transition_to(CloneState::Verifying).is_ok());
        assert!(sm.transition_to(CloneState::Completed).is_ok());
        assert_eq!(sm.state(), CloneState::Completed);
    }

    #[test]
    fn test_illegal_skip_rejected() {
        let mut sm = StateMachine::new();
        // Cannot skip from Idle directly to Cloning.
        let res = sm.transition_to(CloneState::Cloning);
        assert!(res.is_err());
        assert_eq!(sm.state(), CloneState::Idle);
    }

    #[test]
    fn test_cannot_skip_confirmation() {
        let mut sm = StateMachine::new();
        // Walk to ValidatingSelection
        sm.transition_to(CloneState::ScanningDevices).unwrap();
        sm.transition_to(CloneState::AwaitingSourceSelection)
            .unwrap();
        sm.transition_to(CloneState::AwaitingDestinationSelection)
            .unwrap();
        sm.transition_to(CloneState::ValidatingSelection).unwrap();
        // Skip AwaitingConfirmation → go straight to Preparing: must fail.
        let res = sm.transition_to(CloneState::Preparing);
        assert!(res.is_err());
        assert_eq!(sm.state(), CloneState::ValidatingSelection);
    }

    #[test]
    fn test_error_transition_from_any_active_state() {
        let mut sm = StateMachine::new();
        sm.transition_to(CloneState::ScanningDevices).unwrap();
        sm.fail("disk error");
        assert_eq!(sm.state(), CloneState::Error);
        assert_eq!(sm.context().error_message.as_deref(), Some("disk error"));
    }

    #[test]
    fn test_cancelling_from_cloning() {
        let mut sm = StateMachine::new();
        sm.transition_to(CloneState::ScanningDevices).unwrap();
        sm.transition_to(CloneState::AwaitingSourceSelection)
            .unwrap();
        sm.transition_to(CloneState::AwaitingDestinationSelection)
            .unwrap();
        sm.transition_to(CloneState::ValidatingSelection).unwrap();
        sm.transition_to(CloneState::AwaitingConfirmation).unwrap();
        sm.transition_to(CloneState::Preparing).unwrap();
        sm.transition_to(CloneState::Cloning).unwrap();
        assert!(sm.cancel().is_ok());
        assert_eq!(sm.state(), CloneState::Cancelling);
    }

    #[test]
    fn test_reset_from_completed() {
        let mut sm = StateMachine::new();
        // Bring to Completed.
        sm.transition_to(CloneState::ScanningDevices).unwrap();
        sm.transition_to(CloneState::AwaitingSourceSelection)
            .unwrap();
        sm.transition_to(CloneState::AwaitingDestinationSelection)
            .unwrap();
        sm.transition_to(CloneState::ValidatingSelection).unwrap();
        sm.transition_to(CloneState::AwaitingConfirmation).unwrap();
        sm.transition_to(CloneState::Preparing).unwrap();
        sm.transition_to(CloneState::Cloning).unwrap();
        sm.transition_to(CloneState::Flush).unwrap();
        sm.transition_to(CloneState::Verifying).unwrap();
        sm.transition_to(CloneState::Completed).unwrap();

        assert!(sm.reset().is_ok());
        assert_eq!(sm.state(), CloneState::Idle);
    }

    #[test]
    fn test_reset_from_active_fails() {
        let mut sm = StateMachine::new();
        sm.transition_to(CloneState::ScanningDevices).unwrap();
        // Cannot reset while scanning
        assert!(sm.reset().is_err());
        assert_eq!(sm.state(), CloneState::ScanningDevices);
    }

    #[test]
    fn test_can_transition_requires_from_matches_current() {
        let sm = StateMachine::new(); // Idle
                                      // From must match current (Idle); using ScanningDevices as from → false.
        assert!(!sm.can_transition(
            CloneState::ScanningDevices,
            CloneState::AwaitingSourceSelection
        ));
    }

    #[test]
    fn test_cannot_cancel_from_completed() {
        let mut sm = StateMachine::new();
        sm.transition_to(CloneState::ScanningDevices).unwrap();
        sm.transition_to(CloneState::AwaitingSourceSelection)
            .unwrap();
        sm.transition_to(CloneState::AwaitingDestinationSelection)
            .unwrap();
        sm.transition_to(CloneState::ValidatingSelection).unwrap();
        sm.transition_to(CloneState::AwaitingConfirmation).unwrap();
        sm.transition_to(CloneState::Preparing).unwrap();
        sm.transition_to(CloneState::Cloning).unwrap();
        sm.transition_to(CloneState::Flush).unwrap();
        sm.transition_to(CloneState::Verifying).unwrap();
        sm.transition_to(CloneState::Completed).unwrap();
        // Cancelling from Completed is not allowed.
        assert!(sm.cancel().is_err());
    }
}
