pub mod engine;
pub mod history;
pub mod run;
pub mod shared;
pub mod syncs;
pub mod wizard;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::{Emitter, Manager};
    tauri::Builder::default()
        .manage(run::runner::ActiveRun::default())
        .manage(shared::preview_memo::PreviewMemo::default())
        .on_window_event(|window, event| {
            // Closing during a run: the UI asks first (same question as "Ferma la copia").
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window.state::<run::runner::ActiveRun>().is_running()
            {
                api.prevent_close();
                let _ = window.emit("close-requested", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            wizard::commands::preview_sync,
            run::commands::run_sync,
            run::commands::cancel_run,
            wizard::commands::browse_folders,
            wizard::commands::create_folder,
            syncs::commands::list_syncs,
            syncs::commands::save_sync,
            syncs::commands::default_sync_name,
            history::commands::list_runs,
            history::commands::read_run_log,
            wizard::commands::check_step,
            wizard::commands::describe_place,
            shared::elevation::is_elevated,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Never leave the copy engine running after the app is gone.
            if let tauri::RunEvent::Exit = event {
                app.state::<run::runner::ActiveRun>().cancel(false);
            }
        });
}
