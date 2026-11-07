//! Thin `#[tauri::command]` wrappers for the Storico screen.

use super::summary::{self, HistorySummary};
use super::{Run, RunLog, read, read_log};
use crate::shared::blocking::blocking;
use crate::shared::domain::{RunReport, SavedSync};
use crate::shared::storage::Storage;
use crate::shared::sync_store;
use serde::Serialize;
use std::path::Path;

/// What the Storico screen shows: the runs, newest first (each with the explanation of its failure,
/// which the log window shows), and the totals and per-sync state for its side panel.
#[derive(Serialize)]
pub struct RunList {
    runs: Vec<RunReport>,
    summary: HistorySummary,
}

fn run_list(runs: Vec<Run>, syncs: &[SavedSync]) -> RunList {
    RunList {
        summary: summary::summarize(&runs, syncs),
        runs: runs.into_iter().map(RunReport::from).collect(),
    }
}

/// A `syncs.json` that cannot be read only means the syncs keep the names they had in their runs.
#[tauri::command]
pub async fn list_runs() -> Result<RunList, String> {
    let storage = Storage::from_env()?;
    blocking(move || {
        let runs = read(&storage.history_file()).map_err(|e| e.to_string())?;
        let syncs = sync_store::load(&storage.syncs_file()).unwrap_or_default();
        Ok(run_list(runs, &syncs))
    })
    .await
}

/// The engine log of one run with its error lines; fails when the file is gone or not inside the
/// logs folder.
#[tauri::command]
pub async fn read_run_log(path: String) -> Result<RunLog, String> {
    let storage = Storage::from_env()?;
    blocking(move || read_log(&storage.logs_dir(), Path::new(&path)).map_err(|e| e.to_string()))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::domain::{Cause, Folder, Outcome, sample_run};

    #[test]
    fn a_failed_run_in_the_list_carries_its_explanation() {
        let failed = Run {
            destination: r"\\NAS\Backup".into(),
            outcome: Outcome::Error,
            cause: Some(Cause {
                code: 53,
                folder: Folder::B,
            }),
            ..sample_run()
        };

        let json = serde_json::to_value(run_list(vec![failed, sample_run()], &[])).unwrap();

        assert_eq!(json["runs"][0]["syncName"], "Foto");
        assert_eq!(
            json["runs"][0]["explanation"]["title"],
            r"Non completato: la cartella di rete \\NAS non è raggiungibile."
        );
        assert_eq!(
            json["runs"][0]["explanation"]["advice"],
            "Controlla la connessione e riprova."
        );
        assert!(json["runs"][1]["explanation"].is_null());
        assert_eq!(json["summary"]["runs"], 2);
    }
}
