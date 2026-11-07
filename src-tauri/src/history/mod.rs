//! `history.jsonl`: append-only, one `Run` per line.

pub mod commands;
pub mod summary;

use crate::engine;
use crate::shared::domain::Run;
use crate::shared::storage::read_if_present;
use serde::Serialize;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

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
    let Some(bytes) = read_if_present(file)? else {
        return Ok(Vec::new());
    };
    let mut runs: Vec<Run> = bytes
        .split(|&b| b == b'\n')
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect();
    runs.reverse();
    Ok(runs)
}

/// The latest run of each sync, in the order of `runs` (newest first, as `read` returns them): the
/// first run seen of a sync is its latest. Runs without a sync id (written before syncs had ids)
/// belong to no sync.
pub fn latest_per_sync(runs: &[Run]) -> Vec<(u64, &Run)> {
    let mut seen = HashSet::new();
    runs.iter()
        .filter_map(|run| {
            let id = run.sync_id?;
            seen.insert(id).then_some((id, run))
        })
        .collect()
}

/// Longest log tail shown: 2 MiB of the file.
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
const TRUNCATED_NOTICE: &str = "… (mostro solo l’ultima parte del registro)\n";
/// Heads the error lines found in the cut part, above `TRUNCATED_NOTICE`.
const EARLY_ERRORS_TITLE: &str = "Errori nella parte non mostrata del registro:\n";
const MAX_EARLY_ERRORS: usize = 200;
/// How far before the shown tail the errors are searched: 64 MiB. Older bytes are not read.
const MAX_SCAN_BYTES: u64 = 64 * 1024 * 1024;
/// Longest line kept while scanning: a longer one is cut to its first 64 KiB.
const MAX_LINE_BYTES: usize = 64 * 1024;
/// How much of the cut part is read at a time (even: UTF-16 units stay whole).
const SCAN_CHUNK_BYTES: usize = 256 * 1024;

/// What the log window shows of a run's log: the text, and which of its lines (from 0, as
/// `str::lines` splits it) are engine errors, for the UI to highlight.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunLog {
    pub text: String,
    pub error_lines: Vec<usize>,
}

/// Why a log could not be read.
#[derive(Debug)]
pub enum LogError {
    /// The file is not inside the folder of the logs: asked for by a wrong path, never by the app.
    OutsideLogsFolder,
    /// The OS could not open or read it (gone, no permission, …).
    Io(io::Error),
}

impl From<io::Error> for LogError {
    fn from(error: io::Error) -> Self {
        LogError::Io(error)
    }
}

/// A run's log as text (robocopy's `/UNILOG`, UTF-16LE with BOM, or rsync's UTF-8 output), only
/// if the file is inside `logs_dir`. Long logs are cut to their last part.
pub fn read_log(logs_dir: &Path, file: &Path) -> Result<RunLog, LogError> {
    read_log_capped(logs_dir, file, MAX_LOG_BYTES)
}

fn read_log_capped(logs_dir: &Path, file: &Path, max_bytes: u64) -> Result<RunLog, LogError> {
    read_log_with_limits(logs_dir, file, max_bytes, MAX_SCAN_BYTES)
}

/// `read_log` with its limits as parameters: the tail shown is `max_bytes` long, and the errors
/// are searched in at most `max_scan_bytes` before it.
fn read_log_with_limits(
    logs_dir: &Path,
    file: &Path,
    max_bytes: u64,
    max_scan_bytes: u64,
) -> Result<RunLog, LogError> {
    let text = read_text_capped(logs_dir, file, max_bytes, max_scan_bytes)?;
    Ok(RunLog {
        error_lines: engine::error_lines(&text),
        text,
    })
}

fn read_text_capped(
    logs_dir: &Path,
    file: &Path,
    max_bytes: u64,
    max_scan_bytes: u64,
) -> Result<String, LogError> {
    let lower = |p: &Path| PathBuf::from(p.to_string_lossy().to_lowercase());
    let file = file.canonicalize()?;
    if !lower(&file).starts_with(lower(&logs_dir.canonicalize()?)) {
        return Err(LogError::OutsideLogsFolder);
    }
    let mut input = File::open(&file)?;
    let len = input.metadata()?.len();
    if len <= max_bytes {
        let mut bytes = Vec::with_capacity(len as usize);
        input.read_to_end(&mut bytes)?;
        return Ok(engine::decode_log(&bytes));
    }
    // Start on a UTF-16 unit boundary, then drop the cut line.
    let start = (len - max_bytes) & !1;
    let scan = errors_before(&mut input, start, max_scan_bytes)?;
    let mut early_errors = scan.errors;
    input.seek(SeekFrom::Start(start))?;
    // A character the scan left half read is completed by the first bytes of the tail.
    let mut tail = scan.unfinished_char;
    input.read_to_end(&mut tail)?;
    let text = engine::decode_log(&tail);
    let from_next_line = match text.split_once('\n') {
        Some((rest_of_cut_line, rest)) => {
            // The dropped line began in the part before `start`: it is judged whole.
            let mut cut_line = scan.unfinished_line;
            push_capped(&mut cut_line, rest_of_cut_line);
            let cut_line = cut_line.strip_suffix('\r').unwrap_or(&cut_line);
            if early_errors.len() < MAX_EARLY_ERRORS && engine::is_error_line(cut_line) {
                early_errors.push(cut_line.to_string());
            }
            rest
        }
        None => text.as_str(),
    };
    let title = if early_errors.is_empty() {
        String::new()
    } else {
        format!("{EARLY_ERRORS_TITLE}{}\n", early_errors.join("\n"))
    };
    Ok(format!("{title}{TRUNCATED_NOTICE}{from_next_line}"))
}

/// What the scan of the part before the shown tail leaves for the tail to complete.
struct EarlyScan {
    /// Error lines, at most `MAX_EARLY_ERRORS`, without line end.
    errors: Vec<String>,
    /// The line the scanned part ends in, as far as it was kept (`MAX_LINE_BYTES`).
    unfinished_line: String,
    /// Bytes at the end of the part that begin a character continued in the tail.
    unfinished_char: Vec<u8>,
}

/// Appends `text` to `line` without making it longer than `MAX_LINE_BYTES`, nor splitting a char.
fn push_capped(line: &mut String, text: &str) {
    let mut end = text.len().min(MAX_LINE_BYTES.saturating_sub(line.len()));
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    line.push_str(&text[..end]);
}

/// Scans for errors in the `max_scan_bytes` bytes before `end`, in chunks so a huge log is never held
/// whole. `end` is on a UTF-16 unit boundary. A window that begins inside the log begins inside a
/// line: that line is not judged.
fn errors_before(input: &mut File, end: u64, max_scan_bytes: u64) -> io::Result<EarlyScan> {
    let from = end.saturating_sub(max_scan_bytes) & !1;
    input.seek(SeekFrom::Start(from))?;
    let mut scan = EarlyScan {
        errors: Vec::new(),
        unfinished_line: String::new(),
        unfinished_char: Vec::new(),
    };
    let mut skipping_first_line = from > 0;
    let mut chunk = vec![0; SCAN_CHUNK_BYTES];
    let mut left = end - from;
    while left > 0 && scan.errors.len() < MAX_EARLY_ERRORS {
        let n = left.min(SCAN_CHUNK_BYTES as u64) as usize;
        input.read_exact(&mut chunk[..n])?;
        left -= n as u64;
        // Decode whole characters only: what a chunk ends in the middle of waits for the next.
        let mut bytes = std::mem::take(&mut scan.unfinished_char);
        bytes.extend_from_slice(&chunk[..n]);
        scan.unfinished_char = bytes.split_off(engine::whole_chars_len(&bytes));
        let decoded = engine::decode_log(&bytes);
        let mut text = decoded.as_str();
        if skipping_first_line {
            let Some((_, rest)) = text.split_once('\n') else {
                continue;
            };
            text = rest;
            skipping_first_line = false;
        }
        // Lines as `str::lines` splits them, so they match the line numbers the UI gets.
        let mut pieces = text.split('\n');
        let unfinished_piece = pieces.next_back().unwrap_or_default();
        for piece in pieces {
            push_capped(&mut scan.unfinished_line, piece);
            let line = scan.unfinished_line.strip_suffix('\r');
            let line = line.unwrap_or(&scan.unfinished_line);
            if scan.errors.len() < MAX_EARLY_ERRORS && engine::is_error_line(line) {
                scan.errors.push(line.to_string());
            }
            scan.unfinished_line.clear();
        }
        push_capped(&mut scan.unfinished_line, unfinished_piece);
    }
    if left > 0 {
        // Stopped before `end` (enough errors): nothing of the tail is half read.
        scan.unfinished_char.clear();
    }
    Ok(scan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::error_and_file_lines;
    use crate::shared::domain::{Outcome, sample_run};

    fn run(name: &str, outcome: Outcome) -> Run {
        Run {
            sync_name: name.into(),
            outcome,
            ..sample_run()
        }
    }

    #[test]
    fn the_latest_run_of_each_sync_is_the_first_seen_and_runs_without_a_sync_belong_to_none() {
        let of = |id: Option<u64>, name: &str| Run {
            sync_id: id,
            ..run(name, Outcome::Ok)
        };
        let runs = [
            of(Some(2), "due, nuova"),
            of(None, "vecchia"),
            of(Some(1), "uno, nuova"),
            of(Some(2), "due, vecchia"),
        ];

        let latest: Vec<_> = latest_per_sync(&runs)
            .into_iter()
            .map(|(id, run)| (id, run.sync_name.as_str()))
            .collect();

        assert_eq!(latest, [(2, "due, nuova"), (1, "uno, nuova")]);
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

        assert_eq!(read_log(dir.path(), &file).unwrap().text, "è già\r\n");
        fs::remove_file(&file).unwrap();
        assert!(read_log(dir.path(), &file).is_err());
    }

    #[test]
    fn read_log_says_which_lines_are_errors() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("x.log");
        let (error, fine) = error_and_file_lines();
        fs::write(&file, unilog(&format!("{fine}\r\n{error}\r\n{fine}\r\n"))).unwrap();

        let log = read_log(dir.path(), &file).unwrap();

        assert_eq!(log.error_lines, [1]);
        assert_eq!(
            serde_json::to_value(&log).unwrap(),
            serde_json::json!({ "text": log.text, "errorLines": [1] })
        );
    }

    #[test]
    fn read_log_refuses_files_outside_the_logs_folder() {
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join("logs");
        fs::create_dir(&logs).unwrap();
        let outside = dir.path().join("secret.txt");
        fs::write(&outside, unilog("segreto")).unwrap();
        fs::write(logs.join("ok.log"), unilog("ok")).unwrap();

        assert_eq!(read_log(&logs, &logs.join("ok.log")).unwrap().text, "ok");
        assert!(matches!(
            read_log(&logs, &outside),
            Err(LogError::OutsideLogsFolder)
        ));
        assert!(matches!(
            read_log(&logs, &logs.join("..").join("secret.txt")),
            Err(LogError::OutsideLogsFolder)
        ));
    }

    #[test]
    fn a_log_that_is_not_there_is_an_io_error_not_an_outside_one() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("gone.log");

        let Err(LogError::Io(error)) = read_log(dir.path(), &missing) else {
            panic!("a missing file is an I/O error");
        };
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn read_log_cuts_long_logs_to_the_last_lines() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("long.log");
        let text: String = (1..=100).map(|n| format!("riga {n:03}\r\n")).collect();
        fs::write(&file, unilog(&text)).unwrap();

        // About 10 lines: 2 bytes per character in UTF-16, 1 in UTF-8.
        let cap = if cfg!(windows) { 200 } else { 100 };
        let shown = read_log_capped(dir.path(), &file, cap).unwrap().text;

        assert!(shown.starts_with("… (mostro solo l’ultima parte del registro)\n"));
        assert!(shown.ends_with("riga 100\r\n"));
        assert!(!shown.contains("riga 001"));
        assert!(shown.lines().count() < 20);
        // No half line after the notice.
        assert!(shown.lines().nth(1).unwrap().starts_with("riga "));
    }

    /// A log of an early error followed by many file lines, and the cap that keeps only the end.
    fn early_error_log(error: &str, fine: &str) -> (String, u64) {
        let files: String = (1..=100).map(|n| format!("{fine} {n}\r\n")).collect();
        // About 10 lines: 2 bytes per character in UTF-16, 1 in UTF-8.
        (
            format!("{fine}\r\n{error}\r\n{files}"),
            if cfg!(windows) { 400 } else { 200 },
        )
    }

    #[test]
    fn an_error_in_the_part_that_is_cut_is_listed_above_the_notice() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("long.log");
        let (error, fine) = error_and_file_lines();
        let (text, cap) = early_error_log(error, fine);
        fs::write(&file, unilog(&text)).unwrap();

        let log = read_log_capped(dir.path(), &file, cap).unwrap();

        let top = format!("{EARLY_ERRORS_TITLE}{error}\n{TRUNCATED_NOTICE}");
        assert!(log.text.starts_with(&top), "{}", log.text);
        assert!(log.text.ends_with(&format!("{fine} 100\r\n")));
        assert!(!log.text.contains(&format!("{fine} 1\r\n")));
        // Line 0 is the title; the error is line 1.
        assert_eq!(log.error_lines, [1]);
    }

    #[test]
    fn an_error_line_cut_in_half_is_still_listed_whole() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("long.log");
        let (error, fine) = error_and_file_lines();
        let (text, _) = early_error_log(error, fine);
        fs::write(&file, unilog(&text)).unwrap();
        // The log is cut 5 characters into the error line.
        let unit = if cfg!(windows) { 2 } else { 1 };
        let bom = if cfg!(windows) { 2 } else { 0 };
        let total = bom + unit * text.len() as u64;
        let cut_at = bom + unit * (fine.len() as u64 + 2 + 5);

        let log = read_log_capped(dir.path(), &file, total - cut_at).unwrap();

        assert_eq!(log.text.matches(error).count(), 1, "{}", log.text);
        assert!(
            log.text
                .starts_with(&format!("{EARLY_ERRORS_TITLE}{error}\n"))
        );
        assert_eq!(log.error_lines, [1]);
    }

    #[test]
    fn at_most_200_errors_of_the_cut_part_are_listed() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("long.log");
        let (error, fine) = error_and_file_lines();
        let errors: String = (1..=300).map(|n| format!("{error} {n}\r\n")).collect();
        let files: String = (1..=50).map(|n| format!("{fine} {n}\r\n")).collect();
        fs::write(&file, unilog(&format!("{errors}{files}"))).unwrap();
        let cap = if cfg!(windows) { 200 } else { 100 };

        let log = read_log_capped(dir.path(), &file, cap).unwrap();

        // The title, then the first 200 errors, then the notice and the end of the log.
        assert_eq!(log.error_lines, (1..=200).collect::<Vec<_>>());
        assert!(log.text.lines().nth(200).unwrap().ends_with(" 200"));
        assert_eq!(
            log.text.lines().nth(201).unwrap(),
            TRUNCATED_NOTICE.trim_end()
        );
    }

    #[test]
    fn errors_are_found_in_every_chunk_of_a_cut_part_bigger_than_one_chunk() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("long.log");
        let (error, fine) = error_and_file_lines();
        // Several chunks of file lines, an error in each of the first three (the chunk size is not
        // a multiple of the line length, so lines straddle the chunk ends).
        let lines_per_error = SCAN_CHUNK_BYTES / 20;
        let text: String = (0..4 * lines_per_error)
            .map(|n| match n {
                n if n % lines_per_error == 7 => format!("{error}\r\n"),
                _ => format!("{fine} {n}\r\n"),
            })
            .collect();
        fs::write(&file, unilog(&text)).unwrap();

        let log = read_log_capped(dir.path(), &file, 400).unwrap();

        assert!(log.text.starts_with(EARLY_ERRORS_TITLE));
        assert_eq!(log.text.matches(error).count(), 4);
        assert_eq!(log.error_lines, [1, 2, 3, 4]);
    }

    /// A character of more than one unit in this platform's log encoding: é in UTF-8, a surrogate
    /// pair in UTF-16.
    const WIDE_CHAR: &str = if cfg!(windows) { "📁" } else { "é" };

    #[test]
    fn an_error_line_with_a_wide_character_split_by_a_chunk_end_is_listed_whole() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("long.log");
        let (error, fine) = error_and_file_lines();
        // The first chunk ends inside WIDE_CHAR: one unit (byte, or UTF-16 code unit) of it is in
        // the first chunk, the rest in the second.
        let unit = if cfg!(windows) { 2 } else { 1 };
        let bom = if cfg!(windows) { 2 } else { 0 };
        let chars_before = (SCAN_CHUNK_BYTES - unit - bom) / unit;
        let error_line = format!("{error} {WIDE_CHAR}");
        let padding = "x".repeat(chars_before - error.len() - 1 - 2);
        let files: String = (1..=100).map(|n| format!("{fine} {n}\r\n")).collect();
        fs::write(
            &file,
            unilog(&format!("{padding}\r\n{error_line}\r\n{files}")),
        )
        .unwrap();
        let cap = if cfg!(windows) { 400 } else { 200 };

        let log = read_log_capped(dir.path(), &file, cap).unwrap();

        let top = format!("{EARLY_ERRORS_TITLE}{error_line}\n{TRUNCATED_NOTICE}");
        assert!(log.text.starts_with(&top), "{:?}", log.text.lines().nth(1));
    }

    #[test]
    fn a_line_longer_than_the_limit_is_listed_cut_to_it_however_many_chunks_it_spans() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("long.log");
        let (error, fine) = error_and_file_lines();
        let junk = "y".repeat(3 * SCAN_CHUNK_BYTES);
        let files: String = (1..=100).map(|n| format!("{fine} {n}\r\n")).collect();
        fs::write(&file, unilog(&format!("{error}{junk}\r\n{files}"))).unwrap();
        let cap = if cfg!(windows) { 400 } else { 200 };

        let log = read_log_capped(dir.path(), &file, cap).unwrap();

        let listed = log.text.lines().nth(1).unwrap();
        assert!(listed.starts_with(error));
        assert_eq!(listed.len(), MAX_LINE_BYTES);
    }

    #[test]
    fn only_the_scan_window_before_the_shown_end_is_searched_for_errors() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("long.log");
        let (error, fine) = error_and_file_lines();
        let pad =
            |count: usize| -> String { (0..count).map(|n| format!("{fine} {n}\r\n")).collect() };
        // An old error, a lot of file lines, a recent error, 100 file lines of which the last
        // ~10 are shown.
        let text = format!("{error} old\r\n{}{error} recent\r\n{}", pad(200), pad(100));
        fs::write(&file, unilog(&text)).unwrap();
        let (cap, unit) = if cfg!(windows) { (400, 2) } else { (200, 1) };

        let everything = read_log_with_limits(dir.path(), &file, cap, u64::MAX).unwrap();
        let window = read_log_with_limits(dir.path(), &file, cap, unit * 3000).unwrap();

        assert_eq!(everything.error_lines, [1, 2]);
        assert!(window.text.contains(&format!("{error} recent\n")));
        assert!(!window.text.contains(&format!("{error} old")));
        assert_eq!(window.error_lines, [1]);
    }

    #[test]
    fn a_log_with_no_error_in_the_cut_part_has_no_title() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("long.log");
        let (error, fine) = error_and_file_lines();
        let files: String = (1..=100).map(|n| format!("{fine} {n}\r\n")).collect();
        fs::write(&file, unilog(&format!("{files}{error}\r\n"))).unwrap();
        let cap = if cfg!(windows) { 400 } else { 200 };

        let log = read_log_capped(dir.path(), &file, cap).unwrap();

        assert!(log.text.starts_with(TRUNCATED_NOTICE));
        assert_eq!(log.error_lines.len(), 1, "only the error in the shown end");
    }
}
