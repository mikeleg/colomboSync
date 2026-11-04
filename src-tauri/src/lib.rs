pub mod commands;
pub mod history;
pub mod preview_parser;
pub mod robocopy;
pub mod runner;
pub mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::preview_sync,
            commands::run_sync,
            commands::list_runs,
            commands::read_run_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
