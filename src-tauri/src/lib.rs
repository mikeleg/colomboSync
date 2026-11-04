pub mod commands;
pub mod folder_browser;
pub mod history;
pub mod preview_parser;
pub mod robocopy;
pub mod runner;
pub mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::preview_sync,
            commands::run_sync,
            commands::browse_folders,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
