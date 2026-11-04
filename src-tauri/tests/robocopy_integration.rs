//! Runs the real `robocopy.exe` on temp folders.

use colombo_sync_lib::history::{Outcome, Run};
use colombo_sync_lib::robocopy::SyncMode;
use colombo_sync_lib::runner;
use colombo_sync_lib::storage::Storage;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn path_str(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn copy_keeps_extra_files_in_destination() {
    let temp = tempfile::tempdir().unwrap();
    let (a, b) = (temp.path().join("A"), temp.path().join("B"));
    write(&a.join("new.txt"), "nuovo");
    write(&a.join("sub").join("deep.txt"), "x");
    write(&a.join("updated.txt"), "versione nuova lunga");
    write(&a.join("same.txt"), "uguale");
    write(&b.join("updated.txt"), "old");
    fs::File::options()
        .write(true)
        .open(b.join("updated.txt"))
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(86_400 * 365))
        .unwrap();
    fs::copy(a.join("same.txt"), b.join("same.txt")).unwrap();
    write(&b.join("only-in-b.txt"), "extra");
    write(&b.join("sub").join("only-in-b-too.txt"), "extra");
    let storage = Storage::at(temp.path().join("ColomboSync"));

    let preview = runner::preview(path_str(&a), path_str(&b), SyncMode::Copy).unwrap();

    assert_eq!(preview.new.files, ["new.txt", r"sub\deep.txt"]);
    assert_eq!(preview.updated.files, ["updated.txt"]);
    assert_eq!(preview.deleted.count, 0, "extras in B are not counted");
    assert!(
        !format!("{preview:?}").contains("only-in-b"),
        "extras in B are not listed anywhere"
    );
    assert!(!b.join("new.txt").exists(), "a preview touches nothing");

    let run = runner::run(
        &storage,
        path_str(&a),
        path_str(&b),
        SyncMode::Copy,
        preview.counts(),
        "Le foto di Anna".into(),
    )
    .unwrap();

    assert_eq!(run.outcome, Outcome::Ok);
    assert_eq!((run.new, run.updated, run.deleted), (2, 1, 0));
    assert_eq!(
        run.sync_name, "Le foto di Anna",
        "the run carries the sync's name"
    );
    assert_eq!(fs::read_to_string(b.join("new.txt")).unwrap(), "nuovo");
    assert_eq!(
        fs::read_to_string(b.join("updated.txt")).unwrap(),
        "versione nuova lunga"
    );
    assert!(b.join("only-in-b.txt").exists());
    assert!(b.join("sub").join("only-in-b-too.txt").exists());

    assert!(Path::new(&run.log_path).is_file(), "robocopy log exists");
    assert!(run.log_path.starts_with(path_str(&storage.logs_dir())));
    let history = fs::read_to_string(storage.history_file()).unwrap();
    assert_eq!(history.lines().count(), 1);
    assert_eq!(serde_json::from_str::<Run>(history.trim()).unwrap(), run);
}

#[test]
fn robocopy_failure_is_recorded_as_error() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("non-esiste");
    let b = temp.path().join("B");
    let storage = Storage::at(temp.path().join("ColomboSync"));

    let preview = runner::preview(path_str(&missing), path_str(&b), SyncMode::Copy);
    assert!(preview.is_err());

    let run = runner::run(
        &storage,
        path_str(&missing),
        path_str(&b),
        SyncMode::Copy,
        Default::default(),
        "Copia di non-esiste".into(),
    )
    .unwrap();

    assert_eq!(run.outcome, Outcome::Error);
    assert!(run.error.is_some());
    assert_eq!(
        fs::read_to_string(storage.history_file())
            .unwrap()
            .lines()
            .count(),
        1
    );
}
