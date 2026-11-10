//! `syncs.json`: stores the saved syncs. What the home screen shows about each is in `syncs/listing.rs`.

use crate::shared::domain::{SavedSync, SyncMode};
use crate::shared::storage::read_if_present;
use chrono::Local;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

/// One writer at a time: `add` reads, changes and rewrites the whole file.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

/// A missing file is no syncs yet. Any other problem (unreadable, corrupt) is an error: the file
/// may still hold the user's syncs, so nobody may treat it as empty and write over it (see
/// `load_or_set_aside`).
pub fn load(file: &Path) -> io::Result<Vec<SavedSync>> {
    let Some(bytes) = read_if_present(file)? else {
        return Ok(Vec::new());
    };
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

/// Writes a temp file next to `file` and renames it over, so a crash never leaves half a file.
pub fn save(file: &Path, syncs: &[SavedSync]) -> io::Result<()> {
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

/// `load_or_set_aside` under the same lock as `add`, for the home screen's listing.
pub fn read_for_listing(file: &Path) -> io::Result<(Vec<SavedSync>, bool)> {
    let _one_writer = WRITE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    load_or_set_aside(file)
}

/// Saves a new sync at the top of the list. A damaged file is set aside first (see
/// `load_or_set_aside`); fails, writing nothing, when that is not possible. The id is above every
/// saved one and above `highest_id_in_history` (0 for none): a set-aside list must not hand out
/// again the id of Runs already in the history.
pub fn add(
    file: &Path,
    highest_id_in_history: u64,
    name: &str,
    source: &str,
    destination: &str,
    mode: SyncMode,
) -> io::Result<SavedSync> {
    let _one_writer = WRITE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let (mut syncs, _) = load_or_set_aside(file)?;
    let highest_taken = syncs
        .iter()
        .map(|s| s.id)
        .fold(highest_id_in_history, u64::max);
    let sync = SavedSync {
        id: highest_taken + 1,
        name: name.trim().to_string(),
        source: source.trim().to_string(),
        destination: destination.trim().to_string(),
        mode,
    };
    syncs.insert(0, sync.clone());
    save(file, &syncs)?;
    Ok(sync)
}

/// "Copia di <last folder of A>"; for a drive root, "Copia di disco D:".
pub fn default_sync_name(source: &str) -> String {
    let trimmed = source.trim().trim_end_matches(['\\', '/']);
    let last = trimmed.rsplit(['\\', '/']).next().unwrap_or(trimmed);
    if last.ends_with(':') {
        format!("Copia di disco {last}")
    } else {
        format!("Copia di {last}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::domain::sample_sync;

    fn sync(id: u64, name: &str) -> SavedSync {
        SavedSync {
            id,
            name: name.into(),
            ..sample_sync()
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

        let added = add(&file, 0, "Foto", r"D:\Foto", r"E:\B", SyncMode::Copy).unwrap();

        assert_eq!(load(&file).unwrap(), vec![added]);
        let copies = damaged_copies(dir.path());
        assert_eq!(copies.len(), 1);
        assert_eq!(copies[0].1, "[ { \"id\": 1, rovinato");
    }

    #[test]
    fn add_puts_the_new_sync_first_with_a_fresh_id() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("syncs.json");

        let first = add(&file, 0, " Foto ", r"D:\Foto ", r"E:\B", SyncMode::Copy).unwrap();
        let second = add(&file, 0, "Doc", r"D:\Doc", r"E:\B", SyncMode::Mirror).unwrap();

        assert_eq!(first.name, "Foto");
        assert_eq!(first.source, r"D:\Foto");
        assert_ne!(first.id, second.id);
        assert_eq!(load(&file).unwrap(), vec![second, first]);
    }

    #[test]
    fn add_takes_an_id_above_the_saved_ones_and_above_the_ids_in_the_history() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("syncs.json");

        let after_history = add(&file, 5, "Foto", r"D:\Foto", r"E:\B", SyncMode::Copy).unwrap();
        let after_saved = add(&file, 2, "Doc", r"D:\Doc", r"E:\B", SyncMode::Copy).unwrap();

        assert_eq!(after_history.id, 6);
        assert_eq!(after_saved.id, 7);
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
                        0,
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

    #[test]
    fn default_name_is_copia_di_last_folder_of_a() {
        assert_eq!(default_sync_name(r"D:\Foto"), "Copia di Foto");
        assert_eq!(default_sync_name(r"D:\Foto\2024\"), "Copia di 2024");
        assert_eq!(default_sync_name(r"\\NAS\Condivisa\Foto"), "Copia di Foto");
        assert_eq!(default_sync_name(r"D:\"), "Copia di disco D:");
    }
}
