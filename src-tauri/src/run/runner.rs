//! A Preview or a Run end to end: copy engine + parser + storage + history. No Tauri types.

use super::mirror_guard;
use super::progress::{Progress, StdoutFollower};
use crate::engine;
use crate::engine::preview_parser::{Counts, Preview};
use crate::history;
use crate::shared::domain::{Cause, Folder, Outcome, Run, SavedSync, SyncMode};
use crate::shared::error_messages::{self, ENGINE_MISSING, Explanation, Moment};
use crate::shared::path_rules;
use crate::shared::storage::Storage;
use chrono::Local;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Child, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// Dry run: lists what a Run would do, touches nothing. Blocks until the engine exits.
pub fn preview(source: &str, destination: &str, mode: SyncMode) -> Result<Preview, Explanation> {
    path_rules::check_destination(source, destination)?;

    // A that cannot be read says why (unplugged disk, share down, no permission): that is not
    // the same as an A with nothing in it, and the engine would only fail on it.
    fs::metadata(source)
        .map_err(|error| error_messages::explain_source_error(&error, source, destination))?;
    // An empty A would empty B: answer "blocked" without running the engine.
    if mode == SyncMode::Mirror
        && let Some(reason) = mirror_guard::source_problem(Path::new(source))
    {
        return Ok(Preview {
            blocked: Some(reason.to_string()),
            ..Preview::default()
        });
    }

    let finished = engine::dry_run(mode, source, destination).map_err(|error| {
        // Never the raw OS error: either the engine is not installed or it is a generic failure.
        error_messages::describe(missing_engine(&error), source, destination, Moment::Preview)
    })?;

    if !engine::is_success(finished.code) {
        let cause = engine::find_cause(finished.code, &finished.log, source, destination);
        return Err(error_messages::describe(
            cause,
            source,
            destination,
            Moment::Preview,
        ));
    }
    let mut preview = engine::parse_preview(&finished.log, source, destination);
    if mode == SyncMode::Mirror {
        preview.blocked = mirror_guard::counts_problem(preview.counts()).map(str::to_string);
    }
    Ok(preview)
}

pub const NO_PREVIEW: &str = "Prima controlla le due cartelle con l’anteprima, poi avvia.";

type LastPreview = Option<(String, String, SyncMode, Counts)>;

/// The last preview shown to the user. A Mirror run takes its counts from here, never from what
/// the UI sends back, and is refused when no preview of the same A, B and mode came first.
#[derive(Default)]
pub struct PreviewMemo(Mutex<LastPreview>);

impl PreviewMemo {
    pub fn remember(&self, source: &str, destination: &str, mode: SyncMode, counts: Counts) {
        *self.slot() = Some((source.into(), destination.into(), mode, counts));
    }

    /// Counts of the preview this run confirms; each preview confirms one run. Copy never
    /// deletes, so without a matching preview it falls back to the `sent` counts.
    pub fn take_counts(
        &self,
        source: &str,
        destination: &str,
        mode: SyncMode,
        sent: Counts,
    ) -> Result<Counts, String> {
        match self.slot().take() {
            Some((s, d, m, counts)) if s == source && d == destination && m == mode => Ok(counts),
            _ if mode == SyncMode::Copy => Ok(sent),
            _ => Err(NO_PREVIEW.to_string()),
        }
    }

    fn slot(&self) -> MutexGuard<'_, LastPreview> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The engine process of the Run in progress, shared so another thread can stop it.
#[derive(Default)]
pub struct ActiveRun {
    child: Mutex<Option<Child>>,
    cancelled: AtomicBool,
    exit_when_done: AtomicBool,
}

impl ActiveRun {
    /// A Run is in flight: from the engine's start until its history line is written.
    pub fn is_running(&self) -> bool {
        self.slot().is_some()
    }

    /// Kills the engine process; the Run then ends as `Cancelled`. `false` when nothing is running.
    /// `exit_when_done` (the app is closing) is remembered either way, so the caller of `run` can
    /// exit after the history line is written; when `false` is returned there is no run left to
    /// wait for.
    pub fn cancel(&self, exit_when_done: bool) -> bool {
        let mut slot = self.slot();
        if exit_when_done {
            self.exit_when_done.store(true, Ordering::SeqCst);
        }
        let Some(child) = slot.as_mut() else {
            return false;
        };
        self.cancelled.store(true, Ordering::SeqCst);
        let _ = child.kill();
        true
    }

    pub fn exit_when_done(&self) -> bool {
        self.exit_when_done.load(Ordering::SeqCst)
    }

    fn slot(&self) -> MutexGuard<'_, Option<Child>> {
        self.child.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts the engine; what it writes comes back from the returned reader.
    fn start(
        &self,
        mode: SyncMode,
        source: &str,
        destination: &str,
        log: &Path,
    ) -> Result<Box<dyn Read + Send>, StartError> {
        let mut slot = self.slot();
        if slot.is_some() {
            return Err(StartError::Busy);
        }
        let (child, stdout) =
            engine::spawn(mode, source, destination, log).map_err(StartError::Failed)?;
        self.cancelled.store(false, Ordering::SeqCst);
        self.exit_when_done.store(false, Ordering::SeqCst);
        *slot = Some(child);
        Ok(stdout)
    }

    /// Waits for the engine to exit without holding the lock, so `cancel` stays possible.
    fn wait(&self) -> io::Result<ExitStatus> {
        loop {
            {
                let mut slot = self.slot();
                let child = slot
                    .as_mut()
                    .ok_or_else(|| io::Error::other("no engine process"))?;
                if let Some(status) = child.try_wait()? {
                    return Ok(status);
                }
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
}

/// Frees the `ActiveRun` slot when `run` ends on any path, and makes sure the engine process is gone.
struct SlotGuard<'a>(&'a ActiveRun);

impl Drop for SlotGuard<'_> {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.slot().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

enum StartError {
    /// Another Run is in flight.
    Busy,
    /// The engine process could not be started.
    Failed(io::Error),
}

/// The cause to record when the engine is not installed; nothing else about a failed start is
/// worth a message of its own.
fn missing_engine(error: &io::Error) -> Option<Cause> {
    engine::is_missing(error).then_some(Cause {
        code: ENGINE_MISSING,
        folder: Folder::A,
    })
}

/// Progress is reported at most this often (the engine can print thousands of lines a second).
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

/// Real run. Always returns the recorded `Run` (outcome `Ok`, `Error` or `Cancelled`, also when the
/// engine cannot be started); `Err` only when the history cannot be written. `counts` come from the
/// preview the user just confirmed and give the total of `on_progress`, which is called at most
/// every `PROGRESS_EVERY` while the engine writes, and once more when its output ends.
pub fn run(
    storage: &Storage,
    sync: &SavedSync,
    counts: Counts,
    active: &ActiveRun,
    mut on_progress: impl FnMut(&Progress),
) -> Result<Run, String> {
    let (source, destination, mode) = (sync.source.as_str(), sync.destination.as_str(), sync.mode);
    path_rules::check_destination(source, destination)?;
    if mode == SyncMode::Mirror
        && let Some(reason) = mirror_guard::problem(Path::new(source), counts)
    {
        return Err(reason.to_string());
    }
    let started = Local::now();
    let log = storage.log_file(&sync.name, started.naive_local());
    fs::create_dir_all(storage.logs_dir()).map_err(|e| e.to_string())?;

    let clock = Instant::now();
    let record =
        |outcome: Outcome, counts: Counts, partial: Option<usize>, cause: Option<Cause>| Run {
            sync_id: Some(sync.id),
            sync_name: sync.name.clone(),
            source: source.to_string(),
            destination: destination.to_string(),
            mode,
            new: counts.new,
            updated: counts.updated,
            deleted: counts.deleted,
            duration_ms: clock.elapsed().as_millis() as u64,
            outcome,
            started_at: started.to_rfc3339(),
            log_path: log.to_string_lossy().into_owned(),
            partial,
            cause,
        };
    let mut stdout = match active.start(mode, source, destination, &log) {
        Ok(stdout) => stdout,
        Err(StartError::Busy) => return Err("C’è già una copia in corso.".into()),
        Err(StartError::Failed(error)) => {
            // The engine could not even start: a failed Run like any other, in the history.
            let run = record(
                Outcome::Error,
                Counts::default(),
                Some(0),
                missing_engine(&error),
            );
            history::append(&storage.history_file(), &run).map_err(|e| e.to_string())?;
            return Ok(run);
        }
    };
    let _slot = SlotGuard(active);
    let total = counts.new + counts.updated + counts.deleted;
    let mut follower = StdoutFollower::default();
    let progress = |follower: &StdoutFollower| Progress {
        sync_name: sync.name.clone(),
        done: follower.done().min(total),
        total,
        elapsed_ms: clock.elapsed().as_millis() as u64,
        current_file: follower.current_file(),
    };
    let mut last_report: Option<Instant> = None;
    let mut chunk = [0u8; 8192];
    // Ends at EOF: the engine exited or was killed.
    while let Ok(n @ 1..) = stdout.read(&mut chunk) {
        follower.feed(&chunk[..n]);
        if last_report.is_none_or(|t| t.elapsed() >= PROGRESS_EVERY) {
            last_report = Some(Instant::now());
            on_progress(&progress(&follower));
        }
    }
    // The state the throttle may have skipped, so the UI ends on the real count.
    on_progress(&progress(&follower));
    let status = active.wait().map_err(|e| e.to_string())?;

    // A killed robocopy exits with code 1, which alone would read as success; a killed rsync has
    // no exit code of its own (128 + 9).
    let cancelled = active.cancelled.load(Ordering::SeqCst);
    let code = engine::exit_code(status).map_err(|e| e.to_string())?;
    let ok = !cancelled && engine::is_success(code);
    // Only a failed or stopped run needs its log: it can be huge.
    let log_text = if ok { String::new() } else { read_log(&log) };
    let cause = if ok || cancelled {
        None
    } else {
        engine::find_cause(code, &log_text, source, destination)
    };
    let run = record(
        match (ok, cancelled) {
            (true, _) => Outcome::Ok,
            (false, true) => Outcome::Cancelled,
            (false, false) => Outcome::Error,
        },
        if ok { counts } else { Counts::default() },
        (!ok).then(|| engine::partial_count(&log_text, source, destination, code, cancelled)),
        cause,
    );
    history::append(&storage.history_file(), &run).map_err(|e| e.to_string())?;
    Ok(run)
}

fn read_log(path: &Path) -> String {
    fs::read(path)
        .map(|bytes| engine::decode_log(&bytes))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(new: usize, updated: usize, deleted: usize) -> Counts {
        Counts {
            new,
            updated,
            deleted,
        }
    }

    #[test]
    fn a_mirror_run_takes_its_counts_from_the_matching_preview_once() {
        let memo = PreviewMemo::default();
        let sent = counts(9, 9, 0);
        let (a, b) = (r"D:\Foto", r"E:\Backup");

        assert_eq!(
            memo.take_counts(a, b, SyncMode::Mirror, sent),
            Err(NO_PREVIEW.to_string()),
            "no preview at all"
        );

        memo.remember(a, b, SyncMode::Mirror, counts(1, 0, 5));
        assert_eq!(
            memo.take_counts(a, b, SyncMode::Mirror, sent),
            Ok(counts(1, 0, 5)),
            "what the UI sends is ignored"
        );
        assert!(
            memo.take_counts(a, b, SyncMode::Mirror, sent).is_err(),
            "one preview confirms one run"
        );

        for (source, destination, mode) in [
            (r"D:\Altro", b, SyncMode::Mirror),
            (a, r"E:\Altro", SyncMode::Mirror),
            (a, b, SyncMode::Copy),
        ] {
            memo.remember(a, b, SyncMode::Mirror, counts(1, 0, 0));
            let taken = memo.take_counts(source, destination, mode, sent);
            assert_eq!(
                taken.is_err(),
                mode == SyncMode::Mirror,
                "{source} {destination}"
            );
        }
    }

    #[test]
    fn a_copy_run_without_a_matching_preview_uses_the_sent_counts() {
        let memo = PreviewMemo::default();
        assert_eq!(
            memo.take_counts("A", "B", SyncMode::Copy, counts(2, 1, 0)),
            Ok(counts(2, 1, 0))
        );
    }

    #[test]
    fn preview_and_run_enforce_the_path_rules_themselves() {
        let temp = tempfile::tempdir().unwrap();
        let storage = Storage::at(temp.path().join("ColomboSync"));
        let cases = if cfg!(windows) {
            [
                (r"D:\Foto", r"d:\foto\", path_rules::SAME_FOLDER),
                (r"D:\Foto", r"D:\Foto\Backup", path_rules::INSIDE_SOURCE),
                (
                    r"D:\Foto\2024",
                    r"D:\Foto",
                    path_rules::SOURCE_INSIDE_DESTINATION,
                ),
            ]
        } else {
            [
                (
                    "/home/anna/Foto",
                    "/home/anna/Foto/",
                    path_rules::SAME_FOLDER,
                ),
                (
                    "/home/anna/Foto",
                    "/home/anna/Foto/Backup",
                    path_rules::INSIDE_SOURCE,
                ),
                (
                    "/home/anna/Foto/2024",
                    "/home/anna/Foto",
                    path_rules::SOURCE_INSIDE_DESTINATION,
                ),
            ]
        };
        for (a, b, message) in cases {
            for mode in [SyncMode::Copy, SyncMode::Mirror] {
                assert_eq!(preview(a, b, mode).unwrap_err(), Explanation::from(message));
                let active = ActiveRun::default();
                let sync = SavedSync {
                    id: 1,
                    name: "Foto".into(),
                    source: a.into(),
                    destination: b.into(),
                    mode,
                };
                let result = run(&storage, &sync, Counts::default(), &active, |_| {});
                assert_eq!(result.unwrap_err(), message);
            }
        }
        assert!(!storage.history_file().exists());
    }

    #[test]
    fn exit_request_is_remembered_even_when_no_run_is_active() {
        let active = ActiveRun::default();
        assert!(!active.cancel(false));
        assert!(!active.exit_when_done());
        assert!(!active.cancel(true), "nothing to stop");
        assert!(active.exit_when_done());
    }

    #[test]
    fn a_program_that_is_not_found_is_a_missing_engine_only_where_the_engine_is_installed_by_hand()
    {
        let not_found = io::Error::from(io::ErrorKind::NotFound);
        assert_eq!(missing_engine(&not_found).is_some(), cfg!(unix));
        assert_eq!(
            missing_engine(&io::Error::from(io::ErrorKind::PermissionDenied)),
            None
        );
        if cfg!(unix) {
            let explanation =
                error_messages::describe(missing_engine(&not_found), "/a", "/b", Moment::Preview);
            assert_eq!(explanation.title, "Su questo computer manca rsync.");
        }
    }

    #[test]
    fn preview_of_a_missing_source_says_where_to_look_in_both_modes() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("Foto");
        let missing = missing.to_str().unwrap();
        let backup = temp.path().join("Backup");
        let backup = backup.to_str().unwrap();

        for mode in [SyncMode::Copy, SyncMode::Mirror] {
            let failure = preview(missing, backup, mode).unwrap_err();
            assert_eq!(
                failure.title,
                format!("Non trovo la cartella A ({missing}).")
            );
            assert_eq!(failure.advice, "Il disco è collegato?");
        }
    }
}
