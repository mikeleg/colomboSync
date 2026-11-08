//! Runs the real `rsync` on temp folders (macOS and Linux CI runners).
#![cfg(unix)]

mod common;

use colombo_sync_lib::engine::preview_parser::Counts;
use colombo_sync_lib::history;
use colombo_sync_lib::run::runner::{self, ActiveRun};
use colombo_sync_lib::shared::domain::{Cause, Folder, Outcome, Run, SyncMode};
use colombo_sync_lib::shared::error_messages;
use colombo_sync_lib::shared::mirror_guard;
use colombo_sync_lib::wizard;
use common::{path_str, run_mirror, saved, set_modified, storage_in, write};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// A copy of a file of A in B that rsync finds identical (same size, same modification time).
fn identical_in(b: &Path, a: &Path, relative: &str) {
    fs::create_dir_all(b.join(relative).parent().unwrap()).unwrap();
    fs::copy(a.join(relative), b.join(relative)).unwrap();
    set_modified(
        &b.join(relative),
        fs::metadata(a.join(relative)).unwrap().modified().unwrap(),
    );
}

fn sorted(files: &[String]) -> Vec<String> {
    let mut files = files.to_vec();
    files.sort();
    files
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
    set_modified(
        &b.join("updated.txt"),
        SystemTime::now() - Duration::from_secs(86_400 * 365),
    );
    identical_in(&b, &a, "same.txt");
    write(&b.join("only-in-b.txt"), "extra");
    write(&b.join("sub").join("only-in-b-too.txt"), "extra");
    let storage = storage_in(temp.path());

    let preview = wizard::preview::preview(path_str(&a), path_str(&b), SyncMode::Copy).unwrap();

    assert_eq!(sorted(&preview.new.files), ["new.txt", "sub/deep.txt"]);
    assert_eq!(preview.updated.files, ["updated.txt"]);
    assert_eq!(preview.deleted.count, 0, "extras in B are not counted");
    assert_eq!(preview.bytes, 5 + 1 + 20, "size of what would be copied");
    assert!(
        !format!("{preview:?}").contains("only-in-b"),
        "extras in B are not listed anywhere"
    );
    assert!(!b.join("new.txt").exists(), "a preview touches nothing");
    assert_eq!(fs::read_to_string(b.join("updated.txt")).unwrap(), "old");

    let run = runner::run(
        &storage,
        &saved("Le foto di Anna", &a, &b),
        preview.counts(),
        &ActiveRun::default(),
        |_| {},
    )
    .unwrap();

    assert_eq!(run.outcome, Outcome::Ok);
    assert_eq!(run.partial, None, "an Ok run keeps the preview counts");
    assert_eq!((run.new, run.updated, run.deleted), (2, 1, 0));
    assert_eq!(run.sync_name, "Le foto di Anna");
    assert_eq!(run.sync_id, Some(7), "the run points at its saved sync");
    assert_eq!(fs::read_to_string(b.join("new.txt")).unwrap(), "nuovo");
    assert_eq!(
        fs::read_to_string(b.join("updated.txt")).unwrap(),
        "versione nuova lunga"
    );
    assert!(b.join("only-in-b.txt").exists());
    assert!(b.join("sub").join("only-in-b-too.txt").exists());

    assert!(Path::new(&run.log_path).is_file(), "the run log exists");
    assert!(run.log_path.starts_with(path_str(&storage.logs_dir())));
    let history = fs::read_to_string(storage.history_file()).unwrap();
    assert_eq!(history.lines().count(), 1);
    assert_eq!(serde_json::from_str::<Run>(history.trim()).unwrap(), run);

    // Nothing is left to do: the second preview is empty.
    let again = wizard::preview::preview(path_str(&a), path_str(&b), SyncMode::Copy).unwrap();
    assert_eq!(again.counts(), Counts::default());
}

#[test]
fn a_destination_that_does_not_exist_yet_previews_everything_as_new_and_stays_untouched() {
    let temp = tempfile::tempdir().unwrap();
    let (a, b) = (temp.path().join("A"), temp.path().join("B"));
    write(&a.join("uno.txt"), "uno");
    write(&a.join("sub").join("due.txt"), "due");

    for mode in [SyncMode::Copy, SyncMode::Mirror] {
        let preview = wizard::preview::preview(path_str(&a), path_str(&b), mode).unwrap();
        assert_eq!(sorted(&preview.new.files), ["sub/due.txt", "uno.txt"]);
        assert_eq!(preview.updated.count, 0);
        assert!(!b.exists(), "a preview creates nothing");
    }

    // The first run creates B itself: not a failure, not a "folder not found".
    let storage = storage_in(temp.path());
    let counts = wizard::preview::preview(path_str(&a), path_str(&b), SyncMode::Copy)
        .unwrap()
        .counts();
    let run = runner::run(
        &storage,
        &saved("Copia di A", &a, &b),
        counts,
        &ActiveRun::default(),
        |_| {},
    )
    .unwrap();
    assert_eq!(run.outcome, Outcome::Ok);
    assert_eq!(
        fs::read_to_string(b.join("sub").join("due.txt")).unwrap(),
        "due"
    );
}

#[test]
fn a_destination_whose_parent_is_missing_is_reported_and_never_created() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("A");
    let b = temp.path().join("Backup").join("Foto"); // `Backup` is an unplugged disk
    write(&a.join("uno.txt"), "uno");
    let storage = storage_in(temp.path());

    let failure = wizard::preview::preview(path_str(&a), path_str(&b), SyncMode::Copy).unwrap_err();
    assert_eq!(
        failure.title,
        format!("Non trovo la cartella B ({}).", path_str(&b))
    );
    assert_eq!(failure.advice, "Il disco è collegato?");

    // A run does not start rsync on it, and is recorded like any other failed run.
    let run = runner::run(
        &storage,
        &saved("Copia di A", &a, &b),
        Default::default(),
        &ActiveRun::default(),
        |_| {},
    )
    .unwrap();
    assert_eq!(run.outcome, Outcome::Error);
    assert_eq!(
        run.cause,
        Some(Cause {
            code: 3,
            folder: Folder::B
        })
    );
    assert_eq!(run.partial, Some(0));
    assert_eq!(
        error_messages::explain_run(&run).unwrap().title,
        format!(
            "Non completato: non trovo la cartella B ({}).",
            path_str(&b)
        )
    );
    let history = fs::read_to_string(storage.history_file()).unwrap();
    assert_eq!(serde_json::from_str::<Run>(history.trim()).unwrap(), run);
    assert!(!temp.path().join("Backup").exists(), "nothing was created");
}

#[test]
fn an_rsync_failure_is_recorded_as_error() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("non-esiste");
    let b = temp.path().join("B");
    let storage = storage_in(temp.path());

    let a = path_str(&missing);
    let failure = wizard::preview::preview(a, path_str(&b), SyncMode::Copy).unwrap_err();
    assert_eq!(failure.title, format!("Non trovo la cartella A ({a})."));
    assert_eq!(failure.advice, "Il disco è collegato?");

    let run = runner::run(
        &storage,
        &saved("Copia di non-esiste", &missing, &b),
        Default::default(),
        &ActiveRun::default(),
        |_| {},
    )
    .unwrap();

    assert_eq!(run.outcome, Outcome::Error);
    // Mapped from rsync's exit code (23) and the state of the folders, never from its text.
    assert_eq!(
        run.cause,
        Some(Cause {
            code: 3,
            folder: Folder::A
        })
    );
    assert_eq!(
        error_messages::explain_run(&run).unwrap().title,
        format!("Non completato: non trovo la cartella A ({a}).")
    );
    assert_eq!(run.partial, Some(0), "an Error run keeps a partial count");
    // What rsync says under its own name (`rsync: …`, `rsync(<pid>): error: …`) is marked as error.
    let log = history::read_log(&storage.logs_dir(), Path::new(&run.log_path)).unwrap();
    assert!(!log.error_lines.is_empty(), "log: {}", log.text);
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
    write(&a.join("due.txt"), "due");
    write(&a.join("uno.txt"), "uno");
    write(&a.join("è già.txt"), "tre");
    write(&b.join("only-in-b.txt"), "extra");
    let storage = storage_in(temp.path());
    let counts = wizard::preview::preview(path_str(&a), path_str(&b), SyncMode::Copy)
        .unwrap()
        .counts();
    // The progress is measured against this total: the extra file in B is not in it.
    assert_eq!(counts.total(), 3, "extras in B are not counted");

    let mut events = Vec::new();
    let run = runner::run(
        &storage,
        &saved("Copia di A", &a, &b),
        counts,
        &ActiveRun::default(),
        |progress| events.push(progress.clone()),
    )
    .unwrap();

    assert_eq!(run.outcome, Outcome::Ok);
    let last = events.last().expect("at least one progress event");
    assert_eq!(last.done, 3, "extras in B are not counted");
    assert_eq!(last.sync_name, "Copia di A");
    assert_eq!(
        last.current_file, "è già.txt",
        "the final state names the last file, readable"
    );
    assert!(events.windows(2).all(|w| w[0].done <= w[1].done));
}

#[test]
fn cancel_kills_rsync_and_records_a_cancelled_run() {
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
    let storage = storage_in(temp.path());
    let counts = wizard::preview::preview(path_str(&a), path_str(&b), SyncMode::Copy)
        .unwrap()
        .counts();
    let active = ActiveRun::default();

    let run = runner::run(
        &storage,
        &saved("Copia di A", &a, &b),
        counts,
        &active,
        |_| {
            active.cancel(false);
        },
    )
    .unwrap();

    assert_eq!(
        run.outcome,
        Outcome::Cancelled,
        "a killed rsync has no exit code of its own"
    );
    assert_eq!((run.new, run.updated, run.deleted), (0, 0, 0));
    assert!(run.cause.is_none());
    assert!(run.partial.is_some());
    assert!(!active.is_running(), "the rsync process is gone");
    assert!(
        !b.join("z-grande.bin").exists(),
        "the big file was not copied completely"
    );
    let history = fs::read_to_string(storage.history_file()).unwrap();
    let recorded: Run = serde_json::from_str(history.trim()).unwrap();
    assert_eq!(recorded, run);
    assert!(!active.cancel(false), "nothing left to cancel");
}

#[test]
fn the_active_run_is_freed_when_the_history_cannot_be_written() {
    let temp = tempfile::tempdir().unwrap();
    let (a, b) = (temp.path().join("A"), temp.path().join("B"));
    write(&a.join("uno.txt"), "uno");
    let storage = storage_in(temp.path());
    fs::create_dir_all(storage.history_file()).unwrap(); // a folder where the file should be
    let active = ActiveRun::default();

    let result = runner::run(
        &storage,
        &saved("Copia di A", &a, &b),
        Default::default(),
        &active,
        |_| {},
    );

    assert!(result.is_err());
    assert!(!active.is_running());
}

#[test]
fn extra_files_count_as_deleted_only_in_mirror() {
    let temp = tempfile::tempdir().unwrap();
    let (a, b) = (temp.path().join("A"), temp.path().join("B"));
    write(&a.join("new.txt"), "nuovo");
    write(&b.join("only-in-b.txt"), "extra");
    write(&b.join("old-dir").join("inside.txt"), "extra");

    let copy = wizard::preview::preview(path_str(&a), path_str(&b), SyncMode::Copy).unwrap();
    let mirror = wizard::preview::preview(path_str(&a), path_str(&b), SyncMode::Mirror).unwrap();

    assert_eq!(copy.deleted.count, 0);
    assert!(copy.blocked.is_none());
    assert_eq!(mirror.new.files, ["new.txt"]);
    assert_eq!(mirror.deleted.count, 2, "files only, not the folder");
    assert_eq!(
        sorted(&mirror.deleted.files),
        ["old-dir/inside.txt", "only-in-b.txt"]
    );
    assert_eq!(
        mirror.blocked.as_deref(),
        Some(mirror_guard::TOO_MANY_DELETES)
    );
    assert!(
        b.join("only-in-b.txt").exists(),
        "a preview touches nothing"
    );
}

#[test]
fn mirror_deletes_extra_files_in_destination() {
    let temp = tempfile::tempdir().unwrap();
    let (a, b) = (temp.path().join("A"), temp.path().join("B"));
    write(&a.join("new.txt"), "nuovo");
    write(&a.join("sub").join("deep.txt"), "x");
    write(&a.join("keep.txt"), "uguale");
    identical_in(&b, &a, "keep.txt");
    write(&b.join("only-in-b.txt"), "extra");
    write(&b.join("old-dir").join("inside.txt"), "extra");
    let storage = storage_in(temp.path());

    let preview = wizard::preview::preview(path_str(&a), path_str(&b), SyncMode::Mirror).unwrap();
    assert_eq!(preview.deleted.count, 2);
    assert!(preview.blocked.is_none());

    let run = run_mirror(&storage, &a, &b, preview.counts()).unwrap();

    assert_eq!(run.outcome, Outcome::Ok);
    assert_eq!(run.mode, SyncMode::Mirror);
    assert_eq!((run.new, run.updated, run.deleted), (2, 0, 2));
    assert!(!b.join("only-in-b.txt").exists());
    assert!(!b.join("old-dir").exists());
    assert!(b.join("keep.txt").exists());
    assert_eq!(
        fs::read_to_string(b.join("sub").join("deep.txt")).unwrap(),
        "x"
    );
    let history = fs::read_to_string(storage.history_file()).unwrap();
    let recorded = serde_json::from_str::<Run>(history.trim()).unwrap();
    assert_eq!((recorded.mode, recorded.deleted), (SyncMode::Mirror, 2));
}

#[test]
fn mirror_with_a_missing_or_empty_source_is_blocked_and_never_run() {
    let temp = tempfile::tempdir().unwrap();
    let (empty, b) = (temp.path().join("A"), temp.path().join("B"));
    fs::create_dir_all(&empty).unwrap();
    write(&b.join("precious.txt"), "do not delete");
    let missing = temp.path().join("scollegato");
    let storage = storage_in(temp.path());

    // A that exists and is empty: the guard blocks the mirror.
    let preview =
        wizard::preview::preview(path_str(&empty), path_str(&b), SyncMode::Mirror).unwrap();
    assert_eq!(preview.blocked.as_deref(), Some(mirror_guard::SOURCE_EMPTY));
    assert!(run_mirror(&storage, &empty, &b, preview.counts()).is_err());

    // A that is not there at all is said as such, in Mirror too (and nothing can be started).
    let failure =
        wizard::preview::preview(path_str(&missing), path_str(&b), SyncMode::Mirror).unwrap_err();
    assert_eq!(
        failure.title,
        format!("Non trovo la cartella A ({}).", path_str(&missing))
    );
    assert!(run_mirror(&storage, &missing, &b, Default::default()).is_err());

    assert!(b.join("precious.txt").exists());
    assert!(!storage.history_file().exists(), "nothing was recorded");
}

#[test]
fn mirror_with_more_deleted_than_new_plus_updated_is_blocked() {
    let temp = tempfile::tempdir().unwrap();
    let (a, b) = (temp.path().join("A"), temp.path().join("B"));
    write(&a.join("a.txt"), "x");
    identical_in(&b, &a, "a.txt");
    write(&b.join("extra1.txt"), "x");

    let preview = wizard::preview::preview(path_str(&a), path_str(&b), SyncMode::Mirror).unwrap();

    assert_eq!(preview.counts().deleted, 1);
    assert_eq!(
        preview.blocked.as_deref(),
        Some(mirror_guard::TOO_MANY_DELETES)
    );
}
