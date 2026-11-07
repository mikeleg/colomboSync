//! Thin `#[tauri::command]` wrappers for the saved syncs.

use super::listing::{self, ListedSyncs};
use crate::shared::blocking::blocking;
use crate::shared::domain::{SavedSync, SyncMode};
use crate::shared::error_messages::{self, Explanation};
use crate::shared::storage::Storage;
use crate::shared::sync_store;

#[tauri::command]
pub async fn list_syncs() -> Result<ListedSyncs, Explanation> {
    let storage = Storage::from_env()?;
    blocking(move || listing::list(&storage).map_err(|_| error_messages::cannot_read_syncs())).await
}

#[tauri::command]
pub async fn save_sync(
    name: String,
    source: String,
    destination: String,
    mode: SyncMode,
) -> Result<SavedSync, Explanation> {
    let storage = Storage::from_env()?;
    blocking(move || {
        sync_store::add(&storage.syncs_file(), &name, &source, &destination, mode)
            .map_err(|e| error_messages::cannot_save_sync(&e))
    })
    .await
}

#[tauri::command]
pub fn default_sync_name(source: String) -> String {
    sync_store::default_sync_name(&source)
}
