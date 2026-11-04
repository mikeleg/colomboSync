//! Thin `#[tauri::command]` wrappers: robocopy runs on a blocking thread so the UI never freezes.

use crate::folder_browser::{self, FolderListing};
use crate::history::Run;
use crate::path_rules;
use crate::preview_parser::{Counts, Preview};
use crate::robocopy::SyncMode;
use crate::runner;
use crate::storage::Storage;
use tauri::async_runtime::spawn_blocking;

/// Rejects a destination that is A itself or inside A, with the message to show on step "Dove".
#[tauri::command]
pub fn check_destination(source: String, destination: String) -> Result<(), String> {
    path_rules::check_destination(&source, &destination).map_err(str::to_string)
}

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
) -> Result<Run, String> {
    let storage = Storage::from_env()?;
    spawn_blocking(move || runner::run(&storage, &source, &destination, mode, counts))
        .await
        .map_err(|e| e.to_string())?
}

/// Drives (empty `path`) or the subfolders of `path`. On a blocking thread: a disconnected
/// mapped drive can take seconds to answer and must not freeze the UI.
#[tauri::command]
pub async fn browse_folders(path: String) -> Result<FolderListing, String> {
    spawn_blocking(move || folder_browser::browse(&path))
        .await
        .map_err(|e| e.to_string())
}
