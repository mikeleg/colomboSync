//! Thin `#[tauri::command]` wrappers for a Run: the engine runs on a blocking thread so the UI
//! never freezes.

use super::runner::{self, ActiveRun};
use crate::engine::preview_parser::Counts;
use crate::shared::blocking::blocking;
use crate::shared::domain::{RunReport, SavedSync};
use crate::shared::storage::Storage;
use crate::wizard::preview::PreviewMemo;
use tauri::{AppHandle, Emitter, Manager, State};

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
    let run = blocking({
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
    .await;
    // The user confirmed closing the window during the run: the history line is already written.
    if app.state::<ActiveRun>().exit_when_done() {
        app.exit(0);
    }
    run.map(RunReport::from)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::domain::{Cause, Folder, Outcome, Run, sample_run};

    #[test]
    fn a_failed_run_goes_to_the_ui_with_its_explanation_next_to_the_stored_fields() {
        let run = Run {
            sync_id: Some(1),
            destination: r"\\NAS\Backup".into(),
            outcome: Outcome::Error,
            partial: Some(0),
            cause: Some(Cause {
                code: 53,
                folder: Folder::B,
            }),
            ..sample_run()
        };
        let json = serde_json::to_value(RunReport::from(run)).unwrap();

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
