//! Thin `#[tauri::command]` wrappers: robocopy runs on a blocking thread so the UI never freezes.

use crate::folder_browser::{self, FolderListing};
use crate::history::{self, Run};
use crate::preview_parser::{Counts, Preview};
use crate::robocopy::SyncMode;
use crate::runner::{self, ActiveRun};
use crate::storage::Storage;
use crate::sync_store::{self, SavedSync, SyncSummary};
use std::path::Path;
use tauri::async_runtime::spawn_blocking;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub async fn preview_sync(
    source: String,
    destination: String,
    mode: SyncMode,
) -> Result<Preview, String> {
    spawn_blocking(move || runner::preview(&source, &destination, mode))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_runs() -> Result<Vec<Run>, String> {
    let storage = Storage::from_env()?;
    spawn_blocking(move || history::read(&storage.history_file()).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

/// The robocopy log of one run; fails when the file is gone or not inside the logs folder.
#[tauri::command]
pub async fn read_run_log(path: String) -> Result<String, String> {
    let storage = Storage::from_env()?;
    spawn_blocking(move || {
        history::read_log(&storage.logs_dir(), Path::new(&path)).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn run_sync(
    app: AppHandle,
    source: String,
    destination: String,
    mode: SyncMode,
    counts: Counts,
    sync_name: String,
) -> Result<Run, String> {
    let storage = Storage::from_env()?;
    let run = spawn_blocking({
        let app = app.clone();
        move || {
            runner::run(
                &storage,
                &source,
                &destination,
                mode,
                counts,
                sync_name,
                &app.state::<ActiveRun>(),
                |progress| {
                    let _ = app.emit("run-progress", progress);
                },
            )
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    // The user confirmed closing the window during the run: the history line is already written.
    if app.state::<ActiveRun>().exit_when_done() {
        app.exit(0);
    }
    run
}

/// Stops the running copy; `exit_when_done` when the user confirmed closing the window. Returns
/// `false` when no run was in flight any more: closing the window then has nothing to wait for.
#[tauri::command]
pub fn cancel_run(app: AppHandle, active: State<'_, ActiveRun>, exit_when_done: bool) -> bool {
    let stopped = active.cancel(exit_when_done);
    if !stopped && exit_when_done {
        app.exit(0);
    }
    stopped
}

#[tauri::command]
pub async fn list_syncs() -> Result<Vec<SyncSummary>, String> {
    let storage = Storage::from_env()?;
    spawn_blocking(move || sync_store::list(&storage))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_sync(
    name: String,
    source: String,
    destination: String,
    mode: SyncMode,
) -> Result<SavedSync, String> {
    let storage = Storage::from_env()?;
    spawn_blocking(move || {
        sync_store::add(&storage.syncs_file(), &name, &source, &destination, mode)
            .map_err(|e| format!("Non riesco a salvare la sincronizzazione: {e}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn default_sync_name(source: String) -> String {
    runner::default_sync_name(&source)
}

/// Drives (empty `path`) or the subfolders of `path`. On a blocking thread: a disconnected
/// mapped drive can take seconds to answer and must not freeze the UI.
#[tauri::command]
pub async fn browse_folders(path: String) -> Result<FolderListing, String> {
    spawn_blocking(move || folder_browser::browse(&path))
        .await
        .map_err(|e| e.to_string())
}
