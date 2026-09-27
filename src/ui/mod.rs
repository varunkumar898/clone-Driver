//! User Interface (Tauri) backend and IPC layer.
//!
//! Provides:
//! - State management for screen routing, selection, confirmation, and progress tracking
//! - IPC command handlers bridging Tauri invoke calls to domain logic
//! - Real-time confirmation validation and recovery integration

pub mod handlers;
pub mod state;

pub use handlers::*;
pub use state::*;
