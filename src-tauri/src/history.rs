//! `history.jsonl`: append-only, one `Run` per line.

use crate::robocopy::{SyncMode, decode_unilog};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

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
    /// Raw robocopy message when the outcome is `Error` (plain language comes in S6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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

/// Runs newest first. Lines that do not parse are skipped; a missing file is an empty history.
pub fn read(file: &Path) -> Vec<Run> {
    let text = fs::read_to_string(file).unwrap_or_default();
    let mut runs: Vec<Run> = text
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    runs.reverse();
    runs
}

/// A run's robocopy log (`/UNILOG`, UTF-16LE with BOM) as text.
pub fn read_log(file: &Path) -> io::Result<String> {
    fs::read(file).map(|bytes| decode_unilog(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(name: &str, outcome: Outcome) -> Run {
        Run {
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
            error: None,
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

    #[test]
    fn reads_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("history.jsonl");
        append(&file, &run("prima", Outcome::Ok)).unwrap();
        append(&file, &run("seconda", Outcome::Error)).unwrap();
        append(&file, &run("terza", Outcome::Cancelled)).unwrap();

        let names: Vec<String> = read(&file).into_iter().map(|r| r.sync_name).collect();

        assert_eq!(names, ["terza", "seconda", "prima"]);
    }

    #[test]
    fn skips_malformed_lines() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("history.jsonl");
        let good = |name: &str| serde_json::to_string(&run(name, Outcome::Ok)).unwrap();
        let text = format!(
            "{}\nnot json\n{{\"syncName\":\"incompleta\"}}\n\n{}\n",
            good("buona"),
            good("ultima")
        );
        fs::write(&file, text).unwrap();

        let names: Vec<String> = read(&file).into_iter().map(|r| r.sync_name).collect();

        assert_eq!(names, ["ultima", "buona"]);
    }

    #[test]
    fn missing_history_file_is_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(&dir.path().join("history.jsonl")).is_empty());
    }

    #[test]
    fn read_log_decodes_utf16le_and_fails_when_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("x.log");
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend("è già\r\n".encode_utf16().flat_map(u16::to_le_bytes));
        fs::write(&file, bytes).unwrap();

        assert_eq!(read_log(&file).unwrap(), "è già\r\n");
        fs::remove_file(&file).unwrap();
        assert!(read_log(&file).is_err());
    }
}
