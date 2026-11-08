//! The domain words that more than one slice speaks: the mode of a Sync, a saved Sync, one Run
//! and why it failed. Plain data; the slices depend on this module, never on each other for it.

use super::copy_text;
use super::error_messages::{Explanation, explain_run};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncMode {
    Copy,
    Mirror,
}

/// A saved A→B pair (the glossary's `Sync`; not named so to avoid shadowing `std::marker::Sync`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSync {
    pub id: u64,
    pub name: String,
    pub source: String,
    pub destination: String,
    pub mode: SyncMode,
}

/// A and B of a Sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Folder {
    A,
    B,
}

/// What went wrong: a mapped Windows error code, and the folder it is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cause {
    pub code: u32,
    pub folder: Folder,
}

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

/// A Run as the UI gets it (result screen, Storico and its log window): what is stored, plus what
/// to tell the user about it. The texts are made when the Run is sent, never stored.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReport {
    #[serde(flatten)]
    run: Run,
    explanation: Option<Explanation>,
    /// Heading of the result: what was done, or why it was not.
    title: String,
    /// The schema panel's one word for how it ended ("Fatto.").
    caption: &'static str,
    /// What the result says about the files copied before a failed or stopped Run ended.
    result_note: Option<String>,
    /// "40 file copiati (parziale)", for the Run's figures.
    partial_text: Option<String>,
    /// When it started, "02/11/2026 21:30".
    when: String,
    /// How long it took, "3 min".
    duration_text: String,
}

impl From<Run> for RunReport {
    fn from(run: Run) -> Self {
        let explanation = explain_run(&run);
        RunReport {
            title: copy_text::run_title(&run, explanation.as_ref()),
            caption: copy_text::run_caption(run.outcome),
            result_note: copy_text::run_result_note(&run),
            partial_text: run.partial.map(copy_text::partial_text),
            when: copy_text::format_when(&run.started_at),
            duration_text: copy_text::format_duration(run.duration_ms),
            explanation,
            run,
        }
    }
}

/// A finished Ok run of a copy of `D:\Foto` to `E:\Backup`, for the tests: take it with struct
/// update syntax and change what the test is about.
#[cfg(test)]
pub(crate) fn sample_run() -> Run {
    Run {
        sync_id: None,
        sync_name: "Foto".into(),
        source: r"D:\Foto".into(),
        destination: r"E:\Backup".into(),
        mode: SyncMode::Copy,
        new: 12,
        updated: 1,
        deleted: 0,
        duration_ms: 61_000,
        outcome: Outcome::Ok,
        started_at: "2026-11-02T21:30:00+01:00".into(),
        log_path: r"C:\L\logs\x.log".into(),
        partial: None,
        cause: None,
    }
}

/// A saved copy of `D:\Foto` to `\\NAS\Backup`, for the tests (see `sample_run`).
#[cfg(test)]
pub(crate) fn sample_sync() -> SavedSync {
    SavedSync {
        id: 1,
        name: "Foto".into(),
        source: r"D:\Foto".into(),
        destination: r"\\NAS\Backup".into(),
        mode: SyncMode::Copy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(name: &str, outcome: Outcome) -> Run {
        Run {
            sync_name: name.into(),
            outcome,
            ..sample_run()
        }
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
                folder: Folder::B,
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
