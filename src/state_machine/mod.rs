//! Clone finite state machine module.
//!
//! Enforces deterministic lifecycle transitions for disk cloning operations,
//! preventing illegal state mutations and ensuring safe resource teardown.

pub mod context;
pub mod state;
pub mod transitions;

pub use context::StateMachineContext;
pub use state::CloneState;
pub use transitions::StateMachine;
