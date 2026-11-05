//! Thin `#[tauri::command]` wrappers for the Storico screen.

use super::summary::{self, HistorySummary};
use super::{Run, read, read_log};
use crate::shared::blocking::blocking;
use crate::shared::storage::Storage;
use crate::syncs::sync_store;
use serde::Serialize;
use std::path::Path;

/// What the Storico screen shows: the runs, newest first, and the totals and per-sync state for its
/// side panel.
#[derive(Serialize)]
pub struct RunList {
    runs: Vec<Run>,
    summary: HistorySummary,
}

/// A `syncs.json` that cannot be read only means the syncs keep the names they had in their runs.
#[tauri::command]
pub async fn list_runs() -> Result<RunList, String> {
    let storage = Storage::from_env()?;
    blocking(move || {
        let runs = read(&storage.history_file()).map_err(|e| e.to_string())?;
        let syncs = sync_store::load(&storage.syncs_file()).unwrap_or_default();
        let summary = summary::summarize(&runs, &syncs);
        Ok(RunList { runs, summary })
    })
    .await
}

/// The robocopy log of one run; fails when the file is gone or not inside the logs folder.
#[tauri::command]
pub async fn read_run_log(path: String) -> Result<String, String> {
    let storage = Storage::from_env()?;
    blocking(move || read_log(&storage.logs_dir(), Path::new(&path)).map_err(|e| e.to_string()))
        .await
}
