//! Runs the real `robocopy.exe` on temp folders.

use colombo_sync_lib::history::{Outcome, Run};
use colombo_sync_lib::robocopy::SyncMode;
use colombo_sync_lib::runner::{self, ActiveRun};
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
        &ActiveRun::default(),
        |_| {},
    )
    .unwrap();

    assert_eq!(run.outcome, Outcome::Ok);
    assert_eq!(run.partial, None, "an Ok run keeps the preview counts");
    assert_eq!((run.new, run.updated, run.deleted), (2, 1, 0));
    assert_eq!(run.sync_name, "Copia di A");
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
        &ActiveRun::default(),
        |_| {},
    )
    .unwrap();

    assert_eq!(run.outcome, Outcome::Error);
    assert!(run.error.is_some());
    assert_eq!(run.partial, Some(0), "an Error run keeps a partial count");
    assert_eq!(
        fs::read_to_string(storage.history_file())
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn run_reports_progress_with_the_current_file() {
    let temp = tempfile::tempdir().unwrap();
    let (a, b) = (temp.path().join("A"), temp.path().join("B"));
    write(&a.join("è già.txt"), "uno");
    write(&a.join("due.txt"), "due");
    write(&a.join("sub").join("tre.txt"), "tre");
    write(&b.join("only-in-b.txt"), "extra");
    let storage = Storage::at(temp.path().join("ColomboSync"));
    let counts = runner::preview(path_str(&a), path_str(&b), SyncMode::Copy)
        .unwrap()
        .counts();

    let mut events = Vec::new();
    let run = runner::run(
        &storage,
        path_str(&a),
        path_str(&b),
        SyncMode::Copy,
        counts,
        &ActiveRun::default(),
        |progress| events.push(progress.clone()),
    )
    .unwrap();

    assert_eq!(run.outcome, Outcome::Ok);
    let last = events.last().expect("at least one progress event");
    assert_eq!(
        (last.done, last.total),
        (3, 3),
        "extras in B are not counted"
    );
    assert_eq!(last.sync_name, "Copia di A");
    let seen: Vec<&str> = events.iter().map(|e| e.current_file.as_str()).collect();
    assert!(
        seen.contains(&"è già.txt"),
        "file names are readable: {seen:?}"
    );
    assert!(events.windows(2).all(|w| w[0].done <= w[1].done));
}

#[test]
fn cancel_kills_robocopy_and_records_a_cancelled_run() {
    let temp = tempfile::tempdir().unwrap();
    let (a, b) = (temp.path().join("A"), temp.path().join("B"));
    for i in 0..20 {
        write(&a.join(format!("a{i:02}.txt")), "piccolo");
    }
    // Sparse file: instant to create, long enough to copy for the cancel to land mid-copy.
    fs::create_dir_all(&a).unwrap();
    fs::File::create(a.join("z-grande.bin"))
        .unwrap()
        .set_len(3_000_000_000)
        .unwrap();
    let storage = Storage::at(temp.path().join("ColomboSync"));
    let counts = runner::preview(path_str(&a), path_str(&b), SyncMode::Copy)
        .unwrap()
        .counts();
    let active = ActiveRun::default();

    let run = runner::run(
        &storage,
        path_str(&a),
        path_str(&b),
        SyncMode::Copy,
        counts,
        &active,
        |progress| {
            if progress.done >= 1 {
                assert!(active.cancel(false));
            }
        },
    )
    .unwrap();

    assert_eq!(
        run.outcome,
        Outcome::Cancelled,
        "exit code 1 must not read as Ok"
    );
    assert_eq!((run.new, run.updated, run.deleted), (0, 0, 0));
    assert!(run.error.is_none());
    assert!(run.partial.is_some());
    assert!(!active.is_running(), "the robocopy process is gone");
    assert!(
        !b.join("z-grande.bin").exists()
            || fs::metadata(b.join("z-grande.bin")).unwrap().len() < 3_000_000_000,
        "the big file was not copied completely"
    );
    let history = fs::read_to_string(storage.history_file()).unwrap();
    let recorded: Run = serde_json::from_str(history.trim()).unwrap();
    assert_eq!(recorded, run);
    assert!(!active.cancel(false), "nothing left to cancel");
}
