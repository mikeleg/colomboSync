//! A Preview or a Run end to end: robocopy + parser + storage + history. No Tauri types.

use crate::error_messages::{self, Explanation, Moment};
use crate::history::{self, Outcome, Run};
use crate::mirror_guard;
use crate::path_rules;
use crate::preview_parser::{self, Counts, Preview};
use crate::progress::{Progress, StdoutFollower};
use crate::robocopy::{self, SyncMode};
use crate::storage::Storage;
use crate::sync_store::SavedSync;
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
pub fn preview(source: &str, destination: &str, mode: SyncMode) -> Result<Preview, Explanation> {
    path_rules::check_destination(source, destination)?;
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let log = std::env::temp_dir().join(format!(
        "colombosync-anteprima-{}-{}.log",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let log_arg = log.to_string_lossy();

    // A that cannot be read says why (unplugged disk, share down, no permission): that is not
    // the same as an A with nothing in it, and robocopy would only fail on it.
    if let Err(error) = fs::metadata(source) {
        return Err(error_messages::explain_source_error(
            &error,
            source,
            destination,
        ));
    }
    // An empty A would empty B: answer "blocked" without running robocopy.
    if mode == SyncMode::Mirror
        && let Some(reason) = mirror_guard::source_problem(Path::new(source))
    {
        return Ok(Preview {
            blocked: Some(reason.to_string()),
            ..Preview::default()
        });
    }

    let code = robocopy::execute(&robocopy::preview_args(mode, source, destination, &log_arg))
        .map_err(|e| Explanation::from(e.to_string()))?;
    let text = read_log(&log);
    let _ = fs::remove_file(&log);

    if !robocopy::is_success(code) {
        let cause = error_messages::find_cause(&text, source, destination);
        return Err(error_messages::describe(
            cause,
            source,
            destination,
            Moment::Preview,
        ));
    }
    let mut preview = preview_parser::parse(&text, source, destination, |p| p.exists());
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
    sync: &SavedSync,
    counts: Counts,
    active: &ActiveRun,
    mut on_progress: impl FnMut(&Progress),
) -> Result<Run, String> {
    let (source, destination, mode) = (sync.source.as_str(), sync.destination.as_str(), sync.mode);
    let sync_name = sync.name.clone();
    path_rules::check_destination(source, destination)?;
    if mode == SyncMode::Mirror
        && let Some(reason) = mirror_guard::problem(Path::new(source), counts)
    {
        return Err(reason.to_string());
    }
    let started = Local::now();
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
    let cause = if ok || cancelled {
        None
    } else {
        error_messages::find_cause(&log_text, source, destination)
    };
    let run = Run {
        sync_id: Some(sync.id),
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
        partial: (!ok).then(|| partial_count(&log_text, source, destination)),
        cause,
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
        let cases = [
            (r"D:\Foto", r"d:\foto\", path_rules::SAME_FOLDER),
            (r"D:\Foto", r"D:\Foto\Backup", path_rules::INSIDE_SOURCE),
            (
                r"D:\Foto\2024",
                r"D:\Foto",
                path_rules::SOURCE_INSIDE_DESTINATION,
            ),
        ];
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
    fn preview_of_a_missing_source_says_where_to_look_in_both_modes() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("Foto");
        let missing = missing.to_str().unwrap();

        for mode in [SyncMode::Copy, SyncMode::Mirror] {
            let failure = preview(missing, r"E:\Backup", mode).unwrap_err();
            assert_eq!(
                failure.title,
                format!("Non trovo la cartella A ({missing}).")
            );
            assert_eq!(failure.advice, "Il disco è collegato?");
        }
    }
}
