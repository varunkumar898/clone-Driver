//! Integration tests for Phase 6 Safety Layer.
//!
//! Validates end-to-end integration between:
//! - Device safety checks (SystemDiskDetector, CapacityValidator)
//! - Confirmation manager and text validation
//! - State machine transitions across the confirmation gate

#[cfg(test)]
mod tests {
    use diskclone::confirm::{ConfirmationManager, ConfirmationRequest, ConfirmationResponse};
    use diskclone::device::safety::{CapacityValidator, SystemDiskDetector};
    use diskclone::state::{CloneState, StateMachine};

    #[test]
    fn test_confirmation_flow_complete() {
        let mut manager = ConfirmationManager::new();

        // Set request
        let request = ConfirmationRequest::new(
            "Source".to_string(),
            100.0,
            "S1".to_string(),
            "Dest".to_string(),
            200.0,
            "D1".to_string(),
        );
        manager.set_request(request);

        // User confirms visually & via text
        let response = ConfirmationResponse {
            visual_confirmed: true,
            text_confirmed: true,
            text_provided: "CLONE TO THIS DISK".to_string(),
        };
        manager.set_response(response);

        // Verify complete
        assert!(manager.is_confirmed());
    }

    #[test]
    fn test_text_confirmation_case_sensitive() {
        let result = ConfirmationManager::validate_text_confirmation("clone to this disk");
        assert!(result.is_err());

        let result_correct = ConfirmationManager::validate_text_confirmation("CLONE TO THIS DISK");
        assert!(result_correct.is_ok());
    }

    #[test]
    fn test_system_disk_detection_interface() {
        // Just verify no panic
        let result = SystemDiskDetector::detect_boot_device();
        assert!(result.is_ok());
    }

    #[test]
    fn test_capacity_validation_integration() {
        // Destination >= Source is allowed
        assert!(CapacityValidator::validate_capacity(1_000_000_000, 2_000_000_000).is_ok());
        assert!(CapacityValidator::validate_capacity(1_000_000_000, 1_000_000_000).is_ok());

        // Destination < Source fails
        let err = CapacityValidator::validate_capacity(2_000_000_000, 1_000_000_000);
        assert!(err.is_err());
    }

    #[test]
    fn test_state_machine_with_safety_integration() {
        let mut sm = StateMachine::new();

        // Navigate to VALIDATING
        sm.transition_to(CloneState::AWAITING_SOURCE).unwrap();
        sm.transition_to(CloneState::AWAITING_DESTINATION).unwrap();
        sm.transition_to(CloneState::VALIDATING).unwrap();

        // Safety gate 1: Capacity check passes
        let cap_ok = CapacityValidator::validate_capacity(500 * 1024 * 1024, 1000 * 1024 * 1024);
        assert!(cap_ok.is_ok());

        // Transition to visual confirmation
        sm.transition_to_awaiting_confirmation().unwrap();
        assert_eq!(sm.current_state(), CloneState::AWAITING_CONFIRMATION);

        // User visual confirmation acknowledged -> move to text confirmation
        sm.transition_to_awaiting_text_confirmation().unwrap();
        assert_eq!(sm.current_state(), CloneState::AWAITING_TEXT_CONFIRMATION);

        // Text confirmation validated
        let text_res = ConfirmationManager::validate_text_confirmation("CLONE TO THIS DISK");
        assert!(text_res.is_ok());

        sm.transition_to_ready_to_clone().unwrap();
        assert_eq!(sm.current_state(), CloneState::READY_TO_CLONE);

        // Final safety gate check before preparing
        sm.transition_to_preparing().unwrap();
        assert_eq!(sm.current_state(), CloneState::PREPARING);
    }
}
