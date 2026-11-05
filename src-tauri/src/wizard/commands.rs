//! Thin `#[tauri::command]` wrappers for the wizard: the engine runs on a blocking thread so the UI
//! never freezes.

use super::folder_browser::{self, FolderListing};
use super::preview::{self, PreviewMemo};
use crate::engine::preview_parser::Preview;
use crate::shared::blocking::blocking;
use crate::shared::domain::SyncMode;
use crate::shared::error_messages::Explanation;
use crate::shared::path_rules;
use tauri::{AppHandle, Manager};

/// Rejects a destination that is A itself or inside A, with the message to show on step "Dove".
#[tauri::command]
pub fn check_destination(source: String, destination: String) -> Result<(), String> {
    path_rules::check_destination(&source, &destination).map_err(str::to_string)
}

#[tauri::command]
pub async fn preview_sync(
    app: AppHandle,
    source: String,
    destination: String,
    mode: SyncMode,
) -> Result<Preview, Explanation> {
    let preview = blocking({
        let (source, destination) = (source.clone(), destination.clone());
        move || preview::preview(&source, &destination, mode)
    })
    .await?;
    app.state::<PreviewMemo>()
        .remember(&source, &destination, mode, preview.counts());
    Ok(preview)
}

/// Drives (empty `path`) or the subfolders of `path`. On a blocking thread: a disconnected
/// mapped drive can take seconds to answer and must not freeze the UI.
#[tauri::command]
pub async fn browse_folders(path: String) -> Result<FolderListing, String> {
    blocking(move || Ok(folder_browser::browse(&path))).await
}

/// A new folder `name` inside `parent`, whose path is returned. Blocking thread, like browsing.
#[tauri::command]
pub async fn create_folder(parent: String, name: String) -> Result<String, Explanation> {
    blocking(move || folder_browser::create_folder(&parent, &name)).await
}
