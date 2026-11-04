//! Thin `#[tauri::command]` wrappers: robocopy runs on a blocking thread so the UI never freezes.

use crate::folder_browser::{self, FolderListing};
use crate::history::Run;
use crate::preview_parser::{Counts, Preview};
use crate::robocopy::SyncMode;
use crate::runner;
use crate::storage::Storage;
use crate::sync_store::{self, SavedSync, SyncSummary};
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
pub async fn run_sync(
    source: String,
    destination: String,
    mode: SyncMode,
    counts: Counts,
    sync_name: String,
) -> Result<Run, String> {
    let storage = Storage::from_env()?;
    spawn_blocking(move || runner::run(&storage, &source, &destination, mode, counts, sync_name))
        .await
        .map_err(|e| e.to_string())?
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
