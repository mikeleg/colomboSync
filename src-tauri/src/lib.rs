pub mod commands;
pub mod elevation;
pub mod error_messages;
pub mod folder_browser;
pub mod history;
pub mod mirror_guard;
pub mod path_rules;
pub mod preview_parser;
pub mod progress;
pub mod robocopy;
pub mod runner;
pub mod storage;
pub mod sync_store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::{Emitter, Manager};
    tauri::Builder::default()
        .manage(runner::ActiveRun::default())
        .manage(runner::PreviewMemo::default())
        .on_window_event(|window, event| {
            // Closing during a run: the UI asks first (same question as "Ferma la copia").
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window.state::<runner::ActiveRun>().is_running()
            {
                api.prevent_close();
                let _ = window.emit("close-requested", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::preview_sync,
            commands::run_sync,
            commands::cancel_run,
            commands::browse_folders,
            commands::list_syncs,
            commands::save_sync,
            commands::default_sync_name,
            commands::list_runs,
            commands::read_run_log,
            commands::check_destination,
            commands::is_elevated,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Never leave robocopy running after the app is gone.
            if let tauri::RunEvent::Exit = event {
                app.state::<runner::ActiveRun>().cancel(false);
            }
        });
}
