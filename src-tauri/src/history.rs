//! `history.jsonl`: append-only, one `Run` per line.

use crate::engine::{self, SyncMode};
use crate::error_messages::Cause;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Ok,
    Error,
    Cancelled,
}

/// One execution of a sync. This is the stored format: S4/S5 read and extend usage, not shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    /// The saved sync this run belongs to (lines written before syncs had ids have none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_id: Option<u64>,
    pub sync_name: String,
    pub source: String,
    pub destination: String,
    pub mode: SyncMode,
    pub new: usize,
    pub updated: usize,
    pub deleted: usize,
    pub duration_ms: u64,
    pub outcome: Outcome,
    /// Local start time, RFC 3339.
    pub started_at: String,
    pub log_path: String,
    /// Files copied before an `Error` or `Cancelled` run stopped (no new/updated split).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partial: Option<usize>,
    /// Why an `Error` run failed (a mapped Windows error code and the folder it is about). The
    /// texts are made from it when they are shown; runs without it read as a generic failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<Cause>,
}

pub fn append(file: &Path, run: &Run) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut line = serde_json::to_string(run).map_err(io::Error::other)?;
    line.push('\n');
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)?
        .write_all(line.as_bytes())
}

/// Runs newest first. Lines that do not parse (also invalid UTF-8) are skipped one by one;
/// a missing file is an empty history, any other read error is reported.
pub fn read(file: &Path) -> io::Result<Vec<Run>> {
    let bytes = match fs::read(file) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut runs: Vec<Run> = bytes
        .split(|&b| b == b'\n')
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect();
    runs.reverse();
    Ok(runs)
}

/// Longest log tail shown: 2 MiB of the file.
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
const TRUNCATED_NOTICE: &str = "… (mostro solo l’ultima parte del registro)\n";

/// A run's log as text (robocopy's `/UNILOG`, UTF-16LE with BOM, or rsync's UTF-8 output), only
/// if the file is inside `logs_dir`. Long logs are cut to their last part.
pub fn read_log(logs_dir: &Path, file: &Path) -> io::Result<String> {
    read_log_capped(logs_dir, file, MAX_LOG_BYTES)
}

fn read_log_capped(logs_dir: &Path, file: &Path, max_bytes: u64) -> io::Result<String> {
    let lower = |p: &Path| PathBuf::from(p.to_string_lossy().to_lowercase());
    let file = file.canonicalize()?;
    if !lower(&file).starts_with(lower(&logs_dir.canonicalize()?)) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Il file non è nella cartella dei registri.",
        ));
    }
    let mut input = File::open(&file)?;
    let len = input.metadata()?.len();
    if len <= max_bytes {
        let mut bytes = Vec::with_capacity(len as usize);
        input.read_to_end(&mut bytes)?;
        return Ok(engine::decode_log(&bytes));
    }
    // Start on a UTF-16 unit boundary, then drop the cut line.
    input.seek(SeekFrom::Start((len - max_bytes) & !1))?;
    let mut tail = Vec::new();
    input.read_to_end(&mut tail)?;
    let text = engine::decode_log(&tail);
    let from_next_line = text
        .split_once('\n')
        .map_or(text.as_str(), |(_, rest)| rest);
    Ok(format!("{TRUNCATED_NOTICE}{from_next_line}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(name: &str, outcome: Outcome) -> Run {
        Run {
            sync_id: None,
            sync_name: name.into(),
            source: r"D:\Foto".into(),
            destination: r"E:\Backup".into(),
            mode: SyncMode::Copy,
            new: 12,
            updated: 1,
            deleted: 0,
            duration_ms: 61_000,
            outcome,
            started_at: "2026-11-02T21:30:00+01:00".into(),
            log_path: r"C:\L\logs\x.log".into(),
            partial: None,
            cause: None,
        }
    }

    #[test]
    fn append_writes_one_json_line_per_run_creating_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("ColomboSync").join("history.jsonl");

        append(&file, &run("Copia di Foto", Outcome::Ok)).unwrap();
        append(&file, &run("Altra", Outcome::Error)).unwrap();

        let text = fs::read_to_string(&file).unwrap();
        let runs: Vec<Run> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0], run("Copia di Foto", Outcome::Ok));
        assert_eq!(runs[1].outcome, Outcome::Error);
    }

    #[test]
    fn stored_format_is_stable() {
        let json = serde_json::to_string(&run("n", Outcome::Cancelled)).unwrap();
        assert_eq!(
            json,
            r#"{"syncName":"n","source":"D:\\Foto","destination":"E:\\Backup","mode":"Copy","new":12,"updated":1,"deleted":0,"durationMs":61000,"outcome":"Cancelled","startedAt":"2026-11-02T21:30:00+01:00","logPath":"C:\\L\\logs\\x.log"}"#
        );
    }

    fn names(file: &Path) -> Vec<String> {
        read(file)
            .unwrap()
            .into_iter()
            .map(|r| r.sync_name)
            .collect()
    }

    fn good(name: &str) -> String {
        serde_json::to_string(&run(name, Outcome::Ok)).unwrap()
    }

    #[test]
    fn reads_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("history.jsonl");
        append(&file, &run("prima", Outcome::Ok)).unwrap();
        append(&file, &run("seconda", Outcome::Error)).unwrap();
        append(&file, &run("terza", Outcome::Cancelled)).unwrap();

        assert_eq!(names(&file), ["terza", "seconda", "prima"]);
    }

    #[test]
    fn skips_malformed_lines() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("history.jsonl");
        let text = format!(
            "{}\nnot json\n{{\"syncName\":\"incompleta\"}}\n\n{}\n",
            good("buona"),
            good("ultima")
        );
        fs::write(&file, text).unwrap();

        assert_eq!(names(&file), ["ultima", "buona"]);
    }

    #[test]
    fn invalid_utf8_skips_only_that_line() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("history.jsonl");
        let mut bytes = format!("{}\n", good("prima")).into_bytes();
        bytes.extend_from_slice(b"{\"syncName\":\"rotta\xFF\"}\n");
        bytes.extend_from_slice(format!("{}\n", good("ultima")).as_bytes());
        fs::write(&file, bytes).unwrap();

        assert_eq!(names(&file), ["ultima", "prima"]);
    }

    #[test]
    fn missing_history_file_is_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(&dir.path().join("history.jsonl")).unwrap().is_empty());
    }

    #[test]
    fn other_read_errors_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        // A folder where the file should be: reading it fails, and it is not "not found".
        assert!(read(dir.path()).is_err());
    }

    /// A run's log file as the engine writes it: UTF-16LE with BOM for robocopy, UTF-8 for rsync.
    fn unilog(text: &str) -> Vec<u8> {
        if !cfg!(windows) {
            return text.as_bytes().to_vec();
        }
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        bytes
    }

    #[test]
    fn read_log_decodes_utf16le_and_fails_when_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("x.log");
        fs::write(&file, unilog("è già\r\n")).unwrap();

        assert_eq!(read_log(dir.path(), &file).unwrap(), "è già\r\n");
        fs::remove_file(&file).unwrap();
        assert!(read_log(dir.path(), &file).is_err());
    }

    #[test]
    fn read_log_refuses_files_outside_the_logs_folder() {
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join("logs");
        fs::create_dir(&logs).unwrap();
        let outside = dir.path().join("secret.txt");
        fs::write(&outside, unilog("segreto")).unwrap();
        fs::write(logs.join("ok.log"), unilog("ok")).unwrap();

        assert_eq!(read_log(&logs, &logs.join("ok.log")).unwrap(), "ok");
        assert!(read_log(&logs, &outside).is_err());
        assert!(read_log(&logs, &logs.join("..").join("secret.txt")).is_err());
    }

    #[test]
    fn read_log_cuts_long_logs_to_the_last_lines() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("long.log");
        let text: String = (1..=100).map(|n| format!("riga {n:03}\r\n")).collect();
        fs::write(&file, unilog(&text)).unwrap();

        let shown = read_log_capped(dir.path(), &file, 200).unwrap();

        assert!(shown.starts_with("… (mostro solo l’ultima parte del registro)\n"));
        assert!(shown.ends_with("riga 100\r\n"));
        assert!(!shown.contains("riga 001"));
        assert!(shown.lines().count() < 20);
        // No half line after the notice.
        assert!(shown.lines().nth(1).unwrap().starts_with("riga "));
    }

    #[test]
    fn run_without_sync_id_field_still_loads() {
        let old = r#"{"syncName":"n","source":"D:\\Foto","destination":"E:\\Backup","mode":"Copy","new":12,"updated":1,"deleted":0,"durationMs":61000,"outcome":"Ok","startedAt":"2026-11-02T21:30:00+01:00","logPath":"C:\\L\\logs\\x.log"}"#;
        let loaded: Run = serde_json::from_str(old).unwrap();
        assert_eq!(loaded.sync_id, None);

        let with_id = Run {
            sync_id: Some(7),
            ..loaded
        };
        let json = serde_json::to_string(&with_id).unwrap();
        assert!(json.starts_with(r#"{"syncId":7,"#));
        assert_eq!(serde_json::from_str::<Run>(&json).unwrap(), with_id);
    }

    #[test]
    fn run_without_partial_field_still_loads() {
        let old = r#"{"syncName":"n","source":"D:\\Foto","destination":"E:\\Backup","mode":"Copy","new":12,"updated":1,"deleted":0,"durationMs":61000,"outcome":"Cancelled","startedAt":"2026-11-02T21:30:00+01:00","logPath":"C:\\L\\logs\\x.log"}"#;
        let loaded: Run = serde_json::from_str(old).unwrap();
        assert_eq!(loaded, run("n", Outcome::Cancelled));

        let partial = Run {
            partial: Some(40),
            ..loaded
        };
        let json = serde_json::to_string(&partial).unwrap();
        assert!(json.ends_with(r#","partial":40}"#));
        assert_eq!(serde_json::from_str::<Run>(&json).unwrap(), partial);
    }

    #[test]
    fn cause_is_stored_only_when_set_and_old_error_lines_still_load() {
        let failed = Run {
            cause: Some(Cause {
                code: 53,
                folder: crate::error_messages::Folder::B,
            }),
            ..run("n", Outcome::Error)
        };
        let json = serde_json::to_string(&failed).unwrap();
        assert!(json.ends_with(r#","cause":{"code":53,"folder":"B"}}"#));
        assert_eq!(serde_json::from_str::<Run>(&json).unwrap(), failed);
        let ok = serde_json::to_string(&run("n", Outcome::Ok)).unwrap();
        assert!(!ok.contains("cause"));

        // Before S6 a failed run kept robocopy's raw message in `error`: no cause, nothing breaks.
        let old = r#"{"syncName":"n","source":"D:\\Foto","destination":"E:\\Backup","mode":"Copy","new":0,"updated":0,"deleted":0,"durationMs":1,"outcome":"Error","startedAt":"2026-11-02T21:30:00+01:00","logPath":"C:\\L\\logs\\x.log","error":"robocopy ha restituito il codice 16."}"#;
        assert_eq!(serde_json::from_str::<Run>(old).unwrap().cause, None);
    }
}
