//! A Preview or a Run end to end: robocopy + parser + storage + history. No Tauri types.

use crate::history::{self, Outcome, Run};
use crate::mirror_guard;
use crate::preview_parser::{self, Counts, Preview};
use crate::progress::{Progress, StdoutFollower};
use crate::robocopy::{self, SyncMode};
use crate::storage::Storage;
use chrono::Local;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Child, ChildStdout, ExitStatus};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// Dry run (`/L`): lists what a Run would do, touches nothing. Blocks until robocopy exits.
pub fn preview(source: &str, destination: &str, mode: SyncMode) -> Result<Preview, String> {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let log = std::env::temp_dir().join(format!(
        "colombosync-anteprima-{}-{}.log",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let log_arg = log.to_string_lossy();

    // A missing or empty A would make robocopy fail or empty B: answer "blocked" without running it.
    let is_mirror = mode == SyncMode::Mirror;
    if is_mirror && mirror_guard::blocks(Path::new(source), Counts::default()) {
        return Ok(Preview {
            blocked: true,
            ..Preview::default()
        });
    }

    let code = robocopy::execute(&robocopy::preview_args(mode, source, destination, &log_arg))
        .map_err(|e| e.to_string())?;
    let text = read_log(&log);
    let _ = fs::remove_file(&log);

    if !robocopy::is_success(code) {
        return Err(failure_message(code, &text));
    }
    let mut preview = preview_parser::parse(&text, source, destination, |p| p.exists());
    preview.blocked = is_mirror && mirror_guard::blocks(Path::new(source), preview.counts());
    Ok(preview)
}

/// The robocopy process of the Run in progress, shared so another thread can stop it.
#[derive(Default)]
pub struct ActiveRun {
    child: Mutex<Option<Child>>,
    cancelled: AtomicBool,
    exit_when_done: AtomicBool,
}

impl ActiveRun {
    /// A Run is in flight: from robocopy's start until its history line is written.
    pub fn is_running(&self) -> bool {
        self.slot().is_some()
    }

    /// Kills robocopy; the Run then ends as `Cancelled`. `false` when nothing is running.
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

    fn start(&self, args: &[String]) -> Result<ChildStdout, String> {
        let mut slot = self.slot();
        if slot.is_some() {
            return Err("C’è già una copia in corso.".into());
        }
        let mut child = robocopy::spawn(args).map_err(|e| e.to_string())?;
        let stdout = child.stdout.take().ok_or("robocopy senza stdout")?;
        self.cancelled.store(false, Ordering::SeqCst);
        self.exit_when_done.store(false, Ordering::SeqCst);
        *slot = Some(child);
        Ok(stdout)
    }

    /// Waits for robocopy to exit without holding the lock, so `cancel` stays possible.
    fn wait(&self) -> io::Result<ExitStatus> {
        loop {
            {
                let mut slot = self.slot();
                let child = slot
                    .as_mut()
                    .ok_or_else(|| io::Error::other("no robocopy process"))?;
                if let Some(status) = child.try_wait()? {
                    return Ok(status);
                }
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
}

/// Frees the `ActiveRun` slot when `run` ends on any path, and makes sure robocopy is gone.
struct SlotGuard<'a>(&'a ActiveRun);

impl Drop for SlotGuard<'_> {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.slot().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Progress is reported at most this often (robocopy can print thousands of lines a second).
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

/// Real run. Always returns the recorded `Run` (outcome `Ok`, `Error` or `Cancelled`); `Err` only
/// when robocopy cannot be started or the history cannot be written. `counts` come from the
/// preview the user just confirmed and give the total of `on_progress`, which is called at most
/// every `PROGRESS_EVERY` while robocopy writes, and once more when its output ends.
pub fn run(
    storage: &Storage,
    source: &str,
    destination: &str,
    mode: SyncMode,
    counts: Counts,
    active: &ActiveRun,
    mut on_progress: impl FnMut(&Progress),
) -> Result<Run, String> {
    if mode == SyncMode::Mirror && mirror_guard::blocks(Path::new(source), counts) {
        return Err(mirror_guard::WARNING.to_string());
    }
    let started = Local::now();
    let sync_name = default_sync_name(source);
    let log = storage.log_file(&sync_name, started.naive_local());
    fs::create_dir_all(storage.logs_dir()).map_err(|e| e.to_string())?;

    let clock = Instant::now();
    let mut stdout = active.start(&robocopy::run_args(
        mode,
        source,
        destination,
        &log.to_string_lossy(),
    ))?;
    let _slot = SlotGuard(active);
    let total = counts.new + counts.updated + counts.deleted;
    let mut follower = StdoutFollower::default();
    let progress = |follower: &StdoutFollower| Progress {
        sync_name: sync_name.clone(),
        done: follower.done().min(total),
        total,
        elapsed_ms: clock.elapsed().as_millis() as u64,
        current_file: follower.current_file(),
    };
    let mut last_report: Option<Instant> = None;
    let mut chunk = [0u8; 8192];
    // Ends at EOF: robocopy exited or was killed.
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
    let duration_ms = clock.elapsed().as_millis() as u64;

    // A killed robocopy exits with code 1, which alone would read as success.
    let cancelled = active.cancelled.load(Ordering::SeqCst);
    let code = robocopy::exit_code(status).map_err(|e| e.to_string())?;
    let ok = !cancelled && robocopy::is_success(code);
    // Only a failed or stopped run needs its log: it can be huge.
    let log_text = if ok { String::new() } else { read_log(&log) };
    let run = Run {
        sync_name,
        source: source.to_string(),
        destination: destination.to_string(),
        mode,
        new: if ok { counts.new } else { 0 },
        updated: if ok { counts.updated } else { 0 },
        deleted: if ok { counts.deleted } else { 0 },
        duration_ms,
        outcome: match (ok, cancelled) {
            (true, _) => Outcome::Ok,
            (false, true) => Outcome::Cancelled,
            (false, false) => Outcome::Error,
        },
        started_at: started.to_rfc3339(),
        log_path: log.to_string_lossy().into_owned(),
        error: (!ok && !cancelled).then(|| failure_message(code, &log_text)),
        partial: (!ok).then(|| partial_count(&log_text, source, destination)),
    };
    history::append(&storage.history_file(), &run).map_err(|e| e.to_string())?;
    Ok(run)
}

/// Files copied before a failed or stopped run ended. The last file line is the file in flight
/// (possibly cut short), so it is not counted.
fn partial_count(log: &str, source: &str, destination: &str) -> usize {
    preview_parser::count_source_files(log, source, destination).saturating_sub(1)
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

fn read_log(path: &Path) -> String {
    fs::read(path)
        .map(|bytes| robocopy::decode_unilog(&bytes))
        .unwrap_or_default()
}

/// Raw message: exit code plus the last lines of the log.
fn failure_message(code: i32, log: &str) -> String {
    let tail: Vec<&str> = log
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let tail = tail[tail.len().saturating_sub(3)..].join(" ");
    format!("robocopy ha restituito il codice {code}. {tail}")
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_name_is_copia_di_last_folder_of_a() {
        assert_eq!(default_sync_name(r"D:\Foto"), "Copia di Foto");
        assert_eq!(default_sync_name(r"D:\Foto\2024\"), "Copia di 2024");
        assert_eq!(default_sync_name(r"\\NAS\Condivisa\Foto"), "Copia di Foto");
        assert_eq!(default_sync_name(r"D:\"), "Copia di disco D:");
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
    fn partial_count_skips_the_file_in_flight() {
        let line = |name: &str| format!("\t  \t\t 5\tD:\\Foto\\{name}\r\n");
        let log = line("a.txt") + &line("b.txt") + &line("c.txt");
        assert_eq!(partial_count(&log, r"D:\Foto", r"E:\Backup"), 2);
        assert_eq!(partial_count(&line("a.txt"), r"D:\Foto", r"E:\Backup"), 0);
        assert_eq!(partial_count("", r"D:\Foto", r"E:\Backup"), 0);
    }

    #[test]
    fn failure_message_has_exit_code_and_log_tail() {
        let log = "\r\nERROR 53 (0x00000035) Accessing Source Directory \\\\NAS\\x\\\r\nThe network path was not found.\r\n";
        assert_eq!(
            failure_message(16, log),
            "robocopy ha restituito il codice 16. ERROR 53 (0x00000035) Accessing Source Directory \\\\NAS\\x\\ The network path was not found."
        );
        assert_eq!(
            failure_message(16, ""),
            "robocopy ha restituito il codice 16."
        );
    }
}
