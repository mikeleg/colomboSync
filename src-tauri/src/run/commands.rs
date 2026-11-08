//! Thin `#[tauri::command]` wrappers for a Run: the engine runs on a blocking thread so the UI
//! never freezes.

use super::runner::{self, ActiveRun};
use crate::shared::blocking::blocking;
use crate::shared::domain::{RunReport, SavedSync};
use crate::shared::error_messages::Explanation;
use crate::shared::storage::Storage;
use crate::wizard::preview::PreviewMemo;
use tauri::{AppHandle, Emitter, Manager, State};

/// Runs `sync` as the preview whose `preview_token` `preview_sync` gave confirms it. What the UI
/// sends is only a claim: the counts are the ones Rust measured in that preview, and the run is
/// refused with an `Explanation` unless the token is the last preview's, for this A, B and mode,
/// and `consent` is given when a mirror would delete from B.
#[tauri::command]
pub async fn run_sync(
    app: AppHandle,
    sync: SavedSync,
    preview_token: Option<String>,
    consent: bool,
) -> Result<RunReport, Explanation> {
    let storage = Storage::from_env()?;
    let counts = app
        .state::<PreviewMemo>()
        .confirm(preview_token.as_deref(), &sync, consent)?;
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
        assert_eq!(json["title"], json["explanation"]["title"]);
        assert_eq!(json["caption"], "Non completato.");
        assert_eq!(json["resultNote"], "Nessun file è stato copiato.");
        assert_eq!(json["partialText"], "0 file copiati (parziale)");
    }

    #[test]
    fn a_run_goes_to_the_ui_with_its_texts_made_and_its_stored_fields_untouched() {
        let json = serde_json::to_value(RunReport::from(sample_run())).unwrap();

        assert_eq!(
            json["title"],
            "Fatto: 12 file copiati, 1 aggiornato in 1 min"
        );
        assert_eq!(json["caption"], "Fatto.");
        assert_eq!(json["when"], "02/11/2026 21:30");
        assert_eq!(json["durationText"], "1 min");
        assert!(json["resultNote"].is_null());
        assert!(json["partialText"].is_null());
        assert_eq!(json["startedAt"], "2026-11-02T21:30:00+01:00");
        assert_eq!(json["durationMs"], 61_000);
    }

    #[test]
    fn a_stopped_run_says_what_stays_in_b() {
        let run = Run {
            outcome: Outcome::Cancelled,
            partial: Some(40),
            ..sample_run()
        };
        let json = serde_json::to_value(RunReport::from(run)).unwrap();

        assert_eq!(json["title"], "Annullato: ho fermato la copia.");
        assert_eq!(json["caption"], "Annullato.");
        assert_eq!(
            json["resultNote"],
            "40 file copiati (parziale). I file già copiati restano in B."
        );
        assert_eq!(json["partialText"], "40 file copiati (parziale)");
    }
}
