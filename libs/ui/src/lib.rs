#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


use std::env::{set_var, var};

#[tauri::command]
async fn get_dynamic_url() -> String {
    // Logic to determine the dynamic URL
    var("TAURI_DEV_PATH").unwrap()
}


pub fn start_window(url: &str) -> tauri::Result<()> {
    set_var("TAURI_DEV_PATH", url);
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_dynamic_url])
        .run(tauri::generate_context!())
}
