//! `history.jsonl`: append-only, one `Run` per line. How the Storico shows it is in `history/`.

use crate::shared::domain::Run;
use crate::shared::storage::read_if_present;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

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

/// The highest sync id on any line of the history, 0 if there is none. Unlike `read` it looks
/// only at the id, so a line that is not a whole `Run` (an outcome from a newer version, say)
/// still counts; a line that is not even JSON is skipped. A missing file is an empty history,
/// any other read error is reported.
pub fn highest_sync_id(file: &Path) -> io::Result<u64> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct OnlySyncId {
        sync_id: Option<u64>,
    }
    let Some(bytes) = read_if_present(file)? else {
        return Ok(0);
    };
    Ok(bytes
        .split(|&b| b == b'\n')
        .filter_map(|line| serde_json::from_slice::<OnlySyncId>(line).ok()?.sync_id)
        .max()
        .unwrap_or(0))
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

#[cfg(test)]
mod tests {
    use super::*;
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
    fn the_highest_sync_id_counts_lines_that_read_skips() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("history.jsonl");
        let of_sync_2 = Run {
            sync_id: Some(2),
            ..run("due", Outcome::Ok)
        };
        append(&file, &of_sync_2).unwrap();
        let mut text = fs::read_to_string(&file).unwrap();
        // An outcome from a newer version, a line with no sync id, a line that is not JSON.
        text.push_str("{\"syncId\":7,\"outcome\":\"SomethingNew\"}\n");
        text.push_str("{\"outcome\":\"Ok\"}\nnon è json\n");
        fs::write(&file, text).unwrap();

        assert_eq!(
            read(&file).unwrap(),
            [of_sync_2],
            "read skips what is not a Run"
        );
        assert_eq!(highest_sync_id(&file).unwrap(), 7);
    }

    #[test]
    fn the_highest_sync_id_of_a_missing_history_is_0_and_of_an_unreadable_one_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("history.jsonl");

        assert_eq!(highest_sync_id(&file).unwrap(), 0);
        fs::create_dir(&file).unwrap();
        assert!(highest_sync_id(&file).is_err());
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
}
