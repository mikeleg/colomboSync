//! A Preview or a Run end to end: robocopy + parser + storage + history. No Tauri types.

use crate::history::{self, Outcome, Run};
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

    let code = robocopy::execute(&robocopy::preview_args(mode, source, destination, &log_arg))
        .map_err(|e| e.to_string())?;
    let text = read_log(&log);
    let _ = fs::remove_file(&log);

    if !robocopy::is_success(code) {
        return Err(failure_message(code, &text));
    }
    Ok(preview_parser::parse(&text, source, destination, |p| {
        p.exists()
    }))
}

/// The robocopy process of the Run in progress, shared so another thread can stop it.
#[derive(Default)]
pub struct ActiveRun {
    child: Mutex<Option<Child>>,
    cancelled: AtomicBool,
    exit_when_done: AtomicBool,
}

impl ActiveRun {
    pub fn is_running(&self) -> bool {
        self.slot().is_some()
    }

    /// Kills robocopy; the Run then ends as `Cancelled`. `false` when nothing is running.
    /// `exit_when_done` tells the caller of `run` that the app is closing: the history line is
    /// written first.
    pub fn cancel(&self, exit_when_done: bool) -> bool {
        let mut slot = self.slot();
        let Some(child) = slot.as_mut() else {
            return false;
        };
        self.exit_when_done.store(exit_when_done, Ordering::SeqCst);
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
                    *slot = None;
                    return Ok(status);
                }
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
}

/// Real run. Always returns the recorded `Run` (outcome `Ok`, `Error` or `Cancelled`); `Err` only
/// when robocopy cannot be started or the history cannot be written. `counts` come from the
/// preview the user just confirmed and give the total of `on_progress`, which is called for
/// every chunk robocopy writes to stdout.
pub fn run(
    storage: &Storage,
    source: &str,
    destination: &str,
    mode: SyncMode,
    counts: Counts,
    active: &ActiveRun,
    mut on_progress: impl FnMut(&Progress),
) -> Result<Run, String> {
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
    let total = counts.new + counts.updated + counts.deleted;
    let mut follower = StdoutFollower::default();
    let mut chunk = [0u8; 8192];
    // Ends at EOF: robocopy exited or was killed.
    while let Ok(n @ 1..) = stdout.read(&mut chunk) {
        follower.feed(&chunk[..n]);
        on_progress(&Progress {
            sync_name: sync_name.clone(),
            done: follower.done().min(total),
            total,
            elapsed_ms: clock.elapsed().as_millis() as u64,
            current_file: follower.current_file().to_string(),
        });
    }
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
