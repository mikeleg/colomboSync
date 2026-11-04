//! `syncs.json`: the saved syncs, and what the home screen shows about each.

use crate::error_messages;
use crate::history::{self, Outcome};
use crate::robocopy::SyncMode;
use crate::storage::Storage;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

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

/// One writer at a time: `add` reads, changes and rewrites the whole file.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

/// A missing file is no syncs yet. Any other problem (unreadable, corrupt) is an error: the file
/// may still hold the user's syncs, so nobody may treat it as empty and write over it (see
/// `load_or_set_aside`).
pub fn load(file: &Path) -> io::Result<Vec<SavedSync>> {
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    serde_json::from_str(&text).map_err(io::Error::other)
}

/// Writes a temp file next to `file` and renames it over, so a crash never leaves half a file.
fn save(file: &Path, syncs: &[SavedSync]) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(syncs).map_err(io::Error::other)?;
    let temp = file.with_extension("json.tmp");
    fs::write(&temp, text)?;
    fs::rename(&temp, file).inspect_err(|_| {
        let _ = fs::remove_file(&temp);
    })
}

/// Like `load`, but a file that exists and cannot be read is renamed to
/// `syncs.json.corrotto-<yyyyMMdd-HHmmss>` and the list is empty (`true`: it was set aside).
/// The damaged content is never overwritten; if it cannot be moved aside, that is the error.
fn load_or_set_aside(file: &Path) -> io::Result<(Vec<SavedSync>, bool)> {
    if let Ok(syncs) = load(file) {
        return Ok((syncs, false));
    }
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    fs::rename(
        file,
        file.with_file_name(format!("{name}.corrotto-{stamp}")),
    )?;
    Ok((Vec::new(), true))
}

/// Saves a new sync at the top of the list. A damaged file is set aside first (see
/// `load_or_set_aside`); fails, writing nothing, when that is not possible.
pub fn add(
    file: &Path,
    name: &str,
    source: &str,
    destination: &str,
    mode: SyncMode,
) -> io::Result<SavedSync> {
    let _one_writer = WRITE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let (mut syncs, _) = load_or_set_aside(file)?;
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
    let mut last_runs: HashMap<u64, LastRun> = HashMap::new();
    // Newest first: the first run seen for a sync is its latest.
    for run in history::read(&storage.history_file()).unwrap_or_default() {
        if let Some(id) = run.sync_id
            && let Entry::Vacant(slot) = last_runs.entry(id)
        {
            slot.insert(LastRun {
                note: error_messages::card_note(&run),
                started_at: run.started_at,
                outcome: run.outcome,
            });
        }
    }
    let (saved, set_aside) = {
        let _one_writer = WRITE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        load_or_set_aside(&storage.syncs_file())?
    };
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
    use crate::error_messages::{Cause, Folder};
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

        assert_eq!(load(&file).unwrap(), syncs);
    }

    #[test]
    fn save_replaces_the_file_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("syncs.json");

        save(&file, &[sync(1, "Prima")]).unwrap();
        save(&file, &[sync(2, "Dopo")]).unwrap();

        assert_eq!(load(&file).unwrap(), vec![sync(2, "Dopo")]);
        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["syncs.json"]);
    }

    #[test]
    fn a_missing_file_is_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(&dir.path().join("syncs.json")).unwrap().is_empty());
    }

    #[test]
    fn corrupt_file_is_an_error_not_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("syncs.json");

        fs::write(&file, "{ questo non è json").unwrap();
        assert!(load(&file).is_err(), "corrupt file");
        fs::write(&file, "").unwrap();
        assert!(load(&file).is_err(), "empty file");
        // Not a file at all: any read error other than "missing" is an error too.
        fs::remove_file(&file).unwrap();
        fs::create_dir(&file).unwrap();
        assert!(load(&file).is_err(), "unreadable");
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
    fn add_after_a_corrupt_file_keeps_the_damaged_copy() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("syncs.json");
        fs::write(&file, "[ { \"id\": 1, rovinato").unwrap();

        let added = add(&file, "Foto", r"D:\Foto", r"E:\B", SyncMode::Copy).unwrap();

        assert_eq!(load(&file).unwrap(), vec![added]);
        let copies = damaged_copies(dir.path());
        assert_eq!(copies.len(), 1);
        assert_eq!(copies[0].1, "[ { \"id\": 1, rovinato");
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
        assert_eq!(load(&file).unwrap(), vec![second, first]);
    }

    #[test]
    fn concurrent_adds_are_all_kept_with_distinct_ids() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("syncs.json");

        let handles: Vec<_> = (0..8)
            .map(|i| {
                let file = file.clone();
                std::thread::spawn(move || {
                    add(
                        &file,
                        &format!("Sync {i}"),
                        r"D:\Foto",
                        r"E:\B",
                        SyncMode::Copy,
                    )
                    .unwrap()
                })
            })
            .collect();
        let added: Vec<SavedSync> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        let mut ids: Vec<u64> = added.iter().map(|s| s.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 8, "distinct ids");
        assert_eq!(load(&file).unwrap().len(), 8, "no add was lost");
    }

    fn run(sync: &SavedSync, outcome: Outcome, started_at: &str) -> Run {
        Run {
            sync_id: Some(sync.id),
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
            partial: None,
            cause: None,
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
        save(
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
        save(&storage.syncs_file(), &[twin.clone(), first.clone()]).unwrap();
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
