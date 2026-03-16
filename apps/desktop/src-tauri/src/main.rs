//! AXORA Desktop Entry Point
//!
//! Main entry point for the AXORA desktop application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    axora_desktop::run();
}
