//! Thin `#[tauri::command]` wrappers for the wizard: the engine runs on a blocking thread so the UI
//! never freezes.

use super::folder_browser::{self, FolderListing};
use super::preview::{self, PreviewMemo};
use crate::engine::preview_parser::Preview;
use crate::shared::blocking::blocking;
use crate::shared::domain::SyncMode;
use crate::shared::error_messages::Explanation;
use crate::shared::path_rules;
use serde::Serialize;
use tauri::{AppHandle, Manager};

/// Rejects a destination that is A itself or inside A, with the message to show on step "Dove".
#[tauri::command]
pub fn check_destination(source: String, destination: String) -> Result<(), String> {
    path_rules::check_destination(&source, &destination).map_err(str::to_string)
}

/// A preview and the token that lets `run_sync` confirm it; no token when the preview is blocked.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewReport {
    #[serde(flatten)]
    preview: Preview,
    preview_token: Option<String>,
}

#[tauri::command]
pub async fn preview_sync(
    app: AppHandle,
    source: String,
    destination: String,
    mode: SyncMode,
) -> Result<PreviewReport, Explanation> {
    let preview = blocking({
        let (source, destination) = (source.clone(), destination.clone());
        move || preview::preview(&source, &destination, mode)
    })
    .await?;
    let preview_token = app
        .state::<PreviewMemo>()
        .remember(&source, &destination, mode, &preview);
    Ok(PreviewReport {
        preview,
        preview_token,
    })
}

/// Drives (empty `path`) or the subfolders of `path`. On a blocking thread: a disconnected
/// mapped drive can take seconds to answer and must not freeze the UI.
#[tauri::command]
pub async fn browse_folders(path: String) -> Result<FolderListing, String> {
    blocking(move || Ok(folder_browser::browse(&path))).await
}

/// A new folder `name` inside `parent` to be used as B for the folder A `source`, whose path is
/// returned. Blocking thread, like browsing.
#[tauri::command]
pub async fn create_folder(
    source: String,
    parent: String,
    name: String,
) -> Result<String, Explanation> {
    blocking(move || folder_browser::create_folder(&source, &parent, &name)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_goes_to_the_ui_with_its_token_next_to_the_counted_groups() {
        let report = |preview_token: Option<&str>| PreviewReport {
            preview: Preview::default(),
            preview_token: preview_token.map(str::to_string),
        };

        let json = serde_json::to_value(report(Some("1-abc"))).unwrap();
        assert_eq!(json["previewToken"], "1-abc");
        assert_eq!(json["new"]["count"], 0);
        assert!(json["blocked"].is_null());

        let json = serde_json::to_value(report(None)).unwrap();
        assert!(json["previewToken"].is_null());
    }
}
