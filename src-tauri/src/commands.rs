//! Thin `#[tauri::command]` wrappers: robocopy runs on a blocking thread so the UI never freezes.

use crate::history::{self, Run};
use crate::preview_parser::{Counts, Preview};
use crate::robocopy::SyncMode;
use crate::runner;
use crate::storage::Storage;
use std::path::Path;
use tauri::async_runtime::spawn_blocking;

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
    source: String,
    destination: String,
    mode: SyncMode,
    counts: Counts,
) -> Result<Run, String> {
    let storage = Storage::from_env()?;
    spawn_blocking(move || runner::run(&storage, &source, &destination, mode, counts))
        .await
        .map_err(|e| e.to_string())?
}
