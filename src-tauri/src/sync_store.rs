//! `syncs.json`: the saved syncs, and what the home screen shows about each.

use crate::history::{self, Outcome};
use crate::robocopy::SyncMode;
use crate::storage::Storage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// A saved A→B pair (the glossary's `Sync`; not named so to avoid shadowing `std::marker::Sync`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSync {
    pub id: u64,
    pub name: String,
    pub source: String,
    pub destination: String,
    pub mode: SyncMode,
}

/// How the latest Run of a sync ended, for the home card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastRun {
    pub started_at: String,
    pub outcome: Outcome,
}

/// A saved sync as the home screen lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSummary {
    #[serde(flatten)]
    pub sync: SavedSync,
    pub last_run: Option<LastRun>,
}

/// Missing, unreadable or corrupt file: no syncs (the home shows its empty state).
pub fn load(file: &Path) -> Vec<SavedSync> {
    fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(file: &Path, syncs: &[SavedSync]) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(syncs).map_err(io::Error::other)?;
    fs::write(file, text)
}

/// Saves a new sync at the top of the list.
pub fn add(
    file: &Path,
    name: &str,
    source: &str,
    destination: &str,
    mode: SyncMode,
) -> io::Result<SavedSync> {
    let mut syncs = load(file);
    let sync = SavedSync {
        id: syncs.iter().map(|s| s.id).max().map_or(1, |id| id + 1),
        name: name.trim().to_string(),
        source: source.trim().to_string(),
        destination: destination.trim().to_string(),
        mode,
    };
    syncs.insert(0, sync.clone());
    save(file, &syncs)?;
    Ok(sync)
}

/// Every saved sync with the outcome of its latest Run in the history (an unreadable history
/// just means no last run).
pub fn list(storage: &Storage) -> Vec<SyncSummary> {
    let runs = history::read(&storage.history_file()).unwrap_or_default(); // newest first
    load(&storage.syncs_file())
        .into_iter()
        .map(|sync| {
            let last_run = runs
                .iter()
                .find(|r| {
                    r.sync_name == sync.name
                        && r.source == sync.source
                        && r.destination == sync.destination
                })
                .map(|r| LastRun {
                    started_at: r.started_at.clone(),
                    outcome: r.outcome,
                });
            SyncSummary { sync, last_run }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::Run;

    fn sync(id: u64, name: &str) -> SavedSync {
        SavedSync {
            id,
            name: name.into(),
            source: r"D:\Foto".into(),
            destination: r"\\NAS\Backup".into(),
            mode: SyncMode::Copy,
        }
    }

    #[test]
    fn round_trips_syncs() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("ColomboSync").join("syncs.json");
        let syncs = vec![sync(2, "Copia di Foto"), sync(1, "Documenti")];

        save(&file, &syncs).unwrap();

        assert_eq!(load(&file), syncs);
    }

    #[test]
    fn corrupt_file_returns_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("syncs.json");

        assert!(load(&file).is_empty(), "missing file");
        fs::write(&file, "{ questo non è json").unwrap();
        assert!(load(&file).is_empty(), "corrupt file");
        fs::write(&file, "").unwrap();
        assert!(load(&file).is_empty(), "empty file");
    }

    #[test]
    fn add_puts_the_new_sync_first_with_a_fresh_id() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("syncs.json");

        let first = add(&file, " Foto ", r"D:\Foto ", r"E:\B", SyncMode::Copy).unwrap();
        let second = add(&file, "Doc", r"D:\Doc", r"E:\B", SyncMode::Mirror).unwrap();

        assert_eq!(first.name, "Foto");
        assert_eq!(first.source, r"D:\Foto");
        assert_ne!(first.id, second.id);
        assert_eq!(load(&file), vec![second, first]);
    }

    fn run(sync: &SavedSync, outcome: Outcome, started_at: &str) -> Run {
        Run {
            sync_name: sync.name.clone(),
            source: sync.source.clone(),
            destination: sync.destination.clone(),
            mode: sync.mode,
            new: 1,
            updated: 0,
            deleted: 0,
            duration_ms: 1,
            outcome,
            started_at: started_at.into(),
            log_path: String::new(),
            error: None,
            partial: None,
        }
    }

    #[test]
    fn list_shows_the_latest_run_of_each_sync_and_none_for_a_sync_never_run() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::at(dir.path());
        let (foto, doc) = (sync(1, "Foto"), sync(2, "Documenti"));
        save(&storage.syncs_file(), &[doc.clone(), foto.clone()]).unwrap();
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

        let listed = list(&storage);

        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].sync, doc);
        assert_eq!(listed[0].last_run, None);
        assert_eq!(
            listed[1].last_run,
            Some(LastRun {
                started_at: "2026-11-02T21:30:00+01:00".into(),
                outcome: Outcome::Ok
            })
        );
    }

    #[test]
    fn list_without_files_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list(&Storage::at(dir.path().join("niente"))).is_empty());
    }

    #[test]
    fn summary_is_flat_camel_case_json() {
        let summary = SyncSummary {
            sync: sync(1, "Foto"),
            last_run: Some(LastRun {
                started_at: "2026-11-02T21:30:00+01:00".into(),
                outcome: Outcome::Error,
            }),
        };
        assert_eq!(
            serde_json::to_string(&summary).unwrap(),
            r#"{"id":1,"name":"Foto","source":"D:\\Foto","destination":"\\\\NAS\\Backup","mode":"Copy","lastRun":{"startedAt":"2026-11-02T21:30:00+01:00","outcome":"Error"}}"#
        );
    }
}
