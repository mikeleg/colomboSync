//! Thin `#[tauri::command]` wrappers: robocopy runs on a blocking thread so the UI never freezes.

use crate::history::Run;
use crate::preview_parser::{Counts, Preview};
use crate::robocopy::SyncMode;
use crate::runner::{self, ActiveRun};
use crate::storage::Storage;
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
pub async fn run_sync(
    app: AppHandle,
    source: String,
    destination: String,
    mode: SyncMode,
    counts: Counts,
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
