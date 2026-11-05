//! A Run end to end: copy engine + progress + storage + history. No Tauri types.

use super::progress::{Progress, StdoutFollower};
use crate::engine;
use crate::engine::preview_parser::Counts;
use crate::history;
use crate::shared::domain::{Cause, Outcome, Run, SavedSync, SyncMode};
use crate::shared::mirror_guard;
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
                engine::missing_engine(&error),
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

    #[test]
    fn run_enforces_the_path_rules_itself() {
        let temp = tempfile::tempdir().unwrap();
        let storage = Storage::at(temp.path().join("ColomboSync"));
        for (a, b, message) in path_rules::refused_pairs() {
            for mode in [SyncMode::Copy, SyncMode::Mirror] {
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
}
