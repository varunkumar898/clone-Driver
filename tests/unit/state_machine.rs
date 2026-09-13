use diskclone::state_machine::{CloneState, StateMachine};

#[test]
fn test_initial_state_is_idle() {
    let sm = StateMachine::new();
    assert_eq!(sm.state(), CloneState::Idle);
}

#[test]
fn test_state_display() {
    assert_eq!(format!("{}", CloneState::Idle), "Idle");
    assert_eq!(format!("{}", CloneState::Cloning), "Cloning");
}
