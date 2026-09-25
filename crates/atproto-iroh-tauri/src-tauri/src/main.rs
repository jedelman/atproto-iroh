// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Desktop entry point — mobile has none of its own (see `lib.rs`'s top
//! doc comment); this just calls into the real app logic there.

fn main() {
    atproto_iroh_tauri_lib::run();
}
