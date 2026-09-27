//! Workflow state machine definitions and transition coordinator.

use serde::{Deserialize, Serialize};

use crate::error::ConfirmationError;

/// High-level operational states in the disk cloning workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum CloneState {
    IDLE,
    AWAITING_SOURCE,
    AWAITING_DESTINATION,
    VALIDATING,
    AWAITING_CONFIRMATION,
    AWAITING_TEXT_CONFIRMATION,
    READY_TO_CLONE,
    PREPARING,
    CLONING,
    FLUSH,
    VERIFYING,
    COMPLETED,
    CANCELLING,
    ERROR,
}

impl std::fmt::Display for CloneState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type State = CloneState;

/// State machine tracking operational state and enforcing valid transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateMachine {
    pub state: CloneState,
    pub error_message: Option<String>,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    /// Creates a new state machine initialized at `IDLE`.
    pub fn new() -> Self {
        Self {
            state: CloneState::IDLE,
            error_message: None,
        }
    }

    /// Current operational state.
    pub fn current_state(&self) -> CloneState {
        self.state
    }

    /// Checks whether transition from current state to `target` is allowed.
    #[allow(clippy::match_like_matches_macro)]
    pub fn can_transition(&self, target: CloneState) -> bool {
        if target == CloneState::ERROR {
            return true;
        }

        match (self.state, target) {
            (CloneState::IDLE, CloneState::AWAITING_SOURCE) => true,
            (CloneState::AWAITING_SOURCE, CloneState::AWAITING_DESTINATION) => true,
            (CloneState::AWAITING_DESTINATION, CloneState::VALIDATING) => true,
            (CloneState::VALIDATING, CloneState::AWAITING_CONFIRMATION) => true,
            (CloneState::AWAITING_CONFIRMATION, CloneState::AWAITING_TEXT_CONFIRMATION) => true,
            (CloneState::AWAITING_CONFIRMATION, CloneState::IDLE) => true,
            (CloneState::AWAITING_TEXT_CONFIRMATION, CloneState::READY_TO_CLONE) => true,
            (CloneState::AWAITING_TEXT_CONFIRMATION, CloneState::AWAITING_CONFIRMATION) => true,
            (CloneState::AWAITING_TEXT_CONFIRMATION, CloneState::AWAITING_TEXT_CONFIRMATION) => {
                true
            }
            (CloneState::AWAITING_TEXT_CONFIRMATION, CloneState::IDLE) => true,
            (CloneState::READY_TO_CLONE, CloneState::PREPARING) => true,
            (CloneState::READY_TO_CLONE, CloneState::IDLE) => true,
            (CloneState::PREPARING, CloneState::CLONING) => true,
            (CloneState::CLONING, CloneState::FLUSH) => true,
            (CloneState::FLUSH, CloneState::VERIFYING) => true,
            (CloneState::VERIFYING, CloneState::COMPLETED) => true,
            (CloneState::COMPLETED, CloneState::IDLE) => true,
            (_, CloneState::CANCELLING) => true,
            (CloneState::CANCELLING, CloneState::IDLE) => true,
            (CloneState::ERROR, CloneState::IDLE) => true,
            _ => false,
        }
    }

    /// Transitions to the target state if valid, else returns error without altering state.
    pub fn transition_to(&mut self, target: CloneState) -> Result<(), ConfirmationError> {
        if self.can_transition(target) {
            self.state = target;
            if target != CloneState::ERROR {
                self.error_message = None;
            }
            Ok(())
        } else {
            Err(ConfirmationError::StateInvalid {
                current: self.state,
            })
        }
    }

    pub fn transition_to_awaiting_confirmation(&mut self) -> Result<(), ConfirmationError> {
        self.transition_to(CloneState::AWAITING_CONFIRMATION)
    }

    pub fn transition_to_awaiting_text_confirmation(&mut self) -> Result<(), ConfirmationError> {
        self.transition_to(CloneState::AWAITING_TEXT_CONFIRMATION)
    }

    pub fn transition_to_ready_to_clone(&mut self) -> Result<(), ConfirmationError> {
        self.transition_to(CloneState::READY_TO_CLONE)
    }

    pub fn transition_to_preparing(&mut self) -> Result<(), ConfirmationError> {
        self.transition_to(CloneState::PREPARING)
    }

    /// Transitions directly into Error state with a diagnostic message.
    pub fn fail(&mut self, msg: impl Into<String>) {
        self.state = CloneState::ERROR;
        self.error_message = Some(msg.into());
    }

    /// Resets the state machine back to `IDLE`.
    pub fn reset(&mut self) {
        self.state = CloneState::IDLE;
        self.error_message = None;
    }
}

pub type StateCoordinator = StateMachine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_transition_valid_path() {
        let mut sm = StateMachine::new();
        assert_eq!(sm.current_state(), CloneState::IDLE);

        assert!(sm.transition_to(CloneState::AWAITING_SOURCE).is_ok());
        assert!(sm.transition_to(CloneState::AWAITING_DESTINATION).is_ok());
        assert!(sm.transition_to(CloneState::VALIDATING).is_ok());
        assert!(sm.transition_to_awaiting_confirmation().is_ok());
        assert_eq!(sm.current_state(), CloneState::AWAITING_CONFIRMATION);

        assert!(sm.transition_to_awaiting_text_confirmation().is_ok());
        assert_eq!(sm.current_state(), CloneState::AWAITING_TEXT_CONFIRMATION);

        assert!(sm.transition_to_ready_to_clone().is_ok());
        assert_eq!(sm.current_state(), CloneState::READY_TO_CLONE);

        assert!(sm.transition_to_preparing().is_ok());
        assert_eq!(sm.current_state(), CloneState::PREPARING);

        assert!(sm.transition_to(CloneState::CLONING).is_ok());
        assert!(sm.transition_to(CloneState::FLUSH).is_ok());
        assert!(sm.transition_to(CloneState::VERIFYING).is_ok());
        assert!(sm.transition_to(CloneState::COMPLETED).is_ok());
    }

    #[test]
    fn test_state_transition_invalid_stays() {
        let mut sm = StateMachine::new();
        assert!(sm.transition_to(CloneState::AWAITING_SOURCE).is_ok());

        // Attempt invalid skip from AWAITING_SOURCE to READY_TO_CLONE
        let res = sm.transition_to(CloneState::READY_TO_CLONE);
        assert!(res.is_err());
        assert_eq!(sm.current_state(), CloneState::AWAITING_SOURCE);
    }

    #[test]
    fn test_state_validation_prevents_skip() {
        let mut sm = StateMachine::new();
        // Cannot transition from IDLE directly to CLONING
        assert!(sm.transition_to(CloneState::CLONING).is_err());
        assert_eq!(sm.current_state(), CloneState::IDLE);

        // Cannot skip text confirmation from AWAITING_CONFIRMATION to READY_TO_CLONE
        sm.state = CloneState::AWAITING_CONFIRMATION;
        assert!(sm.transition_to(CloneState::READY_TO_CLONE).is_err());
        assert_eq!(sm.current_state(), CloneState::AWAITING_CONFIRMATION);
    }
}
