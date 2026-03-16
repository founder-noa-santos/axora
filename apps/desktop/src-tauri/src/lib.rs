//! AXORA Desktop Tauri Application
//!
//! This module contains the Tauri application setup and command handlers
//! for the AXORA desktop interface.

use tauri::Manager;

/// Greet command handler
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Initialize the Tauri application
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
