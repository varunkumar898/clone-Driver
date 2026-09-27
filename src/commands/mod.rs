//! IPC command handlers for the DiskClone backend.
//!
//! Pure-Rust handlers callable from Tauri commands or tests.  The `src-tauri`
//! crate wraps these with `#[tauri::command]` and wires them into the Tauri
//! application builder.

pub mod confirm;

pub use confirm::{
    handle_confirm_text_input, handle_confirm_visual_check, handle_final_safety_check,
};
