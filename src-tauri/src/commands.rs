//! Thin `#[tauri::command]` wrappers: robocopy runs on a blocking thread so the UI never freezes.

use crate::elevation;
use crate::engine::SyncMode;
use crate::error_messages::{self, Explanation};
use crate::folder_browser::{self, FolderListing};
use crate::history::{self, Run};
use crate::history_summary::{self, HistorySummary};
use crate::path_rules;
use crate::preview_parser::{Counts, Preview};
use crate::runner::{self, ActiveRun, PreviewMemo};
use crate::storage::Storage;
use crate::sync_store::{self, ListedSyncs, SavedSync};
use serde::Serialize;
use std::path::Path;
use tauri::async_runtime::spawn_blocking;
use tauri::{AppHandle, Emitter, Manager, State};

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
        move || runner::preview(&source, &destination, mode)
    })
    .await?;
    app.state::<PreviewMemo>()
        .remember(&source, &destination, mode, preview.counts());
    Ok(preview)
}

/// Runs `work` on a blocking thread, so a slow disk never freezes the UI.
async fn blocking<T: Send + 'static, E: From<String> + Send + 'static>(
    work: impl FnOnce() -> Result<T, E> + Send + 'static,
) -> Result<T, E> {
    spawn_blocking(work)
        .await
        .map_err(|e| E::from(e.to_string()))?
}

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
        let runs = history::read(&storage.history_file()).map_err(|e| e.to_string())?;
        let syncs = sync_store::load(&storage.syncs_file()).unwrap_or_default();
        let summary = history_summary::summarize(&runs, &syncs);
        Ok(RunList { runs, summary })
    })
    .await
}

/// The robocopy log of one run; fails when the file is gone or not inside the logs folder.
#[tauri::command]
pub async fn read_run_log(path: String) -> Result<String, String> {
    let storage = Storage::from_env()?;
    blocking(move || {
        history::read_log(&storage.logs_dir(), Path::new(&path)).map_err(|e| e.to_string())
    })
    .await
}

/// A finished Run as the result screen gets it: what is stored, plus what to tell the user when
/// it failed.
#[derive(Serialize)]
pub struct RunReport {
    #[serde(flatten)]
    run: Run,
    explanation: Option<Explanation>,
}

#[tauri::command]
pub async fn run_sync(
    app: AppHandle,
    sync: SavedSync,
    counts: Counts,
) -> Result<RunReport, String> {
    let storage = Storage::from_env()?;
    // The counts of the preview Rust measured count, not the ones the UI sends back.
    let counts = app.state::<PreviewMemo>().take_counts(
        &sync.source,
        &sync.destination,
        sync.mode,
        counts,
    )?;
    let run = spawn_blocking({
        let app = app.clone();
        move || {
            runner::run(
                &storage,
                &sync,
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
    run.map(|run| RunReport {
        explanation: error_messages::explain_run(&run),
        run,
    })
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

#[tauri::command]
pub async fn list_syncs() -> Result<ListedSyncs, String> {
    let storage = Storage::from_env()?;
    blocking(move || {
        sync_store::list(&storage)
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
    runner::default_sync_name(&source)
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

/// The app runs as administrator: mapped network drives are then not visible.
#[tauri::command]
pub fn is_elevated() -> bool {
    elevation::is_elevated()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_messages::{Cause, Folder};
    use crate::history::Outcome;

    #[test]
    fn a_failed_run_goes_to_the_ui_with_its_explanation_next_to_the_stored_fields() {
        let run = Run {
            sync_id: Some(1),
            sync_name: "Foto".into(),
            source: r"D:\Foto".into(),
            destination: r"\\NAS\Backup".into(),
            mode: SyncMode::Copy,
            new: 0,
            updated: 0,
            deleted: 0,
            duration_ms: 1,
            outcome: Outcome::Error,
            started_at: "2026-11-02T21:30:00+01:00".into(),
            log_path: String::new(),
            partial: Some(0),
            cause: Some(Cause {
                code: 53,
                folder: Folder::B,
            }),
        };
        let report = RunReport {
            explanation: error_messages::explain_run(&run),
            run,
        };

        let json = serde_json::to_value(&report).unwrap();

        assert_eq!(json["syncName"], "Foto");
        assert_eq!(json["outcome"], "Error");
        assert_eq!(
            json["explanation"]["title"],
            r"Non completato: la cartella di rete \\NAS non è raggiungibile."
        );
        assert_eq!(
            json["explanation"]["advice"],
            "Controlla la connessione e riprova."
        );
    }
}
