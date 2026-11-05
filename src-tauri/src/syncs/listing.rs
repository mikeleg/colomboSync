//! What the home screen lists: each saved sync with the outcome of its latest Run in the history.

use crate::history;
use crate::shared::domain::{Outcome, SavedSync};
use crate::shared::error_messages;
use crate::shared::storage::Storage;
use crate::shared::sync_store;
use serde::Serialize;
use std::collections::HashMap;
use std::io;

/// How the latest Run of a sync ended, for the home card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastRun {
    pub started_at: String,
    pub outcome: Outcome,
    /// What to tell under the card: why it failed, or that it was stopped by hand.
    pub note: Option<String>,
}

/// A saved sync as the home screen lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSummary {
    #[serde(flatten)]
    pub sync: SavedSync,
    pub last_run: Option<LastRun>,
}

/// What the home screen lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedSyncs {
    pub syncs: Vec<SyncSummary>,
    /// `syncs.json` was damaged and has been renamed aside: the list starts again from empty.
    pub set_aside: bool,
}

/// Every saved sync with the outcome of its latest Run in the history (an unreadable history
/// just means no last run).
pub fn list(storage: &Storage) -> io::Result<ListedSyncs> {
    let runs = history::read(&storage.history_file()).unwrap_or_default();
    let last_runs: HashMap<u64, LastRun> = history::latest_per_sync(&runs)
        .into_iter()
        .map(|(id, run)| {
            let last_run = LastRun {
                note: error_messages::card_note(run),
                started_at: run.started_at.clone(),
                outcome: run.outcome,
            };
            (id, last_run)
        })
        .collect();
    let (saved, set_aside) = sync_store::read_for_listing(&storage.syncs_file())?;
    let syncs = saved
        .into_iter()
        .map(|sync| SyncSummary {
            last_run: last_runs.get(&sync.id).cloned(),
            sync,
        })
        .collect();
    Ok(ListedSyncs { syncs, set_aside })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::domain::{Cause, Folder, Run, sample_run, sample_sync};
    use std::fs;
    use std::path::Path;

    fn sync(id: u64, name: &str) -> SavedSync {
        SavedSync {
            id,
            name: name.into(),
            ..sample_sync()
        }
    }

    fn run(sync: &SavedSync, outcome: Outcome, started_at: &str) -> Run {
        Run {
            sync_id: Some(sync.id),
            sync_name: sync.name.clone(),
            source: sync.source.clone(),
            destination: sync.destination.clone(),
            mode: sync.mode,
            outcome,
            started_at: started_at.into(),
            ..sample_run()
        }
    }

    /// The damaged copies set aside next to `syncs.json`, with their content.
    fn damaged_copies(dir: &Path) -> Vec<(String, String)> {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("syncs.json.corrotto-")
            })
            .map(|p| {
                (
                    p.file_name().unwrap().to_string_lossy().into_owned(),
                    fs::read_to_string(&p).unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn list_shows_the_latest_run_of_each_sync_and_none_for_a_sync_never_run() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::at(dir.path());
        let (foto, doc) = (sync(1, "Foto"), sync(2, "Documenti"));
        sync_store::save(&storage.syncs_file(), &[doc.clone(), foto.clone()]).unwrap();
        history::append(
            &storage.history_file(),
            &run(&foto, Outcome::Error, "2026-11-01T10:00:00+01:00"),
        )
        .unwrap();
        history::append(
            &storage.history_file(),
            &run(&foto, Outcome::Ok, "2026-11-02T21:30:00+01:00"),
        )
        .unwrap();

        let listed = list(&storage).unwrap().syncs;

        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].sync, doc);
        assert_eq!(listed[0].last_run, None);
        assert_eq!(
            listed[1].last_run,
            Some(LastRun {
                started_at: "2026-11-02T21:30:00+01:00".into(),
                outcome: Outcome::Ok,
                note: None
            })
        );
    }

    #[test]
    fn the_card_note_says_why_the_last_run_failed_or_that_it_was_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::at(dir.path());
        let (failed, stopped, plain, done) = (
            sync(1, "Rete"),
            sync(2, "Fermata"),
            sync(3, "Vecchia"),
            sync(4, "Fatta"),
        );
        sync_store::save(
            &storage.syncs_file(),
            &[failed.clone(), stopped.clone(), plain.clone(), done.clone()],
        )
        .unwrap();
        let at = "2026-11-02T21:30:00+01:00";
        for r in [
            Run {
                cause: Some(Cause {
                    code: 53,
                    folder: Folder::B,
                }),
                ..run(&failed, Outcome::Error, at)
            },
            run(&stopped, Outcome::Cancelled, at),
            run(&plain, Outcome::Error, at), // written before S6: no note stored
            run(&done, Outcome::Ok, at),
        ] {
            history::append(&storage.history_file(), &r).unwrap();
        }

        let notes: Vec<Option<String>> = list(&storage)
            .unwrap()
            .syncs
            .into_iter()
            .map(|s| s.last_run.unwrap().note)
            .collect();

        assert_eq!(
            notes,
            [
                Some(r"La cartella di rete \\NAS non era raggiungibile. Controlla che sia collegato e riprova.".to_string()),
                Some(error_messages::STOPPED_NOTE.to_string()),
                Some(error_messages::GENERIC_NOTE.to_string()),
                None
            ]
        );
    }

    #[test]
    fn a_run_is_matched_by_sync_id_not_by_name_and_paths() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::at(dir.path());
        let (first, twin) = (sync(1, "Foto"), sync(2, "Foto")); // same name and paths
        sync_store::save(&storage.syncs_file(), &[twin.clone(), first.clone()]).unwrap();
        let from_before_ids = Run {
            sync_id: None,
            ..run(&first, Outcome::Ok, "2026-10-01T10:00:00+01:00")
        };
        history::append(&storage.history_file(), &from_before_ids).unwrap();
        history::append(
            &storage.history_file(),
            &run(&first, Outcome::Error, "2026-11-02T21:30:00+01:00"),
        )
        .unwrap();

        let listed = list(&storage).unwrap().syncs;

        assert_eq!(listed[0].sync, twin);
        assert_eq!(listed[0].last_run, None, "the twin was never run");
        assert_eq!(listed[1].last_run.as_ref().unwrap().outcome, Outcome::Error);
    }

    #[test]
    fn list_without_files_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let listed = list(&Storage::at(dir.path())).unwrap();

        assert!(listed.syncs.is_empty());
        assert!(!listed.set_aside);
    }

    #[test]
    fn corrupt_file_is_set_aside_and_the_list_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::at(dir.path());
        fs::write(storage.syncs_file(), "rovinato").unwrap();

        let listed = list(&storage).unwrap();

        assert!(listed.syncs.is_empty());
        assert!(listed.set_aside);
        assert!(!storage.syncs_file().exists());
        let copies = damaged_copies(dir.path());
        assert_eq!(copies.len(), 1);
        assert_eq!(copies[0].1, "rovinato");
        // Named after the moment, yyyyMMdd-HHmmss.
        let stamp = copies[0].0.strip_prefix("syncs.json.corrotto-").unwrap();
        assert!(chrono::NaiveDateTime::parse_from_str(stamp, "%Y%m%d-%H%M%S").is_ok());
        // The next listing finds nothing damaged any more.
        assert!(!list(&storage).unwrap().set_aside);
    }

    #[test]
    fn summary_is_flat_camel_case_json() {
        let summary = SyncSummary {
            sync: sync(1, "Foto"),
            last_run: Some(LastRun {
                started_at: "2026-11-02T21:30:00+01:00".into(),
                outcome: Outcome::Error,
                note: None,
            }),
        };
        assert_eq!(
            serde_json::to_string(&summary).unwrap(),
            r#"{"id":1,"name":"Foto","source":"D:\\Foto","destination":"\\\\NAS\\Backup","mode":"Copy","lastRun":{"startedAt":"2026-11-02T21:30:00+01:00","outcome":"Error","note":null}}"#
        );
    }
}
