#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


use std::env::{set_var, var};
const HOME_PAGE_KEY:&str = "HOME_PAGE";
#[tauri::command]
async fn get_dynamic_url() -> String {
    // Logic to determine the dynamic URL
    var(HOME_PAGE_KEY).unwrap()
}


pub fn start_window(url: &str) -> tauri::Result<()> {
    set_var(HOME_PAGE_KEY, url);
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_dynamic_url])
        .run(tauri::generate_context!())
}
