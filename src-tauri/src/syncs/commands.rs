//! Thin `#[tauri::command]` wrappers for the saved syncs.

use super::listing::{self, ListedSyncs};
use crate::shared::blocking::blocking;
use crate::shared::domain::{SavedSync, SyncMode};
use crate::shared::storage::Storage;
use crate::shared::sync_store;

#[tauri::command]
pub async fn list_syncs() -> Result<ListedSyncs, String> {
    let storage = Storage::from_env()?;
    blocking(move || {
        listing::list(&storage)
            .map_err(|_| "Non riesco a leggere le sincronizzazioni salvate.".to_string())
    })
    .await
}

#[tauri::command]
pub async fn save_sync(
    name: String,
    source: String,
    destination: String,
    mode: SyncMode,
) -> Result<SavedSync, String> {
    let storage = Storage::from_env()?;
    blocking(move || {
        sync_store::add(&storage.syncs_file(), &name, &source, &destination, mode)
            .map_err(|e| format!("Non riesco a salvare la sincronizzazione: {e}"))
    })
    .await
}

#[tauri::command]
pub fn default_sync_name(source: String) -> String {
    sync_store::default_sync_name(&source)
}
