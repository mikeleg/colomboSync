//! Thin `#[tauri::command]` wrappers: robocopy runs on a blocking thread so the UI never freezes.

use crate::history::Run;
use crate::preview_parser::{Counts, Preview};
use crate::robocopy::SyncMode;
use crate::runner::{self, ActiveRun};
use crate::storage::Storage;
use std::time::{Duration, Instant};
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

/// Progress events are throttled: robocopy can print thousands of lines a second.
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

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
            let mut last_emit: Option<Instant> = None;
            runner::run(
                &storage,
                &source,
                &destination,
                mode,
                counts,
                &app.state::<ActiveRun>(),
                |progress| {
                    if last_emit.is_none_or(|t| t.elapsed() >= PROGRESS_EVERY) {
                        last_emit = Some(Instant::now());
                        let _ = app.emit("run-progress", progress);
                    }
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

/// Stops the running copy; `exit_when_done` when the user confirmed closing the window.
#[tauri::command]
pub fn cancel_run(active: State<'_, ActiveRun>, exit_when_done: bool) -> bool {
    active.cancel(exit_when_done)
}
