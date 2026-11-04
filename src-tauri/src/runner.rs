//! A Preview or a Run end to end: robocopy + parser + storage + history. No Tauri types.

use crate::history::{self, Outcome, Run};
use crate::mirror_guard;
use crate::preview_parser::{self, Counts, Preview};
use crate::robocopy::{self, SyncMode};
use crate::storage::Storage;
use chrono::Local;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

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

/// Real run. Always returns the recorded `Run` (outcome `Ok` or `Error`); `Err` only when
/// robocopy cannot be started or the history cannot be written. `counts` come from the preview
/// the user just confirmed.
pub fn run(
    storage: &Storage,
    source: &str,
    destination: &str,
    mode: SyncMode,
    counts: Counts,
) -> Result<Run, String> {
    if mode == SyncMode::Mirror && mirror_guard::blocks(Path::new(source), counts) {
        return Err(mirror_guard::WARNING.to_string());
    }
    let started = Local::now();
    let sync_name = default_sync_name(source);
    let log = storage.log_file(&sync_name, started.naive_local());
    fs::create_dir_all(storage.logs_dir()).map_err(|e| e.to_string())?;

    let clock = Instant::now();
    let code = robocopy::execute(&robocopy::run_args(
        mode,
        source,
        destination,
        &log.to_string_lossy(),
    ))
    .map_err(|e| e.to_string())?;
    let duration_ms = clock.elapsed().as_millis() as u64;

    let ok = robocopy::is_success(code);
    let run = Run {
        sync_name,
        source: source.to_string(),
        destination: destination.to_string(),
        mode,
        new: if ok { counts.new } else { 0 },
        updated: if ok { counts.updated } else { 0 },
        deleted: if ok { counts.deleted } else { 0 },
        duration_ms,
        outcome: if ok { Outcome::Ok } else { Outcome::Error },
        started_at: started.to_rfc3339(),
        log_path: log.to_string_lossy().into_owned(),
        error: (!ok).then(|| failure_message(code, &read_log(&log))),
    };
    history::append(&storage.history_file(), &run).map_err(|e| e.to_string())?;
    Ok(run)
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
