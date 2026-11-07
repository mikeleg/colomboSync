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

/// A run's log as text (robocopy's `/UNILOG`, UTF-16LE with BOM, or rsync's UTF-8 output), only
/// if the file is inside `logs_dir`. Long logs are cut to their last part.
pub fn read_log(logs_dir: &Path, file: &Path) -> io::Result<RunLog> {
    read_log_capped(logs_dir, file, MAX_LOG_BYTES)
}

fn read_log_capped(logs_dir: &Path, file: &Path, max_bytes: u64) -> io::Result<RunLog> {
    let text = read_text_capped(logs_dir, file, max_bytes)?;
    Ok(RunLog {
        error_lines: engine::error_lines(&text),
        text,
    })
}

fn read_text_capped(logs_dir: &Path, file: &Path, max_bytes: u64) -> io::Result<String> {
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
    let start = (len - max_bytes) & !1;
    let (mut early_errors, unfinished_line) = errors_before(&mut input, start)?;
    input.seek(SeekFrom::Start(start))?;
    let mut tail = Vec::new();
    input.read_to_end(&mut tail)?;
    let text = engine::decode_log(&tail);
    let from_next_line = match text.split_once('\n') {
        Some((rest_of_cut_line, rest)) => {
            // The dropped line began in the part before `start`: it is judged whole.
            let cut_line = format!("{unfinished_line}{rest_of_cut_line}");
            let cut_line = cut_line.trim_end_matches('\r');
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

/// The error lines (at most `MAX_EARLY_ERRORS`, without line end) in the first `end` bytes of a
/// log, read in chunks so a huge log is never held whole; and the unfinished line the part ends
/// with. `end` is on a UTF-16 unit boundary. A multi-byte character split between two chunks
/// reads as a replacement character, which cannot change whether the line is an error.
fn errors_before(input: &mut File, end: u64) -> io::Result<(Vec<String>, String)> {
    input.seek(SeekFrom::Start(0))?;
    let mut errors = Vec::new();
    let mut unfinished = String::new();
    let mut chunk = vec![0; SCAN_CHUNK_BYTES];
    let mut left = end;
    while left > 0 && errors.len() < MAX_EARLY_ERRORS {
        let n = left.min(SCAN_CHUNK_BYTES as u64) as usize;
        input.read_exact(&mut chunk[..n])?;
        left -= n as u64;
        let text = format!("{unfinished}{}", engine::decode_log(&chunk[..n]));
        let Some((complete, rest)) = text.rsplit_once('\n') else {
            unfinished = text;
            continue;
        };
        for line in complete.split('\n') {
            let line = line.trim_end_matches('\r');
            if errors.len() < MAX_EARLY_ERRORS && engine::is_error_line(line) {
                errors.push(line.to_string());
            }
        }
        unfinished = rest.to_string();
    }
    Ok((errors, unfinished))
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
        assert!(read_log(&logs, &outside).is_err());
        assert!(read_log(&logs, &logs.join("..").join("secret.txt")).is_err());
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
