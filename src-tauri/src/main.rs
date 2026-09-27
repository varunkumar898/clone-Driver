// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod handlers;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::scan_devices,
            commands::start_clone,
            commands::pause_clone,
            commands::resume_clone,
            commands::cancel_clone,
            commands::confirm_visual_check,
            commands::confirm_text_input,
            commands::final_safety_check,
            commands::check_recovery_checkpoint,
            commands::verify_clone,
            commands::get_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running DiskClone Tauri application");
}
